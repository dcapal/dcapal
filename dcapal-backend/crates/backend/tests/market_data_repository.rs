use std::collections::HashSet;

use chrono::{TimeZone, Utc};
use dcapal_backend::{
    app::domain::entity::{Asset, AssetKind, Crypto, Fiat, Market, Price},
    config::PriceProvider,
    ports::outbound::repository::{
        ImportedRepository, MiscRepository, StatsRepository,
        market_data::{MarketDataRepository, MarketPriceUpdate},
    },
};
use sqlx::PgPool;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn crypto(id: &str) -> Asset {
    Asset::Crypto(Crypto::new_with_id(id.to_string()))
}

fn fiat(id: &str, name: &str) -> Asset {
    Asset::Fiat(Fiat::new(id.to_string(), name.to_string()))
}

fn price(value: f64, minute: u32) -> Price {
    Price::new(
        value,
        Utc.with_ymd_and_hms(2026, 11, 15, 10, minute, 0).unwrap(),
    )
}

/// Stores btc, eth, eur and usd in the catalog.
async fn seed_catalog(repo: &MarketDataRepository) -> TestResult {
    for asset in [
        crypto("eth"),
        crypto("btc"),
        fiat("usd", "US Dollar"),
        fiat("eur", "Euro"),
    ] {
        repo.store_asset(&asset).await?;
    }
    Ok(())
}

async fn seed_market(
    repo: &MarketDataRepository,
    base: &Asset,
    quote: &Asset,
    price: Option<Price>,
) -> TestResult {
    let id = format!("{}{}", base.id(), quote.id());
    repo.store_market(
        &Market::new(id, base.clone(), quote.clone(), price),
        PriceProvider::Kraken,
    )
    .await?;
    Ok(())
}

/// GIVEN an empty catalog, WHEN crypto and fiat assets are stored,
/// THEN reads are per kind (in no guaranteed order), and a re-stored asset replaces the old one.
#[sqlx::test(migrations = "../../migrations")]
async fn catalog_reads_are_per_kind(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    assert!(!repo.has_assets().await?);

    seed_catalog(&repo).await?;
    repo.store_asset(&fiat("eur", "Euro (updated)")).await?;

    assert!(repo.has_assets().await?);
    let cryptos = repo.load_assets_by_type(AssetKind::Crypto).await?;
    let mut ids = cryptos.iter().map(|a| a.id().as_str()).collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(ids, ["btc", "eth"]);

    let mut fiats = repo.load_assets_by_type(AssetKind::Fiat).await?;
    fiats.sort_by(|a, b| a.id().cmp(b.id()));
    assert_eq!(
        serde_json::to_value(&fiats)?,
        serde_json::json!([
            {"type": "Fiat", "id": "eur", "symbol": "Euro (updated)"},
            {"type": "Fiat", "id": "usd", "symbol": "US Dollar"},
        ])
    );
    assert!(repo.find_asset(&"btc".to_string()).await?.is_some());
    assert!(repo.find_asset(&"doge".to_string()).await?.is_none());
    Ok(())
}

/// GIVEN stored markets with and without a price, WHEN they are read,
/// THEN each carries its assets and its price only when one was stored.
#[sqlx::test(migrations = "../../migrations")]
async fn markets_are_read_with_assets_and_optional_price(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;
    let (btc, eth, eur) = (crypto("btc"), crypto("eth"), fiat("eur", "Euro"));
    seed_market(&repo, &btc, &eur, Some(price(50_000.5, 1))).await?;
    seed_market(&repo, &eth, &eur, None).await?;

    let btceur = repo.find_market(&"btceur".to_string()).await?.unwrap();
    assert_eq!(btceur.pair, "BTC/EUR");
    let stored = btceur.price().unwrap();
    assert_eq!((stored.price, stored.ts), (50_000.5, price(0.0, 1).ts));
    assert!(
        repo.find_market(&"etheur".to_string())
            .await?
            .unwrap()
            .price()
            .is_none()
    );
    assert!(repo.find_market(&"nope".to_string()).await?.is_none());

    let (a, b, c) = (
        "etheur".to_string(),
        "nope".to_string(),
        "btceur".to_string(),
    );
    let found = repo.find_markets(&[&a, &b, &c]).await?;
    assert_eq!(
        found.iter().map(Option::is_some).collect::<Vec<_>>(),
        [true, false, true]
    );
    assert_eq!(repo.load_markets().await?.len(), 2);
    Ok(())
}

/// GIVEN a market with a price, WHEN a batch of updates is upserted (duplicates, an unknown
/// market and a NaN included), THEN known markets take the last finite price and the rest is skipped.
#[sqlx::test(migrations = "../../migrations")]
async fn batch_upsert_replaces_prices_and_skips_unknown_markets(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    let (btc, eth, eur) = (crypto("btc"), crypto("eth"), fiat("eur", "Euro"));
    seed_market(&repo, &btc, &eur, Some(price(1.0, 0))).await?;
    seed_market(&repo, &eth, &eur, None).await?;

    let (btceur, etheur, gone) = (
        "btceur".to_string(),
        "etheur".to_string(),
        "gone".to_string(),
    );
    let stored = repo
        .upsert_prices(
            &[
                MarketPriceUpdate {
                    market_id: &btceur,
                    price: price(2.0, 1),
                },
                MarketPriceUpdate {
                    market_id: &btceur,
                    price: price(3.0, 2),
                },
                MarketPriceUpdate {
                    market_id: &etheur,
                    price: price(f64::NAN, 2),
                },
                MarketPriceUpdate {
                    market_id: &gone,
                    price: price(9.0, 2),
                },
            ],
            PriceProvider::Yahoo,
        )
        .await?;

    assert_eq!(stored, 1);
    let btceur_px = repo.find_market(&btceur).await?.unwrap();
    assert_eq!(btceur_px.price().unwrap().price, 3.0);
    assert!(repo.find_market(&etheur).await?.unwrap().price().is_none());
    let (provider, error): (String, Option<String>) =
        sqlx::query_as("SELECT provider, last_error FROM market_price WHERE market_id = 'btceur'")
            .fetch_one(&pool)
            .await?;
    assert_eq!((provider.as_str(), error), ("Yahoo", None));
    Ok(())
}

/// GIVEN prices from tiny to huge, WHEN stored and read back, THEN the f64 is bit-identical.
#[sqlx::test(migrations = "../../migrations")]
async fn prices_round_trip_exactly(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;
    let (btc, eur) = (crypto("btc"), fiat("eur", "Euro"));
    seed_market(&repo, &btc, &eur, None).await?;
    let id = "btceur".to_string();

    for value in [
        1e-9,
        0.1 + 0.2,
        123_456.789_012_345_67,
        1.7e300,
        f64::MIN_POSITIVE,
    ] {
        repo.upsert_prices(
            &[MarketPriceUpdate {
                market_id: &id,
                price: price(value, 0),
            }],
            PriceProvider::Kraken,
        )
        .await?;
        let read = repo.find_market(&id).await?.unwrap().price().unwrap().price;
        assert_eq!(read.to_bits(), value.to_bits(), "{value}");
    }
    Ok(())
}

/// GIVEN 600 markets, WHEN all prices are written in one call, THEN every market has its price.
#[sqlx::test(migrations = "../../migrations")]
async fn large_worker_batch_is_stored(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;
    let eur = fiat("eur", "Euro");
    let ids = (0..600).map(|i| format!("a{i}eur")).collect::<Vec<_>>();
    for i in 0..600 {
        let base = crypto(&format!("a{i}"));
        repo.store_asset(&base).await?;
        seed_market(&repo, &base, &eur, None).await?;
    }

    let updates = ids
        .iter()
        .enumerate()
        .map(|(i, id)| MarketPriceUpdate {
            market_id: id,
            price: price(i as f64 + 1.0, 0),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        repo.upsert_prices(&updates, PriceProvider::Kraken).await?,
        600
    );
    assert!(
        repo.load_markets()
            .await?
            .iter()
            .all(|m| m.price().is_some())
    );
    Ok(())
}

/// GIVEN stored markets, WHEN reconciling against a snapshot, THEN markets outside it are deleted
/// with their prices, and an empty snapshot is rejected without deleting anything.
#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_deletes_only_markets_outside_the_snapshot(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    let (btc, eth, eur) = (crypto("btc"), crypto("eth"), fiat("eur", "Euro"));
    seed_market(&repo, &btc, &eur, Some(price(1.0, 0))).await?;
    seed_market(&repo, &eth, &eur, Some(price(2.0, 0))).await?;

    assert!(repo.delete_markets_not_in(&HashSet::new()).await.is_err());
    assert_eq!(repo.load_markets().await?.len(), 2);

    let keep = HashSet::from(["btceur".to_string()]);
    assert_eq!(repo.delete_markets_not_in(&keep).await?, ["etheur"]);
    assert_eq!(repo.load_markets().await?.len(), 1);
    let prices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_price")
        .fetch_one(&pool)
        .await?;
    assert_eq!(prices, 1);
    Ok(())
}

/// GIVEN a fresh database, WHEN runtime state is written, THEN markers, counters, visitors and
/// imported portfolios behave as they did on Redis, including the expiry of transfers.
#[sqlx::test(migrations = "../../migrations")]
async fn runtime_state_is_persisted(pool: PgPool) -> TestResult {
    let misc = MiscRepository::new(pool.clone());
    assert!(misc.get_cw_last_fetched().await?.is_none());
    let ts = Utc.with_ymd_and_hms(2026, 11, 15, 1, 2, 3).unwrap();
    misc.set_cw_last_fetched(ts).await?;
    assert_eq!(misc.get_cw_last_fetched().await?, Some(ts));

    let stats = StatsRepository::new(pool.clone());
    assert_eq!(stats.bump_visit("1.2.3.4").await?, 1);
    assert_eq!(stats.bump_visit("1.2.3.4").await?, 2);
    assert_eq!(stats.fetch_all_visitors().await?["1.2.3.4"], 2);
    assert_eq!(stats.get_imported_portfolio_count().await?, 0);
    assert_eq!(stats.increase_imported_portfolio_count().await?, 1);

    let imported = ImportedRepository::new(pool.clone());
    let payload = serde_json::json!({"name": "pf"});
    let stored = imported.store_portfolio(&payload).await?;
    let id = stored.id.simple().to_string();
    assert_eq!(imported.find_portfolio(&id).await?, Some(payload));
    assert_eq!(imported.find_portfolio("not-a-uuid").await?, None);

    sqlx::query("UPDATE imported_portfolio SET expires_at = NOW() - INTERVAL '1 second'")
        .execute(&pool)
        .await?;
    assert_eq!(imported.find_portfolio(&id).await?, None);
    Ok(())
}
