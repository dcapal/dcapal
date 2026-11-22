//! Regression tests pinning the v1 market-data REST responses (`/assets/crypto`,
//! `/assets/fiat`, `/price`) after the move of market data from Redis to PostgreSQL.
//!
//! This file is temporary (`tests/v1_regression.rs`): remove it once the Redis to PostgreSQL move has been
//! deployed and verified in production. Removal is tracked in
//! <https://github.com/dcapal/dcapal/issues/877>.

mod common;

use std::net::SocketAddr;

use common::{ProviderUrls, serve};
use dcapal_backend::{
    app::domain::entity::{Asset, Crypto, Fiat, Market, Price},
    config::PriceProvider,
    ports::outbound::repository::market_data::MarketDataRepository,
};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn get(addr: SocketAddr, path: &str) -> (u16, Value) {
    let response = reqwest::get(format!("http://{addr}{path}")).await.unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or(Value::Null))
}

/// Sorts a JSON array of assets by `id`, so lists are compared without depending on order.
fn sorted_by_id(value: Value) -> Value {
    let mut items = value.as_array().cloned().unwrap_or_default();
    items.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Value::Array(items)
}

/// GIVEN a catalog and markets stored in PostgreSQL,
/// WHEN the v1 asset and price endpoints are called,
/// THEN they answer with the same JSON shapes as when Redis held the data (asset lists are
/// compared regardless of order, which the v1 clients do not rely on), and rows of the shared
/// asset table that are not catalog assets are not listed.
#[sqlx::test(migrations = "../../migrations")]
async fn v1_market_data_responses_keep_their_shape(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    let (btc, eth) = (
        Asset::Crypto(Crypto::new_with_id("btc".into())),
        Asset::Crypto(Crypto::new_with_id("eth".into())),
    );
    let (eur, usd) = (
        Asset::Fiat(Fiat::new("eur".into(), "Euro".into())),
        Asset::Fiat(Fiat::new("usd".into(), "US Dollar".into())),
    );
    for asset in [&eth, &btc, &usd, &eur] {
        repo.store_asset(asset).await.unwrap();
    }
    let ts = jiff::Timestamp::from_second(1_700_000_000).unwrap();
    for (base, quote, px) in [
        (&btc, &eur, 40_000.5),
        (&btc, &usd, 44_000.25),
        (&usd, &eur, 0.9),
    ] {
        let market = Market::new(
            format!("{}{}", base.id(), quote.id()),
            base.clone(),
            quote.clone(),
            Some(Price::new(px, ts)),
        );
        repo.store_market(&market, PriceProvider::Kraken)
            .await
            .unwrap();
    }
    // Shared asset rows that are not part of the catalog (client-supplied crypto, and a
    // currency ticker confirmed by Yahoo) must never show up in the v1 lists.
    sqlx::query(
        "INSERT INTO assets_data
             (id, provider, symbol, name, currency, asset_class, metadata_source)
         VALUES (uuidv7(), 1, 'DOGE', 'Dogecoin', 'DOGE', 4, 0),
                (uuidv7(), 2, 'EURUSD=X', 'EUR/USD', 'USD', 3, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let addr = serve(&pool, ProviderUrls::default()).await;

    let (status, crypto) = get(addr, "/assets/crypto").await;
    assert_eq!(status, 200);
    assert_eq!(
        sorted_by_id(crypto),
        json!([
            {"type": "Crypto", "id": "btc", "symbol": "BTC"},
            {"type": "Crypto", "id": "eth", "symbol": "ETH"},
        ])
    );

    let (_, fiat) = get(addr, "/assets/fiat").await;
    assert_eq!(
        sorted_by_id(fiat),
        json!([
            {"type": "Fiat", "id": "eur", "symbol": "Euro"},
            {"type": "Fiat", "id": "usd", "symbol": "US Dollar"},
        ])
    );

    // Direct market, inverse market and same-asset conversion.
    let (status, direct) = get(addr, "/price/btc?quote=eur").await;
    assert_eq!(
        (status, direct),
        (200, json!({"price": 40_000.5, "ts": 1_700_000_000}))
    );
    let (_, inverse) = get(addr, "/price/eur?quote=usd").await;
    assert_eq!(inverse, json!({"price": 1. / 0.9, "ts": 1_700_000_000}));
    let (_, same) = get(addr, "/price/btc?quote=btc").await;
    assert_eq!(same["price"], json!(1.0));

    let (unknown, _) = get(addr, "/price/doge?quote=eur").await;
    assert_eq!(unknown, 400);
    let (no_price, _) = get(addr, "/price/eth?quote=eur").await;
    assert_eq!(no_price, 404);
}
