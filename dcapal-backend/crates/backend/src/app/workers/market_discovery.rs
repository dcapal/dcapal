use std::{sync::Arc, time::Duration};

use chrono::{NaiveDate, TimeZone, Utc};
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::{
    AppContext, DateTime,
    app::{
        domain::market_data_utils::fetch_market_price,
        infra::utils::{StopToken, should_stop},
        services::market_data::MarketDataService,
    },
    config::PriceProvider,
    error::Result,
    ports::outbound::{
        adapter::PriceProviders,
        repository::{MiscRepository, market_data::MarketDataRepository},
    },
};

const INITIAL_DELAY: Duration = Duration::from_millis(50);
const DAILY_CHECK_INTERVAL: Duration = Duration::from_secs(60);
const HOURLY_INTERVAL: Duration = Duration::from_secs(60 * 60);

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

    pub async fn run(&self, mut stop_token: StopToken) {
        let mut reconciliation_stop_token = stop_token.clone();
        let reconciliation = self.run_market_reconciliation(&mut reconciliation_stop_token);
        let discovery = self.run_market_discovery(&mut stop_token);

        tokio::join!(reconciliation, discovery);
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
            if !is_outdated {
                debug!(
                    "Kraken assets already fetched today ({})",
                    last_fetched_ts.map(|t| t.to_string()).unwrap_or_default()
                );
                continue;
            }

            if let Err(e) = self.discover_new_markets().await {
                error!("Failed to update Kraken Assets and Markets data: {:?}", e);
            } else {
                let now = Utc::now();
                if let Err(e) = self.misc_repo.set_cw_last_fetched(now).await {
                    error!("Failed to update last update time: {:?}", e);
                }
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
                "Removed {} stale Kraken markets from Redis: {:?}",
                deleted.len(),
                deleted
            );
            self.market_data_service.invalidate_markets(&deleted);
        }

        Ok(())
    }

    async fn discover_new_markets(&self) -> Result<()> {
        // Collect assets and markets from Kraken
        let (assets, markets) = self
            .providers
            .kraken
            .fetch_assets(&self.market_data_repo)
            .await?;

        // Store assets in repository
        for a in assets {
            info!("Storing asset '{}'", a.id());
            self.market_data_repo
                .store_asset(&a)
                .await
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to store asset '{}': {} ({})",
                        a.id(),
                        e,
                        serde_json::to_string(&a).unwrap()
                    );
                })
        }

        // Store markets in repository
        for mut m in markets {
            info!("Fetching price for market '{}'", m.id);
            let Some(price) = fetch_market_price(&m, &self.providers, self.price_provider).await
            else {
                continue;
            };

            m.set_price(price);

            info!("Storing market '{}'", m.id);
            if let Err(e) = self.market_data_repo.store_market(&m).await {
                error!(
                    "Failed to store market '{}': {} ({})",
                    m.id,
                    e,
                    serde_json::to_string(&m).unwrap()
                );
            }
        }

        self.market_data_service.invalidate_asset_cache();
        self.market_data_service.invalidate_market_cache();

        Ok(())
    }
}

async fn is_outdated(misc: &MiscRepository) -> Result<(bool, Option<DateTime>)> {
    let last_fetched = misc.get_cw_last_fetched().await?;
    let today = Utc::now().date_naive();
    Ok((is_outdated_at(last_fetched, today), last_fetched))
}

fn is_outdated_at(last_fetched: Option<DateTime>, today: NaiveDate) -> bool {
    last_fetched
        .map(|ts| Utc.from_utc_datetime(&ts.naive_utc()).date_naive() < today)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::{HOURLY_INTERVAL, is_outdated_at};

    #[test]
    fn missing_daily_marker_is_outdated() {
        // GIVEN daily discovery has never completed
        // WHEN the worker checks the marker
        let outdated = is_outdated_at(None, NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());

        // THEN discovery is due
        assert!(outdated);
    }

    #[test]
    fn same_day_daily_marker_is_not_outdated() {
        // GIVEN daily discovery completed earlier on the current UTC date
        let last_fetched = Utc.with_ymd_and_hms(2026, 9, 2, 1, 2, 3).single();

        // WHEN the worker checks the marker
        let outdated = is_outdated_at(last_fetched, NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());

        // THEN discovery is not repeated until the next UTC date
        assert!(!outdated);
    }

    #[test]
    fn previous_day_daily_marker_is_outdated() {
        // GIVEN daily discovery completed on the previous UTC date
        let last_fetched = Utc.with_ymd_and_hms(2026, 9, 1, 23, 59, 59).single();

        // WHEN the worker checks the marker
        let outdated = is_outdated_at(last_fetched, NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());

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
}
