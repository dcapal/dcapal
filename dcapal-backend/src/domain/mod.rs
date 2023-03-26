pub mod command;
pub mod entity;
pub mod utils;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{error, info, warn};

use crate::{
    config::Config,
    error::{DcaError, Result},
    repository::market_data::MarketDataRepository,
    Provider,
};

use self::{
    command::ConversionRateQuery,
    entity::{Asset, AssetId, AssetKind, Market, MarketId, Price},
    utils::{Expiring, ExpiringOnceCell, ExpiringOption},
};

pub struct MarketDataService {
    config: Arc<Config>,
    repo: Arc<MarketDataRepository>,
    providers: Arc<Provider>,
    mkt_loaders: DashMap<MarketId, Arc<ExpiringOnceCell<Option<Arc<Market>>>>>,
    pricers: DashMap<(AssetId, AssetId), Arc<ExpiringOnceCell<ExpiringOption<Price>>>>,
    assets_cache: RwLock<AssetsCache>,
    mkt_cache: DashMap<MarketId, Arc<Market>>,
    px_cache: DashMap<(AssetId, AssetId), Price>,
}

impl MarketDataService {
    const DEFAULT_TTL: Duration = Duration::from_secs(5 * 60);

    pub fn new(
        config: Arc<Config>,
        repo: Arc<MarketDataRepository>,
        providers: Arc<Provider>,
    ) -> Self {
        let mkt_loaders = DashMap::new();
        let pricers = DashMap::new();
        let assets_cache = RwLock::new(AssetsCache::new());
        let mkt_cache = DashMap::new();
        let px_cache = DashMap::new();

        Self {
            config,
            repo,
            providers,
            mkt_loaders,
            pricers,
            assets_cache,
            mkt_cache,
            px_cache,
        }
    }

    pub async fn get_assets_by_type(&self, kind: AssetKind) -> Arc<Vec<Asset>> {
        {
            let cache = self.assets_cache.read();
            match kind {
                AssetKind::Crypto => {
                    if let Some(assets) = &cache.crypto {
                        return assets.clone();
                    }
                }
                AssetKind::Fiat => {
                    if let Some(assets) = &cache.fiats {
                        return assets.clone();
                    }
                }
            }
        }

        let assets = self
            .repo
            .load_assets_by_type(kind)
            .await
            .unwrap_or_else(|e| {
                error!("{:?}", e);
                vec![]
            });

        let mut cache = self.assets_cache.write();

        match kind {
            AssetKind::Crypto => {
                cache.crypto = Some(Arc::new(assets));
                cache.crypto.as_ref().unwrap().clone()
            }
            AssetKind::Fiat => {
                cache.fiats = Some(Arc::new(assets));
                cache.fiats.as_ref().unwrap().clone()
            }
        }
    }

    fn invalidate_asset_cache(&self) {
        let mut cache = self.assets_cache.write();
        cache.crypto = None;
        cache.fiats = None;
    }

    pub async fn get_market(&self, id: MarketId) -> Result<Option<Arc<Market>>> {
        // Check market from cache
        if let Some(mkt) = self.mkt_cache.get(&id) {
            if !mkt.is_price_outdated() {
                return Ok(Some(mkt.clone()));
            }
        }

        let loader = self
            .mkt_loaders
            .entry(id.clone())
            .or_insert_with(|| {
                Arc::new(ExpiringOnceCell::new(|market: &Option<Arc<Market>>| {
                    if let Some(ref m) = market {
                        m.is_price_outdated()
                    } else {
                        false
                    }
                }))
            })
            .clone();

        let market = loader
            .get_or_try_init(|| async { self.load_market(&id).await })
            .await;

        let Err(e) = market else { return market; };

        error!("{:?}", e);
        if matches!(e, DcaError::PriceNotAvailableId(_)) {
            Err(e)
        } else {
            Ok(None)
        }
    }

    async fn load_market(&self, id: &MarketId) -> Result<Option<Arc<Market>>> {
        let mkt = self.repo.find_market(id).await.map_err(|e| {
            error!(mkt = id, "Error occured in loading market: {}", e);
            DcaError::MarketNotFound(id.clone())
        })?;

        if mkt.is_none() {
            info!("Cannot find market '{}'", id);
            return Ok(None);
        }

        let mkt = self
            .refresh_mkt_price(mkt.unwrap())
            .await
            .map(Arc::new)
            .map_err(|e| {
                error!(mkt = id, "Error occured in fetching price: {:?}", e);
                DcaError::PriceNotAvailableId(id.clone())
            })?;

        if let Err(e) = self.repo.update_mkt_price(&mkt).await {
            error!(mkt = id, "Failed to update price: {}", e);
        }

        Ok(Some(mkt))
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

    pub async fn get_conversion_rate(&self, cmd: ConversionRateQuery) -> Result<Option<Price>> {
        let (base, quote) = (cmd.base.id(), cmd.quote.id());
        let pair = (base.clone(), quote.clone());

        // Check price from cache
        if let Some(price) = self.px_cache.get(&pair) {
            if !price.is_outdated() {
                return Ok(Some(*price));
            }
        }

        let pricer = self
            .pricers
            .entry(pair.clone())
            .or_insert_with(|| {
                Arc::new(ExpiringOnceCell::new(|p: &ExpiringOption<Price>| {
                    p.is_outdated()
                }))
            })
            .clone();

        let price = pricer
            .get_or_try_init(|| async { self.compute_conversion_rate(base, quote).await })
            .await;

        let Err(e) = price else { return price.map(|p| p.into()); };

        if let DcaError::PriceNotAvailableId(ref id) = e {
            if let Some(p) = pricer.get().await {
                if p.is_expired {
                    warn!("Serving outdated price for Market '{id}'");
                }

                return Ok(p.value.into());
            }
        }

        Err(e)
    }

    async fn compute_conversion_rate(
        &self,
        base: &AssetId,
        quote: &AssetId,
    ) -> Result<ExpiringOption<Price>> {
        // Base/base => 1.
        if base == quote {
            return Ok(ExpiringOption::Some(Price::new(1., Utc::now())));
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
        let mkt = self.get_market(id).await?;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                return Ok(ExpiringOption::Some(*px));
            }
        }

        // Find quote/base market
        let id = format!("{}{}", quote, base);
        let mkt = self.get_market(id).await?;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                return Ok(ExpiringOption::Some(Price::new(1. / px.price, px.ts)));
            }
        }

        // Find alternative markets
        let Some(base_usd_px) = self.get_base_usd_price(&base, &quote).await? else {
            return Ok(ExpiringOption::None(Instant::now(), Self::DEFAULT_TTL));
        };

        let usd_quote_id = format!("{}{}", "usd", quote);
        let quote_usd_id = format!("{}{}", quote, "usd");

        let usd_quote = self.get_market(usd_quote_id.clone()).await?;
        if let Some(usd_quote) = usd_quote {
            if let Some(usd_quote_px) = usd_quote.price() {
                let price = base_usd_px.price * usd_quote_px.price;
                let ts = std::cmp::min(base_usd_px.ts, usd_quote_px.ts);
                return Ok(ExpiringOption::Some(Price::new(price, ts)));
            }
        }

        let quote_usd = self.get_market(quote_usd_id.clone()).await?;
        if let Some(quote_usd) = quote_usd {
            if let Some(quote_usd_px) = quote_usd.price() {
                let price = base_usd_px.price / quote_usd_px.price;
                let ts = std::cmp::min(base_usd_px.ts, quote_usd_px.ts);
                return Ok(ExpiringOption::Some(Price::new(price, ts)));
            }
        }

        warn!(
            base = base,
            quote = quote,
            "Price not available for markets '{}' and '{}'",
            usd_quote_id,
            quote_usd_id
        );

        Ok(ExpiringOption::None(Instant::now(), Self::DEFAULT_TTL))
    }

    async fn get_base_usd_price(&self, base: &AssetId, quote: &AssetId) -> Result<Option<Price>> {
        let base_usd_id = format!("{}{}", base, "usd");
        let usd_base_id = format!("{}{}", "usd", base);

        let base_usd = self.get_market(base_usd_id.clone()).await?;
        if let Some(ref m) = base_usd {
            if let Some(px) = m.price() {
                return Ok(Some(*px));
            }
        }

        let usd_base = self.get_market(usd_base_id.clone()).await?;
        if let Some(ref m) = usd_base {
            if let Some(px) = m.price() {
                return Ok(Some(Price {
                    price: 1. / px.price,
                    ts: px.ts,
                }));
            }
        }

        match (base_usd, usd_base) {
            (None, None) => {
                warn!(
                    base = base,
                    quote = quote,
                    "Cannot find any of markets: '{}', '{}'",
                    base_usd_id,
                    usd_base_id
                );
                Ok(None)
            }
            (_, _) => {
                warn!(
                    base = base,
                    quote = quote,
                    "Price not available for any of markets: '{}', '{}'",
                    base_usd_id,
                    usd_base_id
                );
                Ok(None)
            }
        }
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
        self.invalidate_asset_cache();
        self.mkt_cache.clear();
        self.px_cache.clear();

        Ok(())
    }

    pub async fn update_market_prices(&self) -> Result<()> {
        let markets = self.repo.load_markets().await?;

        for m in &markets {
            // Trigger market refresh
            if let Err(e) = self.get_market(m.id.clone()).await {
                warn!("{:?}", e);
            }
            // Please the rate limiter
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }
}

struct AssetsCache {
    fiats: Option<Arc<Vec<Asset>>>,
    crypto: Option<Arc<Vec<Asset>>>,
}

impl AssetsCache {
    fn new() -> Self {
        Self {
            fiats: None,
            crypto: None,
        }
    }
}
