use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tracing::{error, info, warn};

use crate::{
    app::{
        domain::market_data_utils::fetch_market_price,
        infra::utils::{should_stop, StopToken},
        services::market_data::MarketDataService,
    },
    config::PriceProvider,
    error::Result,
    ports::outbound::{adapter::PriceProviders, repository::market_data::MarketDataRepository},
    AppContext,
};

/// Worker periodically updating market prices. As of today, prices are refreshed every 5 minutes.
pub struct PriceUpdaterWorker {
    period: Duration,
    market_data_service: Arc<MarketDataService>,
    market_data_repo: Arc<MarketDataRepository>,
    price_provider: PriceProvider,
    providers: Arc<PriceProviders>,
}

impl PriceUpdaterWorker {
    pub fn new(ctx: &AppContext, period: Duration) -> Self {
        let market_data_service = ctx.services.mkt_data.clone();
        let market_data_repo = ctx.repos.mkt_data.clone();
        let price_provider = ctx.config.app.providers.price_provider;
        let providers = ctx.providers.clone();

        Self {
            period,
            market_data_service,
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

            if let Err(e) = self.update_prices().await {
                error!("Error occurred while updating prices: {e:?}");
            }

            sleep = tokio::time::sleep(self.period);
            let next = Utc::now() + chrono::Duration::from_std(self.period).unwrap();
            info!("Next PriceUpdaterWorker execution: {next}");
        }
    }

    async fn update_prices(&self) -> Result<()> {
        // Get all known markets
        let markets = self.market_data_repo.load_markets().await?;

        for mut m in markets {
            let Some(price) = fetch_market_price(&m, &self.providers, self.price_provider).await
            else {
                warn!("Failed to fetch price update for market {}", m.id);
                continue;
            };

            m.set_price(price);
            if let Err(e) = self.market_data_repo.update_mkt_price(&m).await {
                error!("Failed to store market price update {m:?}: {e:?}");
            }

            self.market_data_service.set_price(&m.id, price);

            // Please the rate limiter
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }
}
