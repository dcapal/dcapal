//! PostgreSQL-backed coverage of the FX startup gate that runs before the
//! server binds its listener.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use dcapal_backend::{
    app::{
        domain::entity::{Market, Price},
        services::{
            fx_rates::{FxPriceFetcher, FxStartupPolicy, ensure_fx_rates},
            market_data::MarketDataService,
        },
    },
    config::PriceProvider,
    error::DcaError,
    ports::outbound::adapter::YAHOO_FX_TICKERS,
    ports::outbound::repository::market_data::MarketDataRepository,
};
use jiff::Timestamp;
use sqlx::PgPool;

/// Fake Yahoo that answers every FX market after a number of failed attempts.
struct FakeYahoo {
    failures_before_success: usize,
    calls: AtomicUsize,
}

impl FakeYahoo {
    /// A fetcher failing `failures_before_success` calls in total before it answers.
    fn new(failures_before_success: usize) -> Self {
        Self {
            failures_before_success,
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl FxPriceFetcher for FakeYahoo {
    async fn fetch(&self, _market: &Market) -> Option<Price> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        (call >= self.failures_before_success).then(|| Price::new(1.25, Timestamp::now()))
    }
}

/// A policy with millisecond pauses so retries run in real time.
fn fast_policy(budget: Duration) -> FxStartupPolicy {
    FxStartupPolicy {
        budget,
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(5),
    }
}

/// Counts the stored FX markets that carry a price.
async fn priced_fx_markets(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_price AS p
         JOIN assets_data AS b ON b.id = p.base_asset_id
         JOIN assets_data AS q ON q.id = p.quote_asset_id
         WHERE b.asset_class = 3 AND q.asset_class = 3 AND p.price IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Counts the stored fiat assets.
async fn fiat_assets(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM assets_data WHERE provider = 2 AND asset_class = 3")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn gate_seeds_the_catalog_and_loads_missing_prices_after_retries(pool: PgPool) {
    // GIVEN an empty database and a provider that fails its first three calls,
    let repo = Arc::new(MarketDataRepository::new(pool.clone()));
    let service = MarketDataService::new(repo.clone());
    let yahoo = FakeYahoo::new(3);

    // WHEN the startup gate runs,
    ensure_fx_rates(
        &repo,
        &service,
        &yahoo,
        PriceProvider::Kraken,
        fast_policy(Duration::from_secs(30)),
    )
    .await
    .expect("the gate should succeed once Yahoo answers");

    // THEN the eight fiat assets and all seven FX markets are stored with a price.
    assert_eq!(fiat_assets(&pool).await, 8);
    assert_eq!(
        priced_fx_markets(&pool).await,
        YAHOO_FX_TICKERS.len() as i64
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn gate_is_a_no_op_when_every_fx_market_already_has_a_stale_price(pool: PgPool) {
    // GIVEN FX prices stored long ago by an earlier run,
    let repo = Arc::new(MarketDataRepository::new(pool.clone()));
    let service = MarketDataService::new(repo.clone());
    ensure_fx_rates(
        &repo,
        &service,
        &FakeYahoo::new(0),
        PriceProvider::Kraken,
        fast_policy(Duration::from_secs(30)),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE market_price SET observed_at = '2020-01-01T00:00:00Z'")
        .execute(&pool)
        .await
        .unwrap();

    // WHEN the gate runs again with a provider that always fails,
    let offline = FakeYahoo::new(usize::MAX);
    let result = ensure_fx_rates(
        &repo,
        &service,
        &offline,
        PriceProvider::Kraken,
        fast_policy(Duration::from_millis(50)),
    )
    .await;

    // THEN it succeeds without asking the provider, and seeding stays idempotent.
    result.expect("stale prices are enough to start");
    assert_eq!(offline.calls.load(Ordering::SeqCst), 0);
    assert_eq!(fiat_assets(&pool).await, 8);
    assert_eq!(
        priced_fx_markets(&pool).await,
        YAHOO_FX_TICKERS.len() as i64
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn gate_fails_startup_when_prices_stay_unavailable(pool: PgPool) {
    // GIVEN an empty database and a provider that never answers,
    let repo = Arc::new(MarketDataRepository::new(pool.clone()));
    let service = MarketDataService::new(repo.clone());

    // WHEN the startup gate runs with a short budget,
    let result = ensure_fx_rates(
        &repo,
        &service,
        &FakeYahoo::new(usize::MAX),
        PriceProvider::Kraken,
        fast_policy(Duration::from_millis(200)),
    )
    .await;

    // THEN startup fails so the process exits, and no price was stored.
    assert!(matches!(result, Err(DcaError::StartupFailure(..))));
    assert_eq!(priced_fx_markets(&pool).await, 0);
}
