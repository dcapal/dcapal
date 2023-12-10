use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use parking_lot::RwLock;
use tracing::{error, info, warn};

use crate::{
    app::{
        domain::entity::{Asset, AssetId, AssetKind, Market, MarketId, Price},
        services::command::ConversionRateQuery,
    },
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
};

pub struct MarketDataService {
    repo: Arc<MarketDataRepository>,
    markets: RwLock<HashMap<MarketId, Arc<Market>>>,
    pricers: RwLock<HashMap<(AssetId, AssetId), Option<Price>>>,
    price_deps: RwLock<HashMap<MarketId, Vec<(AssetId, AssetId)>>>,
    assets_cache: RwLock<AssetsCache>,
}

impl MarketDataService {
    pub fn new(repo: Arc<MarketDataRepository>) -> Self {
        let markets = RwLock::new(HashMap::new());
        let pricers = RwLock::new(HashMap::new());
        let assets_cache = RwLock::new(AssetsCache::new());
        let price_deps = RwLock::new(HashMap::new());

        Self {
            repo,
            markets,
            pricers,
            price_deps,
            assets_cache,
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

    pub fn invalidate_asset_cache(&self) {
        let mut cache = self.assets_cache.write();
        cache.crypto = None;
        cache.fiats = None;
    }

    /// Lookup a [`Market`] by [`MarketId`]
    pub async fn get_market(&self, id: &MarketId) -> Result<Option<Arc<Market>>> {
        {
            let markets = self.markets.read();
            if let Some(market) = markets.get(id) {
                return Ok(Some(market.clone()));
            }
        }

        let market = match self.load_market(id).await {
            Ok(m) => m,
            Err(e) => {
                error!("{:?}", e);
                None
            }
        };

        let Some(market) = market else {
            return Ok(None);
        };

        let mut markets = self.markets.write();
        markets.insert(id.clone(), market.clone());
        Ok(Some(market))
    }

    async fn load_market(&self, id: &MarketId) -> Result<Option<Arc<Market>>> {
        let mkt = self.repo.find_market(id).await.map_err(|e| {
            error!(mkt = id, "Error occured in loading market: {}", e);
            DcaError::MarketNotFound(id.clone())
        })?;

        match mkt {
            Some(mkt) => Ok(Some(Arc::new(mkt))),
            None => {
                info!("Cannot find market '{}'", id);
                Ok(None)
            }
        }
    }

    pub fn set_price(&self, id: &MarketId, price: Price) -> bool {
        // Get a Market copy
        let mut updated = {
            let markets = self.markets.read();
            let Some(market) = markets.get(id) else {
                return false;
            };
            market.as_ref().clone()
        };

        // Update Market price
        {
            let mut markets = self.markets.write();
            updated.set_price(price);
            markets.insert(id.clone(), Arc::new(updated));
        }

        // Invalidate dependent syntetic rates
        let mut pricers = self.pricers.write();
        let mut price_deps = self.price_deps.write();
        if let Some(deps) = price_deps.get(id) {
            for pair in deps {
                pricers.remove(pair);
            }
        }

        // Clear dependency list
        price_deps.remove(id);

        true
    }

    pub async fn get_conversion_rate(&self, cmd: ConversionRateQuery) -> Result<Option<Price>> {
        let (base, quote) = (cmd.base.id(), cmd.quote.id());
        let pair = (base.clone(), quote.clone());

        {
            let pricers = self.pricers.read();
            if let Some(price) = pricers.get(&pair) {
                return Ok(*price);
            }
        }

        if let Some((price, deps)) = self.compute_conversion_rate(base, quote).await? {
            {
                // Track market dependencies to this syntetic rate
                let mut price_deps = self.price_deps.write();
                for dep in deps {
                    price_deps.entry(dep).or_default().push(pair.clone());
                }
            }

            // Update cached rate
            let mut pricers = self.pricers.write();
            pricers.insert(pair, Some(price));
            Ok(Some(price))
        } else {
            Ok(None)
        }
    }

    async fn compute_conversion_rate(
        &self,
        base: &AssetId,
        quote: &AssetId,
    ) -> Result<Option<(Price, Vec<MarketId>)>> {
        // Base/base => 1.
        if base == quote {
            return Ok(Some((Price::new(1., Utc::now()), vec![])));
        }

        let base = normalized_asset(base);
        let quote = normalized_asset(quote);

        // Find base/quote market
        let id = format!("{}{}", base, quote);
        let mkt = self.get_market(&id).await?;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                info!("Computed conversion rate for market {}", m.id);
                return Ok(Some((*px, vec![id])));
            }
        }

        // Find quote/base market
        let id = format!("{}{}", quote, base);
        let mkt = self.get_market(&id).await?;
        if let Some(m) = mkt {
            if let Some(px) = m.price() {
                let rate = Price::new(1. / px.price, px.ts);
                info!("Computed conversion rate for market {}", m.id);
                return Ok(Some((rate, vec![id])));
            }
        }

        // Find alternative markets
        let Some((base_usd_id, base_usd_px)) = self.get_base_usd_price(&base, &quote).await? else {
            return Ok(None);
        };

        let base_quote_id = format!("{}{}", base, quote);
        let usd_quote_id = format!("{}{}", "usd", quote);
        let quote_usd_id = format!("{}{}", quote, "usd");

        let usd_quote = self.get_market(&usd_quote_id).await?;
        if let Some(usd_quote) = usd_quote {
            if let Some(usd_quote_px) = usd_quote.price() {
                let price = base_usd_px.price * usd_quote_px.price;
                let ts = std::cmp::min(base_usd_px.ts, usd_quote_px.ts);
                let rate = Price::new(price, ts);
                info!(
                    "Computed conversion rate for market {} triangulating between markets",
                    base_quote_id
                );
                return Ok(Some((rate, vec![base_usd_id, usd_quote_id])));
            }
        }

        let quote_usd = self.get_market(&quote_usd_id).await?;
        if let Some(quote_usd) = quote_usd {
            if let Some(quote_usd_px) = quote_usd.price() {
                let price = base_usd_px.price / quote_usd_px.price;
                let ts = std::cmp::min(base_usd_px.ts, quote_usd_px.ts);
                let rate = Price::new(price, ts);
                info!(
                    "Computed conversion rate for market {} triangulating between markets",
                    base_quote_id
                );
                return Ok(Some((rate, vec![base_quote_id, usd_quote_id])));
            }
        }

        warn!(
            base = base,
            quote = quote,
            "Price not available for markets '{}' and '{}'",
            usd_quote_id,
            quote_usd_id
        );

        Ok(None)
    }

    async fn get_base_usd_price(
        &self,
        base: &AssetId,
        quote: &AssetId,
    ) -> Result<Option<(MarketId, Price)>> {
        let base_usd_id = format!("{}{}", base, "usd");
        let usd_base_id = format!("{}{}", "usd", base);

        let base_usd = self.get_market(&base_usd_id).await?;
        if let Some(ref m) = base_usd {
            if let Some(px) = m.price() {
                return Ok(Some((base_usd_id, *px)));
            }
        }

        let usd_base = self.get_market(&usd_base_id).await?;
        if let Some(ref m) = usd_base {
            if let Some(px) = m.price() {
                return Ok(Some((
                    usd_base_id,
                    Price {
                        price: 1. / px.price,
                        ts: px.ts,
                    },
                )));
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
}

fn normalized_asset(id: &AssetId) -> AssetId {
    lazy_static::lazy_static! {
        static ref NORMALIZED: HashMap<&'static str, &'static str> = {
            [("eth2", "eth"), ("eth2.s", "eth")].into_iter().collect()
        };
    }

    NORMALIZED
        .get(id.as_str())
        .unwrap_or(&id.as_str())
        .to_string()
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
