use std::{sync::Arc, time::Duration};

use jiff::{Timestamp, civil::Date, tz::TimeZone};
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::{
    AppContext, DateTime,
    app::{
        domain::{
            entity::Market,
            market_data_utils::{fetch_market_price, price_provider_for},
        },
        infra::utils::{StopToken, should_stop},
        services::{fx_rates::seed_fx_catalog, market_data::MarketDataService},
    },
    config::PriceProvider,
    error::Result,
    ports::outbound::{
        adapter::PriceProviders,
        repository::{MiscRepository, market_data::MarketDataRepository},
    },
};

/// Number of priced markets stored per transaction during discovery.
const MARKET_FLUSH_SIZE: usize = 50;
const INITIAL_DELAY: Duration = Duration::from_millis(50);
const DAILY_CHECK_INTERVAL: Duration = Duration::from_secs(60);
const HOURLY_INTERVAL: Duration = Duration::from_secs(60 * 60);
const INITIAL_RECOVERY_BACKOFF: Duration = Duration::from_secs(60);
const MAX_RECOVERY_BACKOFF: Duration = Duration::from_secs(60 * 60);

/// Worker that reconciles Kraken markets hourly and discovers new assets daily.
pub struct MarketDiscoveryWorker {
    market_data_service: Arc<MarketDataService>,
    misc_repo: Arc<MiscRepository>,
    market_data_repo: Arc<MarketDataRepository>,
    price_provider: PriceProvider,
    providers: Arc<PriceProviders>,
}

impl MarketDiscoveryWorker {
    pub fn new(ctx: &AppContext) -> Self {
        let market_data_service = ctx.services.mkt_data.clone();
        let misc_repo = ctx.repos.misc.clone();
        let market_data_repo = ctx.repos.mkt_data.clone();
        let price_provider = ctx.config.app.providers.price_provider;
        let providers = ctx.providers.clone();

        Self {
            market_data_service,
            misc_repo,
            market_data_repo,
            price_provider,
            providers,
        }
    }

    pub async fn run(&self, mut stop_token: StopToken) -> Result<()> {
        // FX never depends on Kraken: seed it before any Kraken call is made. The
        // startup gate has normally priced the FX markets already and the price
        // updater keeps them fresh, so seeding is idempotent and stores no prices.
        if let Err(e) = self.seed_fx_catalog().await {
            error!("Failed to seed the fiat catalog and FX markets: {:?}", e);
        }

        let mut reconciliation_stop_token = stop_token.clone();
        let reconciliation = self.run_market_reconciliation(&mut reconciliation_stop_token);
        let discovery = self.run_market_discovery(&mut stop_token);

        tokio::join!(reconciliation, discovery);

        Ok(())
    }

    async fn run_market_reconciliation(&self, stop_token: &mut StopToken) {
        let mut interval =
            tokio::time::interval_at(Instant::now() + INITIAL_DELAY, HOURLY_INTERVAL);

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = should_stop(stop_token) => break,
            }

            if let Err(e) = self.reconcile_markets().await {
                error!("Failed to reconcile Kraken markets: {:?}", e);
            }
        }
    }

    async fn run_market_discovery(&self, stop_token: &mut StopToken) {
        let mut interval =
            tokio::time::interval_at(Instant::now() + INITIAL_DELAY, DAILY_CHECK_INTERVAL);
        let mut recovery_backoff = Duration::ZERO;
        let mut next_recovery_attempt = Instant::now();
        // Set while a failed discovery waits to be retried with backoff.
        let mut recovery_pending = false;

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = should_stop(stop_token) => break,
            }

            let res = is_outdated(&self.misc_repo).await;
            if let Err(e) = res {
                error!("Failed to fetch last update time: {:?}", e);
                continue;
            }

            let (is_outdated, last_fetched_ts) = res.unwrap();
            let recovery_due = recovery_pending && Instant::now() >= next_recovery_attempt;
            if recovery_pending && !recovery_due {
                debug!(
                    "Kraken asset recovery is waiting for its next retry ({})",
                    next_recovery_attempt
                        .saturating_duration_since(Instant::now())
                        .as_secs()
                );
                continue;
            }
            if !recovery_pending && !is_outdated {
                debug!(
                    "Kraken assets already fetched today ({})",
                    last_fetched_ts.map(|t| t.to_string()).unwrap_or_default()
                );
                continue;
            }

            info!(
                last_completed = last_fetched_ts.map(|t| t.to_string()).unwrap_or_default(),
                recovery_pending, "Starting market discovery"
            );
            let started = Instant::now();
            let outcome = self.discover_new_markets().await;
            info!(
                elapsed_secs = started.elapsed().as_secs(),
                ok = outcome.is_ok(),
                "Market discovery finished"
            );
            match outcome {
                Err(e) => {
                    error!("Failed to update Kraken Assets and Markets data: {:?}", e);
                    recovery_pending = true;
                    recovery_backoff = next_recovery_backoff(recovery_backoff);
                    next_recovery_attempt = Instant::now() + recovery_backoff;
                }
                Ok(()) => match self.misc_repo.set_cw_last_fetched(Timestamp::now()).await {
                    Ok(()) => {
                        recovery_pending = false;
                        recovery_backoff = Duration::ZERO;
                        next_recovery_attempt = Instant::now();
                    }
                    Err(e) => {
                        error!("Failed to update last update time: {:?}", e);
                        recovery_pending = true;
                        recovery_backoff = next_recovery_backoff(recovery_backoff);
                        next_recovery_attempt = Instant::now() + recovery_backoff;
                    }
                },
            }
        }
    }

    async fn reconcile_markets(&self) -> Result<()> {
        let market_ids = self.providers.kraken.fetch_tradable_market_ids().await?;
        let deleted = self
            .market_data_repo
            .delete_markets_not_in(&market_ids)
            .await?;

        if !deleted.is_empty() {
            info!(
                "Removed {} stale Kraken markets from the database: {:?}",
                deleted.len(),
                deleted
            );
            self.market_data_service.invalidate_markets(&deleted);
        }

        Ok(())
    }

    /// Idempotently stores the fiat catalog and the FX markets (see
    /// [`seed_fx_catalog`]); prices are the startup gate's and the price updater's job.
    async fn seed_fx_catalog(&self) -> Result<()> {
        seed_fx_catalog(
            &self.market_data_repo,
            &self.market_data_service,
            self.price_provider,
        )
        .await
    }

    async fn discover_new_markets(&self) -> Result<()> {
        self.seed_fx_catalog().await?;

        // Collect assets and markets from Kraken
        let (assets, markets) = self
            .providers
            .kraken
            .fetch_assets(&self.market_data_repo)
            .await?;

        // All market assets must be present before any market record is written;
        // one atomic statement stores them all.
        let asset_result = self.market_data_repo.store_assets(&assets).await;
        self.market_data_service.invalidate_asset_cache();
        if let Err(error) = &asset_result {
            error!(error = %error, count = assets.len(), "Failed to store discovered assets");
        }
        asset_result?;

        // Fetch prices market by market, storing them in batches instead of one
        // transaction per market.
        let mut batch: Vec<(Market, PriceProvider)> = Vec::with_capacity(MARKET_FLUSH_SIZE);
        for mut m in markets {
            info!("Fetching price for market '{}'", m.id);
            let provider = price_provider_for(&m, self.price_provider);
            let Some(price) = fetch_market_price(&m, &self.providers, provider).await else {
                continue;
            };

            m.set_price(price);
            batch.push((m, provider));

            if batch.len() >= MARKET_FLUSH_SIZE {
                self.flush_markets(&mut batch).await?;
            }
        }
        self.flush_markets(&mut batch).await?;

        self.market_data_service.invalidate_market_cache();

        Ok(())
    }

    /// Stores and clears the pending priced markets in one transaction.
    ///
    /// The market cache is invalidated even when the write fails.
    async fn flush_markets(&self, batch: &mut Vec<(Market, PriceProvider)>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        info!("Storing {} markets", batch.len());
        let markets = batch
            .iter()
            .map(|(market, provider)| (market, *provider))
            .collect::<Vec<_>>();
        let result = self.market_data_repo.store_markets(&markets).await;
        self.market_data_service.invalidate_market_cache();
        batch.clear();
        result
    }
}

fn next_recovery_backoff(previous: Duration) -> Duration {
    if previous.is_zero() {
        return INITIAL_RECOVERY_BACKOFF;
    }

    previous.saturating_mul(2).min(MAX_RECOVERY_BACKOFF)
}

async fn is_outdated(misc: &MiscRepository) -> Result<(bool, Option<DateTime>)> {
    let last_fetched = misc.get_cw_last_fetched().await?;
    let today = Timestamp::now().to_zoned_datetime(TimeZone::UTC).date();
    Ok((is_outdated_at(last_fetched, today), last_fetched))
}

fn is_outdated_at(last_fetched: Option<DateTime>, today: Date) -> bool {
    last_fetched
        .map(|ts| ts.to_zoned_datetime(TimeZone::UTC).date() < today)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::{
        HOURLY_INTERVAL, INITIAL_RECOVERY_BACKOFF, MAX_RECOVERY_BACKOFF, is_outdated_at,
        next_recovery_backoff,
    };
    use jiff::{civil::date, tz::TimeZone};

    #[test]
    fn missing_daily_marker_is_outdated() {
        // GIVEN daily discovery has never completed
        // WHEN the worker checks the marker
        let outdated = is_outdated_at(None, date(2026, 9, 2));

        // THEN discovery is due
        assert!(outdated);
    }

    #[test]
    fn same_day_daily_marker_is_not_outdated() {
        // GIVEN daily discovery completed earlier on the current UTC date
        let last_fetched = date(2026, 9, 2)
            .at(1, 2, 3, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .unwrap()
            .timestamp();

        // WHEN the worker checks the marker
        let outdated = is_outdated_at(Some(last_fetched), date(2026, 9, 2));

        // THEN discovery is not repeated until the next UTC date
        assert!(!outdated);
    }

    #[test]
    fn previous_day_daily_marker_is_outdated() {
        // GIVEN daily discovery completed on the previous UTC date
        let last_fetched = date(2026, 9, 1)
            .at(23, 59, 59, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .unwrap()
            .timestamp();

        // WHEN the worker checks the marker
        let outdated = is_outdated_at(Some(last_fetched), date(2026, 9, 2));

        // THEN discovery is due again
        assert!(outdated);
    }

    #[test]
    fn market_reconciliation_runs_hourly() {
        // GIVEN the market reconciliation schedule
        // WHEN its interval is read
        // THEN the worker waits one hour between runs
        assert_eq!(HOURLY_INTERVAL, std::time::Duration::from_secs(3600));
    }

    #[test]
    fn recovery_backoff_grows_exponentially_and_is_capped() {
        // GIVEN a failed recovery attempt
        let first = next_recovery_backoff(std::time::Duration::ZERO);
        let second = next_recovery_backoff(first);

        // WHEN repeated failures extend the retry delay
        let capped = (0..16).fold(second, |delay, _| next_recovery_backoff(delay));

        // THEN retries become less frequent but never exceed the one-hour cap
        assert_eq!(first, INITIAL_RECOVERY_BACKOFF);
        assert_eq!(second, INITIAL_RECOVERY_BACKOFF * 2);
        assert_eq!(capped, MAX_RECOVERY_BACKOFF);
    }
}
