//! Seeding of the fiat catalog and FX markets, and the startup gate that keeps
//! the server offline until every FX market has a price.
//!
//! Every cross-currency conversion depends on the seven Yahoo-priced FX
//! markets, so a server that answers requests without them would fail every
//! conversion. [`ensure_fx_rates`] runs before the HTTP listener is bound. It
//! only needs Yahoo on the first boot: once all FX markets have a stored price,
//! even a stale one, startup does not wait for any provider and the price
//! worker refreshes the rates afterwards.

use std::{collections::HashSet, future::Future, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::future::join_all;
use parking_lot::Mutex;
use tokio::time::{Instant, sleep, timeout};
use tracing::{info, warn};

use crate::{
    app::{
        domain::{
            entity::{Asset, Fiat, Market, MarketId, Price},
            fiat::{FIAT_CATALOG, fiat_name},
            market_data_utils::{fetch_market_price, price_provider_for},
        },
        services::market_data::MarketDataService,
    },
    config::PriceProvider,
    error::{DcaError, Result},
    ports::outbound::{
        adapter::{PriceProviders, YAHOO_FX_TICKERS},
        repository::market_data::MarketDataRepository,
    },
};

/// Source of the first price of an FX market during the startup gate.
#[async_trait]
pub trait FxPriceFetcher: Send + Sync {
    /// Fetches the current price of an FX market, or `None` when it is not available now.
    async fn fetch(&self, market: &Market) -> Option<Price>;
}

/// Fetches FX prices from Yahoo with the hardcoded FX tickers.
pub struct YahooFxFetcher {
    providers: Arc<PriceProviders>,
}

impl YahooFxFetcher {
    /// Creates a fetcher that uses the Yahoo provider of `providers`.
    pub fn new(providers: Arc<PriceProviders>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl FxPriceFetcher for YahooFxFetcher {
    async fn fetch(&self, market: &Market) -> Option<Price> {
        fetch_market_price(market, &self.providers, PriceProvider::Yahoo).await
    }
}

/// How long and how often the startup gate retries missing FX prices.
#[derive(Debug, Clone, Copy)]
pub struct FxStartupPolicy {
    /// Total time the gate may spend before startup fails.
    pub budget: Duration,
    /// Pause after the first failed attempt of a ticker; it doubles after each failure.
    pub initial_backoff: Duration,
    /// Upper bound of the pause between two attempts of a ticker.
    pub max_backoff: Duration,
}

impl Default for FxStartupPolicy {
    /// Two minutes in total, backing off from one second up to thirty seconds.
    fn default() -> Self {
        Self {
            budget: Duration::from_secs(120),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
        }
    }
}

impl FxStartupPolicy {
    /// Pause before the attempt that follows the `failures`-th consecutive failure.
    fn backoff(&self, failures: u32) -> Duration {
        self.initial_backoff
            .saturating_mul(2u32.saturating_pow(failures.saturating_sub(1)))
            .min(self.max_backoff)
    }
}

/// Idempotently stores the fiat catalog and the Yahoo-priced FX markets.
///
/// Fiat assets are written to `assets_data` as provider-confirmed Yahoo cash
/// with English names. This replaces the metadata of a client-supplied row for
/// the same symbol; the catalog is the authority for these eight symbols. FX
/// markets are inserted without a price and existing prices are kept.
pub async fn seed_fx_catalog(
    repo: &MarketDataRepository,
    service: &MarketDataService,
    crypto_provider: PriceProvider,
) -> Result<()> {
    let fiats = FIAT_CATALOG
        .iter()
        .map(|(symbol, name)| Asset::Fiat(Fiat::new(symbol.to_lowercase(), name.to_string())))
        .collect::<Vec<_>>();
    let markets = fx_markets();
    let priced_by = markets
        .iter()
        .map(|market| (market, price_provider_for(market, crypto_provider)))
        .collect::<Vec<_>>();

    // Assets and markets land together, so a failure never leaves FX markets
    // without their fiat assets.
    let stored = repo.store_catalog(&fiats, &priced_by).await;
    service.invalidate_asset_cache();
    service.invalidate_market_cache();
    stored
}

/// The seven FX markets (fiat against USD) without prices.
fn fx_markets() -> Vec<Market> {
    YAHOO_FX_TICKERS
        .iter()
        .map(|(market_id, _)| {
            let (base_id, quote_id) = market_id.split_at(3);
            let fiat = |id: &str| {
                Asset::Fiat(Fiat::new(
                    id.to_string(),
                    fiat_name(id).unwrap_or(id).to_string(),
                ))
            };
            Market::new(market_id.to_string(), fiat(base_id), fiat(quote_id), None)
        })
        .collect()
}

/// Makes sure every FX market has a stored price before the server goes online.
///
/// Seeds the fiat catalog and FX markets, then returns immediately when all of
/// them already have a price, however old. Otherwise it fetches the missing
/// prices with `fetcher`, retrying every ticker with exponential backoff, and
/// fails with [`DcaError::StartupFailure`] once the policy's budget is spent so
/// the process exits and its orchestrator restarts it.
pub async fn ensure_fx_rates(
    repo: &MarketDataRepository,
    service: &MarketDataService,
    fetcher: &dyn FxPriceFetcher,
    crypto_provider: PriceProvider,
    policy: FxStartupPolicy,
) -> Result<()> {
    seed_fx_catalog(repo, service, crypto_provider).await?;

    let missing = unpriced_fx_markets(repo).await?;
    if missing.is_empty() {
        info!("All FX rates are already stored");
        return Ok(());
    }

    info!(
        "Waiting for {} FX rates before starting the server",
        missing.len()
    );
    let outcome = load_prices(missing, fetcher, policy, |market| async move {
        let provider = price_provider_for(&market, crypto_provider);
        repo.store_market(&market, provider).await
    })
    .await?;
    service.invalidate_market_cache();

    match outcome {
        FxLoadOutcome::Loaded => {
            info!("All FX rates loaded");
            Ok(())
        }
        FxLoadOutcome::TimedOut { missing } => Err(DcaError::StartupFailure(
            format!(
                "FX rates not available after {:?}: {}",
                policy.budget,
                missing.join(", ")
            ),
            anyhow::anyhow!("Yahoo did not return the missing FX prices in time"),
        )),
    }
}

/// The FX markets that have no stored price yet.
async fn unpriced_fx_markets(repo: &MarketDataRepository) -> Result<Vec<Market>> {
    let ids = YAHOO_FX_TICKERS
        .iter()
        .map(|(market_id, _)| market_id.to_string())
        .collect::<Vec<_>>();
    let stored = repo.find_markets(&ids).await?;

    Ok(fx_markets()
        .into_iter()
        .zip(stored)
        .filter(|(_, stored)| stored.as_ref().is_none_or(|m| m.price().is_none()))
        .map(|(market, _)| market)
        .collect())
}

/// How [`load_prices`] ended.
#[derive(Debug, PartialEq, Eq)]
enum FxLoadOutcome {
    /// Every market was priced and stored.
    Loaded,
    /// The budget ran out; these markets were still unpriced.
    TimedOut { missing: Vec<MarketId> },
}

/// Fetches the price of every market concurrently, retrying each one with
/// exponential backoff, and hands each priced market to `store`.
///
/// Stops when all markets are stored, when the policy's budget is spent, or
/// when `store` fails.
async fn load_prices<S, Fut>(
    markets: Vec<Market>,
    fetcher: &dyn FxPriceFetcher,
    policy: FxStartupPolicy,
    store: S,
) -> Result<FxLoadOutcome>
where
    S: Fn(Market) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let pending: Mutex<HashSet<MarketId>> =
        Mutex::new(markets.iter().map(|m| m.id.clone()).collect());
    let started = Instant::now();

    let load_one = |mut market: Market| {
        let store = &store;
        let pending = &pending;
        async move {
            let mut failures = 0u32;
            loop {
                if let Some(price) = fetcher.fetch(&market).await {
                    market.set_price(price);
                    let id = market.id.clone();
                    store(market).await?;
                    pending.lock().remove(&id);
                    return Result::Ok(());
                }

                failures += 1;
                let pause = policy.backoff(failures);
                warn!(
                    mkt = market.id,
                    "FX price not available (attempt {failures}), retrying in {pause:?}"
                );
                sleep(pause).await;
            }
        }
    };

    let all = join_all(markets.into_iter().map(load_one));
    match timeout(policy.budget, all).await {
        Ok(results) => {
            results.into_iter().collect::<Result<Vec<_>>>()?;
            info!("FX rates loaded in {:?}", started.elapsed());
            Ok(FxLoadOutcome::Loaded)
        }
        Err(_) => {
            let mut missing = pending.lock().iter().cloned().collect::<Vec<_>>();
            missing.sort();
            Ok(FxLoadOutcome::TimedOut { missing })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use jiff::Timestamp;

    use super::*;

    /// Fake provider that answers a market only after a number of failed attempts.
    struct FlakyFetcher {
        failures_before_success: usize,
        attempts: Mutex<HashMap<MarketId, usize>>,
        calls: AtomicUsize,
    }

    impl FlakyFetcher {
        /// A fetcher that fails `failures_before_success` times per market, then answers.
        /// `usize::MAX` never answers.
        fn new(failures_before_success: usize) -> Self {
            Self {
                failures_before_success,
                attempts: Mutex::new(HashMap::new()),
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl FxPriceFetcher for FlakyFetcher {
        async fn fetch(&self, market: &Market) -> Option<Price> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut attempts = self.attempts.lock();
            let attempt = attempts.entry(market.id.clone()).or_default();
            *attempt += 1;
            (*attempt > self.failures_before_success).then(|| Price::new(1.1, Timestamp::now()))
        }
    }

    fn collecting_store(
        stored: &Mutex<Vec<MarketId>>,
    ) -> impl Fn(Market) -> std::future::Ready<Result<()>> + '_ {
        move |market| {
            assert!(
                market.price().is_some(),
                "a market is stored with its price"
            );
            stored.lock().push(market.id);
            std::future::ready(Ok(()))
        }
    }

    #[test]
    fn backoff_doubles_and_is_capped() {
        // GIVEN the default policy (1s initial, 30s cap),
        // WHEN the pauses after consecutive failures are computed,
        // THEN they double each time until they reach the cap.
        let policy = FxStartupPolicy::default();

        let pauses = (1..=7)
            .map(|failures| policy.backoff(failures).as_secs())
            .collect::<Vec<_>>();

        assert_eq!(pauses, vec![1, 2, 4, 8, 16, 30, 30]);
    }

    #[tokio::test(start_paused = true)]
    async fn prices_that_appear_after_retries_are_loaded_and_stored() {
        // GIVEN seven unpriced FX markets and a provider that fails three times per ticker,
        // WHEN the gate loads them within the default two minute budget,
        // THEN every market is stored with its price after 1s + 2s + 4s of backoff.
        let fetcher = FlakyFetcher::new(3);
        let stored = Mutex::new(Vec::new());
        let started = Instant::now();

        let outcome = load_prices(
            fx_markets(),
            &fetcher,
            FxStartupPolicy::default(),
            collecting_store(&stored),
        )
        .await
        .unwrap();

        assert_eq!(outcome, FxLoadOutcome::Loaded);
        assert_eq!(stored.lock().len(), YAHOO_FX_TICKERS.len());
        assert_eq!(
            fetcher.calls.load(Ordering::SeqCst),
            4 * YAHOO_FX_TICKERS.len()
        );
        assert_eq!(started.elapsed(), Duration::from_secs(7));
    }

    #[tokio::test(start_paused = true)]
    async fn missing_prices_time_out_after_the_budget_and_name_the_markets() {
        // GIVEN a provider that never answers,
        // WHEN the gate loads the FX markets,
        // THEN it gives up exactly when the two minute budget is spent, stores nothing
        // and reports every market as still missing.
        let fetcher = FlakyFetcher::new(usize::MAX);
        let stored = Mutex::new(Vec::new());
        let started = Instant::now();

        let outcome = load_prices(
            fx_markets(),
            &fetcher,
            FxStartupPolicy::default(),
            collecting_store(&stored),
        )
        .await
        .unwrap();

        let FxLoadOutcome::TimedOut { missing } = outcome else {
            panic!("expected a timeout");
        };
        assert_eq!(missing.len(), YAHOO_FX_TICKERS.len());
        assert!(missing.contains(&"eurusd".to_string()));
        assert!(stored.lock().is_empty());
        assert_eq!(started.elapsed(), Duration::from_secs(120));
    }

    #[tokio::test(start_paused = true)]
    async fn a_store_failure_aborts_the_gate() {
        // GIVEN a provider that answers but a repository that cannot store prices,
        // WHEN the gate loads the FX markets,
        // THEN the repository error is returned instead of waiting for the budget.
        let fetcher = FlakyFetcher::new(0);

        let result = load_prices(
            fx_markets(),
            &fetcher,
            FxStartupPolicy::default(),
            |market: Market| async move { Err(DcaError::RepositoryStoreFailure(market.id)) },
        )
        .await;

        assert!(matches!(result, Err(DcaError::RepositoryStoreFailure(_))));
    }
}
