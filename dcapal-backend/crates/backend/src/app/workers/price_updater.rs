use std::{sync::Arc, time::Duration};

use jiff::Timestamp;
use tracing::{error, info, warn};

use crate::{
    AppContext,
    app::{
        domain::{
            entity::{MarketId, Price},
            market_data_utils::{fetch_market_price, price_provider_for},
            market_quote::PriceFailure,
        },
        infra::utils::{StopToken, should_stop},
        services::market_data::MarketDataService,
    },
    config::PriceProvider,
    error::Result,
    ports::outbound::{
        adapter::PriceProviders,
        repository::market_data::{MarketDataRepository, MarketPriceFailure, MarketPriceUpdate},
    },
};

/// Number of fetched prices written to the database by a single flush.
const FLUSH_BATCH_SIZE: usize = 50;

/// Worker periodically updating market prices. As of today, prices are
/// refreshed every 5 minutes.
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
            let next = Timestamp::now() + jiff::Span::try_from(self.period).unwrap();
            info!("Next PriceUpdaterWorker execution: {next}");
        }
    }

    /// Fetches the price of every known market and stores the results in batches.
    ///
    /// Prices are flushed every [`FLUSH_BATCH_SIZE`] markets instead of one write per
    /// market, so a pass costs a handful of statements rather than hundreds.
    async fn update_prices(&self) -> Result<()> {
        // Get all known markets
        let markets = self.market_data_repo.load_markets().await?;

        let mut batch: Vec<(MarketId, Price, PriceProvider)> = Vec::with_capacity(FLUSH_BATCH_SIZE);
        let mut failures: Vec<(MarketId, PriceFailure)> = Vec::new();
        for m in markets {
            let provider = price_provider_for(&m, self.price_provider);
            let price = match fetch_market_price(&m, &self.providers, provider).await {
                Ok(price) => price,
                Err(failure) => {
                    warn!("Failed to fetch price update for market {}", m.id);
                    failures.push((m.id, failure));
                    if failures.len() >= FLUSH_BATCH_SIZE {
                        self.flush_failures(&mut failures).await;
                    }
                    continue;
                }
            };

            self.market_data_service.set_price(&m.id, price);
            batch.push((m.id, price, provider));

            if batch.len() >= FLUSH_BATCH_SIZE {
                self.flush(&mut batch).await;
            }

            // Space out markets (~2 requests per second) so a pass leaves most of the
            // shared Yahoo rate-limit budget to user-driven lookups.
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        self.flush(&mut batch).await;
        self.flush_failures(&mut failures).await;

        Ok(())
    }

    /// Records and clears the pending refresh failures, keeping the last known prices.
    ///
    /// Logs instead of failing the pass.
    async fn flush_failures(&self, failures: &mut Vec<(MarketId, PriceFailure)>) {
        if failures.is_empty() {
            return;
        }

        let pending = failures
            .iter()
            .map(|(market_id, failure)| MarketPriceFailure { market_id, failure })
            .collect::<Vec<_>>();
        if let Err(e) = self.market_data_repo.record_price_failures(&pending).await {
            error!(
                "Failed to record {} market price failures: {e:?}",
                failures.len()
            );
        }
        failures.clear();
    }

    /// Stores and clears the pending prices, logging instead of failing the pass.
    async fn flush(&self, batch: &mut Vec<(MarketId, Price, PriceProvider)>) {
        if batch.is_empty() {
            return;
        }

        let updates = batch
            .iter()
            .map(|(market_id, price, provider)| MarketPriceUpdate {
                market_id,
                price: *price,
                provider: *provider,
            })
            .collect::<Vec<_>>();

        if let Err(e) = self.market_data_repo.upsert_prices(&updates).await {
            error!(
                "Failed to store {} market price updates: {e:?}",
                batch.len()
            );
        }
        batch.clear();
    }
}
