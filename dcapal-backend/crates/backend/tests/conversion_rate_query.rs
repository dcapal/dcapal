//! PostgreSQL-backed regression coverage for conversion asset lookup.

use dcapal_backend::{
    app::{
        domain::entity::{Asset, Crypto, Fiat},
        services::command::ConversionRateQuery,
    },
    ports::outbound::repository::market_data::MarketDataRepository,
};

/// Verifies that public mixed-case currency identifiers resolve against the
/// lower-case ids stored in the market-data catalog.
#[sqlx::test(migrations = "../../migrations")]
async fn conversion_query_resolves_uppercase_symbols_from_the_catalog(pool: sqlx::PgPool) {
    // GIVEN a catalog containing Kraken-style lower-case asset ids
    let repo = MarketDataRepository::new(pool);
    repo.store_asset(&Asset::Crypto(Crypto::new_with_id("btc".to_string())))
        .await
        .expect("BTC asset should be stored");
    repo.store_asset(&Asset::Fiat(Fiat::new(
        "usd".to_string(),
        "US Dollar".to_string(),
    )))
    .await
    .expect("USD asset should be stored");

    // WHEN a conversion is requested with uppercase public symbols
    let query = ConversionRateQuery::try_new("BTC", "USD", &repo)
        .await
        .expect("uppercase symbols should resolve from the lower-case catalog");

    // THEN the command contains the canonical catalog assets
    assert_eq!(query.base.id(), "btc");
    assert_eq!(query.quote.id(), "usd");
}
