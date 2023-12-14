use chrono::{TimeZone, Utc};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

use crate::{
    app::{
        domain::market_data_utils::fetch_market_price,
        infra::utils::{should_stop, StopToken},
        services::market_data::MarketDataService,
    },
    config::PriceProvider,
    error::Result,
    ports::outbound::{
        adapter::PriceProviders,
        repository::{market_data::MarketDataRepository, MiscRepository},
    },
    AppContext, DateTime,
};

/// Worker to periodically discover new crypto assets and markets. As of today, new markets are
/// checked every 24 hours.
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
        let mut sleep = tokio::time::sleep(Duration::from_millis(50));

        loop {
            tokio::select! {
                _ = sleep => {}
                _ = should_stop(&mut stop_token) => break,
            }

            // Reset next check timeout
            sleep = tokio::time::sleep(Duration::from_secs(60));

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
            }

            let now = Utc::now();
            if let Err(e) = self.misc_repo.set_cw_last_fetched(now).await {
                error!("Failed to update last update time: {:?}", e);
            }

            // Reset next check timeout
            sleep = tokio::time::sleep(Duration::from_secs(60));
        }
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

        Ok(())
    }
}

async fn is_outdated(misc: &MiscRepository) -> Result<(bool, Option<DateTime>)> {
    let last_fetched = misc.get_cw_last_fetched().await?;
    if let Some(ts) = last_fetched {
        let ts_day = Utc.from_utc_datetime(&ts.naive_utc()).date_naive();
        let today = Utc::now().date_naive();
        return Ok((ts_day < today, Some(ts)));
    }

    Ok((true, None))
}
