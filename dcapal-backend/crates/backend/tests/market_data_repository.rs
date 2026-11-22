use std::collections::HashSet;

use dcapal_backend::{
    app::domain::entity::{Asset, AssetKind, Crypto, Fiat, Market, Price},
    config::PriceProvider,
    ports::outbound::repository::{
        ImportedRepository, MiscRepository, StatsRepository,
        market_data::{MarketDataRepository, MarketPriceUpdate},
        postgres::types::Provider as DbProvider,
    },
};
use jiff::{civil::date, tz::TimeZone};
use sqlx::PgPool;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn crypto(id: &str) -> Asset {
    Asset::Crypto(Crypto::new_with_id(id.to_string()))
}

fn fiat(id: &str, name: &str) -> Asset {
    Asset::Fiat(Fiat::new(id.to_string(), name.to_string()))
}

fn price(value: f64, minute: i8) -> Price {
    Price::new(
        value,
        date(2026, 11, 15)
            .at(10, minute, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp(),
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

fn update<'a>(
    market_id: &'a String,
    price: Price,
    provider: PriceProvider,
) -> MarketPriceUpdate<'a> {
    MarketPriceUpdate {
        market_id,
        price,
        provider,
    }
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

/// Inserts a client-supplied (unconfirmed) `assets_data` row, as v1 synchronization does.
async fn insert_client_asset(
    pool: &PgPool,
    provider: i16,
    symbol: &str,
    class: i16,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO assets_data (id, provider, symbol, name, currency, asset_class)
         VALUES (uuidv7(), $1, $2, 'Client name', 'EUR', $3)",
    )
    .bind(provider)
    .bind(symbol)
    .bind(class)
    .execute(pool)
    .await?;
    Ok(())
}

/// GIVEN an empty catalog, WHEN crypto and fiat assets are stored,
/// THEN reads are per kind (in no guaranteed order), and provider-confirmed rows are
/// never renamed by a later store.
#[sqlx::test(migrations = "../../migrations")]
async fn catalog_reads_are_per_kind(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    assert!(!repo.has_crypto_assets().await?);

    seed_catalog(&repo).await?;
    repo.store_asset(&fiat("eur", "Euro (updated)")).await?;

    assert!(repo.has_crypto_assets().await?);
    let cryptos = repo.load_assets_by_type(AssetKind::Crypto).await?;
    let mut ids = cryptos.iter().map(|a| a.id().as_str()).collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(ids, ["btc", "eth"]);

    let mut fiats = repo.load_assets_by_type(AssetKind::Fiat).await?;
    fiats.sort_by(|a, b| a.id().cmp(b.id()));
    assert_eq!(
        serde_json::to_value(&fiats)?,
        serde_json::json!([
            {"type": "Fiat", "id": "eur", "symbol": "Euro"},
            {"type": "Fiat", "id": "usd", "symbol": "US Dollar"},
        ])
    );
    assert!(repo.find_asset(&"btc".to_string()).await?.is_some());
    assert!(repo.find_asset(&"doge".to_string()).await?.is_none());
    Ok(())
}

/// GIVEN client-supplied rows (Kraken crypto, Yahoo cash and a Yahoo currency ticker
/// that Yahoo itself confirmed) next to the catalog,
/// WHEN the catalog is read,
/// THEN only catalog crypto and the fiat catalog symbols are listed or found, and a
/// client-supplied crypto row is kept by name but marked confirmed when stored.
#[sqlx::test(migrations = "../../migrations")]
async fn only_confirmed_catalog_rows_are_visible(pool: PgPool) -> TestResult {
    use dcapal_backend::ports::outbound::repository::postgres::types::{AssetClass, Provider};
    let (kraken, yf) = (i16::from(Provider::Kraken), i16::from(Provider::YF));
    let (crypto_class, cash) = (i16::from(AssetClass::Crypto), i16::from(AssetClass::Cash));
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    insert_client_asset(&pool, kraken, "DOGE", crypto_class).await?;
    insert_client_asset(&pool, yf, "CHF", cash).await?;
    sqlx::query(
        "INSERT INTO assets_data
             (id, provider, symbol, name, currency, asset_class, metadata_source)
         VALUES (uuidv7(), $1, 'EURUSD=X', 'EUR/USD', 'USD', $2, 1)",
    )
    .bind(yf)
    .bind(cash)
    .execute(&pool)
    .await?;

    assert!(repo.find_asset(&"doge".to_string()).await?.is_none());
    assert!(repo.find_asset(&"chf".to_string()).await?.is_none());
    assert!(repo.find_asset(&"eurusd=x".to_string()).await?.is_none());
    let cryptos = repo.load_assets_by_type(AssetKind::Crypto).await?;
    assert_eq!(cryptos.len(), 2);
    let fiats = repo.load_assets_by_type(AssetKind::Fiat).await?;
    assert_eq!(fiats.len(), 2);

    repo.store_asset(&crypto("doge")).await?;
    let (name, source): (String, i16) =
        sqlx::query_as("SELECT name, metadata_source FROM assets_data WHERE symbol = 'DOGE'")
            .fetch_one(&pool)
            .await?;
    assert_eq!((name.as_str(), source), ("Client name", 1));
    assert!(repo.find_asset(&"doge".to_string()).await?.is_some());

    repo.store_asset(&fiat("chf", "Swiss franc")).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM assets_data WHERE symbol = 'CHF'")
        .fetch_one(&pool)
        .await?;
    assert_eq!(name, "Swiss franc");
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
        .upsert_prices(&[
            update(&btceur, price(2.0, 1), PriceProvider::Kraken),
            update(&btceur, price(3.0, 2), PriceProvider::Yahoo),
            update(&etheur, price(f64::NAN, 2), PriceProvider::Kraken),
            update(&gone, price(9.0, 2), PriceProvider::Kraken),
        ])
        .await?;

    assert_eq!(stored, 1);
    let btceur_px = repo.find_market(&btceur).await?.unwrap();
    assert_eq!(btceur_px.price().unwrap().price, 3.0);
    assert!(repo.find_market(&etheur).await?.unwrap().price().is_none());
    let (provider, error): (i16, Option<String>) =
        sqlx::query_as("SELECT provider, last_error FROM market_price WHERE price = 3")
            .fetch_one(&pool)
            .await?;
    assert_eq!((provider, error), (i16::from(DbProvider::YF), None));
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
        repo.upsert_prices(&[update(&id, price(value, 0), PriceProvider::Kraken)])
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
        .map(|(i, id)| update(id, price(i as f64 + 1.0, 0), PriceProvider::Kraken))
        .collect::<Vec<_>>();
    assert_eq!(repo.upsert_prices(&updates).await?, 600);
    assert!(
        repo.load_markets()
            .await?
            .iter()
            .all(|m| m.price().is_some())
    );
    Ok(())
}

/// GIVEN stored crypto markets and a Yahoo FX market, WHEN reconciling against a Kraken
/// snapshot, THEN crypto markets outside it are deleted with their prices, the FX market is
/// kept, and an empty snapshot is rejected without deleting anything.
#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_deletes_only_crypto_markets_outside_the_snapshot(
    pool: PgPool,
) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    let (btc, eth, eur, usd) = (
        crypto("btc"),
        crypto("eth"),
        fiat("eur", "Euro"),
        fiat("usd", "US Dollar"),
    );
    seed_market(&repo, &btc, &eur, Some(price(1.0, 0))).await?;
    seed_market(&repo, &eth, &eur, Some(price(2.0, 0))).await?;
    repo.store_market(
        &Market::new("eurusd".into(), eur.clone(), usd, Some(price(1.1, 0))),
        PriceProvider::Yahoo,
    )
    .await?;

    assert!(repo.delete_markets_not_in(&HashSet::new()).await.is_err());
    assert_eq!(repo.load_markets().await?.len(), 3);

    let keep = HashSet::from(["btceur".to_string()]);
    assert_eq!(repo.delete_markets_not_in(&keep).await?, ["etheur"]);
    assert_eq!(repo.load_markets().await?.len(), 2);
    let prices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_price")
        .fetch_one(&pool)
        .await?;
    assert_eq!(prices, 2);
    assert!(repo.find_market(&"eurusd".to_string()).await?.is_some());
    Ok(())
}

/// GIVEN a stored catalog, WHEN several assets are looked up at once with repeated,
/// mixed-case and unknown ids, THEN the result is aligned with the input, and an empty
/// input returns an empty result.
#[sqlx::test(migrations = "../../migrations")]
async fn find_assets_is_aligned_with_the_requested_ids(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;

    let found = repo.find_assets(&["EUR", "nope", "btc", "eur"]).await?;

    let ids = found
        .iter()
        .map(|a| a.as_ref().map(|a| a.id().clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            Some("eur".to_string()),
            None,
            Some("btc".to_string()),
            Some("eur".to_string())
        ]
    );
    assert!(repo.find_assets(&[] as &[&str]).await?.is_empty());
    Ok(())
}

/// GIVEN a client-supplied crypto and fiat row, WHEN a batch of assets is stored together
/// with a duplicate, THEN the fiat row takes the catalog name, the crypto row keeps its name
/// and both become confirmed, new assets are inserted, and an empty batch is a no-op.
#[sqlx::test(migrations = "../../migrations")]
async fn store_assets_batches_inserts_and_confirmations(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    insert_client_asset(&pool, 1, "DOGE", 1).await?;
    insert_client_asset(&pool, 2, "EUR", 3).await?;

    repo.store_assets(&[]).await?;
    repo.store_assets(&[
        crypto("doge"),
        fiat("eur", "Euro"),
        crypto("btc"),
        crypto("btc"),
    ])
    .await?;

    let rows: Vec<(String, String, i16)> =
        sqlx::query_as("SELECT symbol, name, metadata_source FROM assets_data ORDER BY symbol")
            .fetch_all(&pool)
            .await?;
    assert_eq!(
        rows,
        [
            ("BTC".to_string(), "BTC".to_string(), 1),
            ("DOGE".to_string(), "Client name".to_string(), 1),
            ("EUR".to_string(), "Euro".to_string(), 1),
        ]
    );
    Ok(())
}

/// GIVEN a catalog, WHEN several markets are stored in one call in any order, with a repeated
/// market, THEN every market is inserted once, priced markets keep the last price, and an empty
/// batch is a no-op.
#[sqlx::test(migrations = "../../migrations")]
async fn store_markets_inserts_a_batch_regardless_of_order(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;
    let (btc, eth, eur) = (crypto("btc"), crypto("eth"), fiat("eur", "Euro"));
    let btceur = Market::new(
        "btceur".into(),
        btc.clone(),
        eur.clone(),
        Some(price(1.0, 0)),
    );
    let btceur_newer = Market::new("btceur".into(), btc, eur.clone(), Some(price(3.0, 1)));
    let etheur = Market::new("etheur".into(), eth, eur, None);

    repo.store_markets(&[]).await?;
    repo.store_markets(&[
        (&etheur, PriceProvider::Kraken),
        (&btceur, PriceProvider::Kraken),
        (&btceur_newer, PriceProvider::Kraken),
    ])
    .await?;

    assert_eq!(repo.load_markets().await?.len(), 2);
    let stored = repo.find_market(&"btceur".to_string()).await?.unwrap();
    assert_eq!(stored.price().unwrap().price, 3.0);
    assert!(
        repo.find_market(&"etheur".to_string())
            .await?
            .unwrap()
            .price()
            .is_none()
    );
    Ok(())
}

/// GIVEN a catalog missing one asset, WHEN a batch holding a valid market and a market of the
/// missing asset is stored, THEN the call fails naming the market and nothing is written, not
/// even the valid market.
#[sqlx::test(migrations = "../../migrations")]
async fn store_markets_rolls_back_the_whole_batch_on_a_missing_asset(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool);
    seed_catalog(&repo).await?;
    let (btc, eur) = (crypto("btc"), fiat("eur", "Euro"));
    let valid = Market::new(
        "btceur".into(),
        btc.clone(),
        eur.clone(),
        Some(price(1.0, 0)),
    );
    let orphan = Market::new("doge".to_string() + "eur", crypto("doge"), eur, None);

    let result = repo
        .store_markets(&[
            (&valid, PriceProvider::Kraken),
            (&orphan, PriceProvider::Kraken),
        ])
        .await;

    assert!(
        matches!(result, Err(dcapal_backend::error::DcaError::RepositoryStoreFailure(id)) if id == "dogeeur")
    );
    assert!(repo.load_markets().await?.is_empty());
    Ok(())
}

/// GIVEN a fresh database, WHEN the catalog is stored with assets and markets together and one
/// market references an unknown asset, THEN neither the assets nor the markets are written.
#[sqlx::test(migrations = "../../migrations")]
async fn store_catalog_is_all_or_nothing(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    let (usd, eur) = (fiat("usd", "US Dollar"), fiat("eur", "Euro"));
    let eurusd = Market::new("eurusd".into(), eur.clone(), usd.clone(), None);
    let broken = Market::new("eurchf".into(), eur.clone(), fiat("chf", "Franc"), None);

    let failed = repo
        .store_catalog(
            &[usd.clone(), eur.clone()],
            &[
                (&eurusd, PriceProvider::Yahoo),
                (&broken, PriceProvider::Yahoo),
            ],
        )
        .await;
    assert!(failed.is_err());
    let assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets_data")
        .fetch_one(&pool)
        .await?;
    assert_eq!(assets, 0);

    repo.store_catalog(&[usd, eur], &[(&eurusd, PriceProvider::Yahoo)])
        .await?;
    assert_eq!(repo.load_markets().await?.len(), 1);
    Ok(())
}

/// GIVEN a fresh database, WHEN runtime state is written, THEN markers, counters, visitors and
/// imported portfolios behave as they did on Redis, including the expiry of transfers.
#[sqlx::test(migrations = "../../migrations")]
async fn runtime_state_is_persisted(pool: PgPool) -> TestResult {
    let misc = MiscRepository::new(pool.clone());
    assert!(misc.get_cw_last_fetched().await?.is_none());
    let ts = date(2026, 11, 15)
        .at(1, 2, 3, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp();
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

/// Reads `(price, last_error, last_error_at is set)` of the `btceur` market.
async fn price_and_error(pool: &PgPool) -> sqlx::Result<(Option<String>, Option<String>, bool)> {
    sqlx::query_as("SELECT price::text, last_error, last_error_at IS NOT NULL FROM market_price")
        .fetch_one(pool)
        .await
}

/// GIVEN a market with a stored price,
/// WHEN a refresh failure is recorded, and later a fresh price is stored,
/// THEN the failure sits next to the unchanged last known price, and the fresh price
/// clears the failure.
#[sqlx::test(migrations = "../../migrations")]
async fn recorded_failure_keeps_the_last_known_price_until_a_new_price_clears_it(
    pool: PgPool,
) -> TestResult {
    use dcapal_backend::{
        app::domain::market_quote::PriceFailure,
        ports::outbound::repository::market_data::MarketPriceFailure,
    };
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    seed_market(
        &repo,
        &crypto("btc"),
        &fiat("eur", "Euro"),
        Some(price(40_000.5, 1)),
    )
    .await?;
    let market_id = "btceur".to_string();
    let failure = PriceFailure::provider_error("Kraken answered HTTP 503");

    let recorded = repo
        .record_price_failures(&[
            MarketPriceFailure {
                market_id: &market_id,
                failure: &failure,
            },
            MarketPriceFailure {
                market_id: &"unknown".to_string(),
                failure: &failure,
            },
        ])
        .await?;

    assert_eq!(recorded, 1);
    let (stored_price, error, has_error_time) = price_and_error(&pool).await?;
    assert_eq!(stored_price.as_deref(), Some("40000.5"));
    assert_eq!(
        error.as_deref(),
        Some("providerError: Kraken answered HTTP 503")
    );
    assert!(has_error_time);

    let fresh = [update(
        &market_id,
        price(41_000.0, 5),
        PriceProvider::Kraken,
    )];
    repo.upsert_prices(&fresh).await?;
    let (stored_price, error, has_error_time) = price_and_error(&pool).await?;
    assert_eq!(stored_price.as_deref(), Some("41000"));
    assert_eq!((error, has_error_time), (None, false));
    Ok(())
}

/// GIVEN a market with a stored price,
/// WHEN zero, negative and non-finite prices are submitted,
/// THEN none is stored and the last known price is kept.
#[sqlx::test(migrations = "../../migrations")]
async fn non_positive_prices_are_never_stored(pool: PgPool) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    seed_catalog(&repo).await?;
    seed_market(
        &repo,
        &crypto("btc"),
        &fiat("eur", "Euro"),
        Some(price(10.0, 1)),
    )
    .await?;
    let id = "btceur".to_string();

    for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
        let stored = repo
            .upsert_prices(&[update(&id, price(bad, 9), PriceProvider::Kraken)])
            .await?;
        assert_eq!(stored, 0, "{bad}");
    }

    assert_eq!(price_and_error(&pool).await?.0.as_deref(), Some("10"));
    Ok(())
}

/// GIVEN a Kraken catalog with BTC (priced in USD), WBTC and ETH, plus a client-supplied
/// DOGE row,
/// WHEN the catalog is searched by symbol text, by a wildcard character, and with a limit,
/// THEN matches are case-insensitive substrings with the exact symbol first, wildcards are
/// literal, unconfirmed rows never match, and the USD price availability is reported.
#[sqlx::test(migrations = "../../migrations")]
async fn crypto_catalog_search_matches_literally_and_reports_price_availability(
    pool: PgPool,
) -> TestResult {
    let repo = MarketDataRepository::new(pool.clone());
    for asset in [
        crypto("btc"),
        crypto("wbtc"),
        crypto("eth"),
        fiat("usd", "US Dollar"),
    ] {
        repo.store_asset(&asset).await?;
    }
    seed_market(
        &repo,
        &crypto("btc"),
        &fiat("usd", "US Dollar"),
        Some(price(40_000.0, 1)),
    )
    .await?;
    seed_market(&repo, &crypto("eth"), &fiat("usd", "US Dollar"), None).await?;
    insert_client_asset(&pool, 1, "DOGE", 4).await?;

    let hits = repo.search_crypto_catalog("bTc", 10).await?;
    let found = hits
        .iter()
        .map(|h| (h.symbol.as_str(), h.priced))
        .collect::<Vec<_>>();
    assert_eq!(found, [("BTC", true), ("WBTC", false)]);

    assert_eq!(repo.search_crypto_catalog("b", 1).await?.len(), 1);
    assert!(repo.search_crypto_catalog("%", 10).await?.is_empty());
    assert!(repo.search_crypto_catalog("_", 10).await?.is_empty());
    assert!(repo.search_crypto_catalog("doge", 10).await?.is_empty());
    let eth = repo.search_crypto_catalog("eth", 10).await?;
    assert_eq!((eth[0].symbol.as_str(), eth[0].priced), ("ETH", false));
    Ok(())
}
