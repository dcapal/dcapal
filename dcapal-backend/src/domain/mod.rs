pub mod command;
pub mod entity;

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use dashmap::DashMap;
use futures::Future;
use tokio::sync::{OnceCell, RwLock};
use tracing::{error, info, warn};

use crate::{
    config::Config,
    error::{DcaError, Result},
    repository::MarketDataRepository,
    Provider,
};

use self::{
    command::ConversionRateQuery,
    entity::{AssetId, Market, MarketId, Price},
};

pub struct MarketDataService {
    config: Arc<Config>,
    repo: Arc<MarketDataRepository>,
    providers: Arc<Provider>,
    mkt_loaders: DashMap<MarketId, Arc<ExpiringOnceCell<Arc<Market>>>>,
    pricers: DashMap<(AssetId, AssetId), Arc<ExpiringOnceCell<Price>>>,
    mkt_cache: DashMap<MarketId, Arc<Market>>,
    px_cache: DashMap<(AssetId, AssetId), Price>,
}

impl MarketDataService {
    pub fn new(
        config: Arc<Config>,
        repo: Arc<MarketDataRepository>,
        providers: Arc<Provider>,
    ) -> Self {
        let mkt_loaders = DashMap::new();
        let pricers = DashMap::new();
        let mkt_cache = DashMap::new();
        let px_cache = DashMap::new();

        Self {
            config,
            repo,
            providers,
            mkt_loaders,
            pricers,
            mkt_cache,
            px_cache,
        }
    }

    pub async fn get_market(&self, id: MarketId) -> Option<Arc<Market>> {
        // Check market from cache
        if let Some(mkt) = self.mkt_cache.get(&id) {
            if !mkt.is_price_outdated() {
                return Some(mkt.clone());
            }
        }

        let loader = self
            .mkt_loaders
            .entry(id.clone())
            .or_insert_with(|| Arc::new(ExpiringOnceCell::default()))
            .clone();

        loader
            .get_or_try_init(
                || async { self.load_market(&id).await },
                |m| m.is_price_outdated(),
            )
            .await
            .map(Some)
            .unwrap_or_else(|e| {
                error!("{}", e);
                None
            })
    }

    async fn load_market(&self, id: &MarketId) -> Result<Arc<Market>> {
        let mkt = self.repo.find_market(id).await.map_err(|e| {
            error!(mkt = id, "Error occured in loading market: {}", e);
            DcaError::MarketNotFound(id.clone())
        })?;

        if mkt.is_none() {
            warn!("Cannot find market '{}'", id);
            return Err(DcaError::MarketNotFound(id.clone()));
        }

        let mkt = self
            .refresh_mkt_price(mkt.unwrap())
            .await
            .map(|m| Some(Arc::new(m)))
            .map_err(|e| {
                error!(mkt = id, "Error occured in fetching price: {}", e);
                DcaError::MarketNotFound(id.clone())
            })?;

        if let Some(mkt) = mkt {
            if let Err(e) = self.repo.update_mkt_price(&mkt).await {
                error!(mkt = id, "Failed to update price: {}", e);
            }
            Ok(mkt)
        } else {
            Err(DcaError::MarketNotFound(id.clone()))
        }
    }

    async fn refresh_mkt_price(&self, mut mkt: Market) -> Result<Market> {
        use crate::config::PriceProvider;

        info!(
            mkt = mkt.id,
            "Fetching price from {}", self.config.app.providers.price_provider
        );

        let now = Utc::now();
        let price = match self.config.app.providers.price_provider {
            PriceProvider::CryptoWatch => self.providers.cw.fetch_market_price(&mkt, now).await?,
            PriceProvider::Yahoo => self.providers.yahoo.fetch_market_price(&mkt, now).await?,
        };
        if let Some(px) = price {
            mkt.set_price(px, now);
            Ok(mkt)
        } else {
            error!(
                mkt = mkt.id,
                "Cannot fetch price for any frequency (ts={})", now
            );
            Ok(mkt)
        }
    }

    pub async fn get_conversion_rate(&self, cmd: ConversionRateQuery) -> Result<Price> {
        let (base, quote) = (cmd.base.id(), cmd.quote.id());
        let pair = (base.clone(), quote.clone());

        // Check price from cache
        if let Some(price) = self.px_cache.get(&pair) {
            if !price.is_outdated() {
                return Ok(*price);
            }
        }

        let pricer = self
            .pricers
            .entry(pair.clone())
            .or_insert_with(|| Arc::new(ExpiringOnceCell::default()))
            .clone();

        pricer
            .get_or_try_init(
                || async { self.compute_conversion_rate(base, quote).await },
                |p| p.is_outdated(),
            )
            .await
    }

    async fn compute_conversion_rate(&self, base: &AssetId, quote: &AssetId) -> Result<Price> {
        // Base/base => 1.
        if base == quote {
            return Ok(Price::new(1., Utc::now()));
        }

        // Handle ETH2
        let base = if base == "eth2" {
            "eth".to_string()
        } else {
            base.to_owned()
        };

        let quote = if quote == "eth2" {
            "eth".to_string()
        } else {
            quote.to_owned()
        };

        // Find base/quote market
        let id = format!("{}{}", base, quote);
        let mkt = self.get_market(id).await;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                return Ok(*px);
            }
        }

        // Find quote/base market
        let id = format!("{}{}", quote, base);
        let mkt = self.get_market(id).await;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                return Ok(Price::new(1. / px.price, px.ts));
            }
        }

        // Find alternative markets
        let base_usd_id = format!("{}{}", base, "usd");
        let usd_quote_id = format!("{}{}", "usd", quote);
        let quote_usd_id = format!("{}{}", quote, "usd");

        let base_usd = self.get_market(base_usd_id.clone()).await;
        let usd_quote = self.get_market(usd_quote_id.clone()).await;
        let quote_usd = self.get_market(quote_usd_id.clone()).await;

        if base_usd.is_none() {
            warn!(
                base = base,
                quote = quote,
                "Cannot find '{}' market",
                base_usd_id
            );
            return Err(DcaError::PriceNotAvailable(base.clone(), quote.clone()));
        }

        let base_usd = base_usd.unwrap();
        if base_usd.price().is_none() {
            warn!(
                base = base,
                quote = quote,
                "Price not available for '{}' market",
                base_usd_id
            );
            return Err(DcaError::PriceNotAvailable(base.clone(), quote.clone()));
        }

        let base_usd_px = base_usd.price().as_ref().unwrap();

        if let Some(usd_quote) = usd_quote {
            if let Some(usd_quote_px) = usd_quote.price() {
                let price = base_usd_px.price * usd_quote_px.price;
                let ts = std::cmp::min(base_usd_px.ts, usd_quote_px.ts);
                return Ok(Price::new(price, ts));
            }
        }

        if let Some(quote_usd) = quote_usd {
            if let Some(quote_usd_px) = quote_usd.price() {
                let price = base_usd_px.price / quote_usd_px.price;
                let ts = std::cmp::min(base_usd_px.ts, quote_usd_px.ts);
                return Ok(Price::new(price, ts));
            }
        }

        warn!(
            base = base,
            quote = quote,
            "Price not available for markets '{}' and '{}'",
            usd_quote_id,
            quote_usd_id
        );

        Err(DcaError::PriceNotAvailable(base.clone(), quote.clone()))
    }

    pub async fn update_cw_data(&self) -> Result<()> {
        // Collect assets and markets from CW
        let (assets, markets) = self.providers.cw.fetch_assets(&self.repo).await?;

        // Store assets in repository
        for a in assets {
            info!("Storing asset '{}'", a.id());
            self.repo.store_asset(&a).await.unwrap_or_else(|e| {
                error!(
                    "Failed to store asset '{}': {} ({})",
                    a.id(),
                    e,
                    serde_json::to_string(&a).unwrap()
                );
            })
        }

        // Store markets in repository
        for m in &markets {
            info!("Storing market '{}'", m.id);
            self.repo.store_market(m).await.unwrap_or_else(|e| {
                error!(
                    "Failed to store market '{}': {} ({})",
                    m.id,
                    e,
                    serde_json::to_string(m).unwrap()
                );
            })
        }

        // Invalidate caches
        self.mkt_cache.clear();
        self.px_cache.clear();

        Ok(())
    }

    pub async fn update_market_prices(&self) -> Result<()> {
        let markets = self.repo.load_markets().await?;

        for m in &markets {
            // Trigger market refresh
            self.get_market(m.id.clone()).await;
            // Please the rate limiter
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }
}

pub struct ExpiringOnceCell<T>
where
    T: Clone,
{
    cell: RwLock<OnceCell<T>>,
}

impl<T> ExpiringOnceCell<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            cell: RwLock::new(OnceCell::new()),
        }
    }

    pub async fn get_or_try_init<E, I, Fut, F>(
        &self,
        init: I,
        is_expired: F,
    ) -> std::result::Result<T, E>
    where
        I: Fn() -> Fut,
        Fut: Future<Output = std::result::Result<T, E>>,
        F: FnOnce(&T) -> bool,
    {
        {
            let lock = self.cell.read().await;
            let v = lock.get_or_try_init(|| async { init().await }).await?;

            if !is_expired(v) {
                return Ok(v.clone());
            }
        }

        // If value is outdated, clear OnceCell if none is updating it
        if self.cell.try_read().is_ok() {
            if let Ok(mut lock) = self.cell.try_write() {
                lock.take();
            }
        }

        // Compute new value
        let lock = self.cell.read().await;
        let v = lock.get_or_try_init(init).await?;

        Ok(v.clone())
    }
}

impl<T> Default for ExpiringOnceCell<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}
