use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::Utc;
use moka::{policy::EvictionPolicy, sync::Cache};
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

const MAX_MARKET_LOAD_ATTEMPTS: usize = 3;

/// Number of non-FX markets (with their last price) kept in memory by default.
pub const DEFAULT_LAST_PRICE_CAPACITY: u64 = 10_000;

/// Provides cached access to market assets, markets, and conversion rates.
///
/// Markets are cached together with their last price in two [`moka`] caches:
/// FX markets (fiat against fiat) are never evicted, because every cross-currency
/// conversion depends on them and there are only a few hundred; all other markets
/// live in a size-bounded cache that evicts the least recently used entry. Neither
/// cache has a time-to-live: the price worker replaces cached prices in place.
pub struct MarketDataService {
    repo: Arc<MarketDataRepository>,
    fx_markets: Cache<MarketId, Arc<Market>>,
    markets: Cache<MarketId, Arc<Market>>,
    market_cache_generation: AtomicU64,
    // Held for reading while a loaded market is inserted and for writing while the
    // caches are invalidated, so an invalidation can never be followed by a stale insert.
    invalidation_lock: RwLock<()>,
    pricers: RwLock<HashMap<(AssetId, AssetId), Option<Price>>>,
    price_deps: RwLock<HashMap<MarketId, Vec<(AssetId, AssetId)>>>,
    assets_cache: RwLock<AssetsCache>,
}

impl MarketDataService {
    /// Creates a service that keeps up to [`DEFAULT_LAST_PRICE_CAPACITY`] non-FX markets in memory.
    pub fn new(repo: Arc<MarketDataRepository>) -> Self {
        Self::with_last_price_capacity(repo, DEFAULT_LAST_PRICE_CAPACITY)
    }

    /// Creates a service whose non-FX market cache holds at most `capacity` entries.
    pub fn with_last_price_capacity(repo: Arc<MarketDataRepository>, capacity: u64) -> Self {
        // No capacity bound and no TTL: FX markets are replaced in place, never evicted.
        let fx_markets = Cache::builder().build();
        let markets = Cache::builder()
            .max_capacity(capacity)
            .eviction_policy(EvictionPolicy::lru())
            .build();

        Self {
            repo,
            fx_markets,
            markets,
            market_cache_generation: AtomicU64::new(0),
            invalidation_lock: RwLock::new(()),
            pricers: RwLock::new(HashMap::new()),
            price_deps: RwLock::new(HashMap::new()),
            assets_cache: RwLock::new(AssetsCache::new()),
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

    /// Clears cached markets and conversion rates after market reconciliation.
    pub fn invalidate_market_cache(&self) {
        let _guard = self.invalidation_lock.write();
        self.market_cache_generation.fetch_add(1, Ordering::AcqRel);
        self.fx_markets.invalidate_all();
        self.markets.invalidate_all();
        self.pricers.write().clear();
        self.price_deps.write().clear();
    }

    /// Removes deleted markets and conversion rates that depend on them.
    pub fn invalidate_markets(&self, ids: &[MarketId]) {
        let _guard = self.invalidation_lock.write();
        self.market_cache_generation.fetch_add(1, Ordering::AcqRel);
        for id in ids {
            self.fx_markets.invalidate(id);
            self.markets.invalidate(id);
        }

        let mut pricers = self.pricers.write();
        let mut price_deps = self.price_deps.write();
        for id in ids {
            if let Some(deps) = price_deps.remove(id) {
                for dep in deps {
                    pricers.remove(&dep);
                }
            }
        }
    }

    /// Returns the cached market, refreshing its recency in the bounded cache.
    fn cached_market(&self, id: &MarketId) -> Option<Arc<Market>> {
        self.fx_markets.get(id).or_else(|| self.markets.get(id))
    }

    /// Caches a market in the FX cache when both sides are fiat, in the bounded cache otherwise.
    fn cache_for(&self, market: &Market) -> &Cache<MarketId, Arc<Market>> {
        if market.is_fiat() {
            &self.fx_markets
        } else {
            &self.markets
        }
    }

    /// Lookup a [`Market`] by [`MarketId`]
    pub async fn get_market(&self, id: &MarketId) -> Result<Option<Arc<Market>>> {
        for _ in 0..MAX_MARKET_LOAD_ATTEMPTS {
            let generation = self.market_cache_generation.load(Ordering::Acquire);
            if let Some(market) = self.cached_market(id)
                && self.market_cache_generation.load(Ordering::Acquire) == generation
            {
                return Ok(Some(market));
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

            let _guard = self.invalidation_lock.read();
            if self.market_cache_generation.load(Ordering::Acquire) != generation {
                continue;
            }
            self.cache_for(&market).insert(id.clone(), market.clone());
            return Ok(Some(market));
        }

        info!(
            mkt = id,
            "Market cache generation changed during every load attempt"
        );
        Ok(None)
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

    /// Replaces the price of a cached market in place; returns whether the market was cached.
    pub fn set_price(&self, id: &MarketId, price: Price) -> bool {
        let Some(market) = self.cached_market(id) else {
            return false;
        };

        // Update Market price
        let mut updated = market.as_ref().clone();
        updated.set_price(price);
        self.cache_for(&updated)
            .insert(id.clone(), Arc::new(updated));

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
        let id = format!("{base}{quote}");
        let mkt = self.get_market(&id).await?;
        if let Some(m) = mkt
            && let Some(px) = m.price()
        {
            info!("Computed conversion rate for market {}", m.id);
            return Ok(Some((*px, vec![id])));
        }

        // Find quote/base market
        let id = format!("{quote}{base}");
        let mkt = self.get_market(&id).await?;
        if let Some(m) = mkt
            && let Some(px) = m.price()
        {
            let rate = Price::new(1. / px.price, px.ts);
            info!("Computed conversion rate for market {}", m.id);
            return Ok(Some((rate, vec![id])));
        }

        // Find alternative markets
        let Some((base_usd_id, base_usd_px)) = self.get_base_usd_price(&base, &quote).await? else {
            return Ok(None);
        };

        let usd_ccy = "usd";
        let base_quote_id = format!("{base}{quote}");
        let usd_quote_id = format!("{usd_ccy}{quote}");
        let quote_usd_id = format!("{quote}{usd_ccy}");

        let usd_quote = self.get_market(&usd_quote_id).await?;
        if let Some(usd_quote) = usd_quote
            && let Some(usd_quote_px) = usd_quote.price()
        {
            let price = base_usd_px.price * usd_quote_px.price;
            let ts = std::cmp::min(base_usd_px.ts, usd_quote_px.ts);
            let rate = Price::new(price, ts);
            info!(
                "Computed conversion rate for market {} triangulating between markets",
                base_quote_id
            );
            return Ok(Some((rate, vec![base_usd_id, usd_quote_id])));
        }

        let quote_usd = self.get_market(&quote_usd_id).await?;
        if let Some(quote_usd) = quote_usd
            && let Some(quote_usd_px) = quote_usd.price()
        {
            let price = base_usd_px.price / quote_usd_px.price;
            let ts = std::cmp::min(base_usd_px.ts, quote_usd_px.ts);
            let rate = Price::new(price, ts);
            info!(
                "Computed conversion rate for market {} triangulating between markets",
                base_quote_id
            );
            return Ok(Some((rate, vec![base_usd_id, quote_usd_id])));
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
        if let Some(ref m) = base_usd
            && let Some(px) = m.price()
        {
            return Ok(Some((base_usd_id, *px)));
        }

        let usd_base = self.get_market(&usd_base_id).await?;
        if let Some(ref m) = usd_base
            && let Some(px) = m.price()
        {
            return Ok(Some((
                usd_base_id,
                Price {
                    price: 1. / px.price,
                    ts: px.ts,
                },
            )));
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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::{
        app::domain::entity::{Crypto, Fiat},
        config::PriceProvider,
    };

    fn crypto(id: &str) -> Asset {
        Asset::Crypto(Crypto::new_with_id(id.into()))
    }

    fn fiat(id: &str) -> Asset {
        Asset::Fiat(Fiat::new(id.into(), id.to_uppercase()))
    }

    /// Stores every asset and market (quote is always the fiat `eur`) and returns a service
    /// whose non-FX cache holds `capacity` markets.
    async fn service_with(
        pool: PgPool,
        capacity: u64,
        markets: &[(&str, &str)],
    ) -> MarketDataService {
        let repo = Arc::new(MarketDataRepository::new(pool));
        for (base, quote) in markets {
            for asset in [base, quote] {
                let asset = if ["eur", "usd", "gbp"].contains(asset) {
                    fiat(asset)
                } else {
                    crypto(asset)
                };
                repo.store_asset(&asset).await.unwrap();
            }
            let (b, q) = (
                repo.find_asset(&base.to_string()).await.unwrap().unwrap(),
                repo.find_asset(&quote.to_string()).await.unwrap().unwrap(),
            );
            let market = Market::new(
                format!("{base}{quote}"),
                b,
                q,
                Some(Price::new(1., Utc::now())),
            );
            repo.store_market(&market, PriceProvider::Kraken)
                .await
                .unwrap();
        }
        MarketDataService::with_last_price_capacity(repo, capacity)
    }

    fn cached(service: &MarketDataService, id: &str) -> bool {
        service.cached_market(&id.to_string()).is_some()
    }

    /// GIVEN a bounded cache of two entries and several FX markets,
    /// WHEN more markets than the capacity are loaded,
    /// THEN every FX market stays cached while the least recently used other market is evicted.
    #[sqlx::test(migrations = "../../migrations")]
    async fn fx_markets_are_never_evicted_and_others_are_lru(pool: PgPool) {
        let service = service_with(
            pool,
            2,
            &[
                ("usd", "eur"),
                ("eur", "usd"),
                ("btc", "eur"),
                ("eth", "eur"),
                ("ltc", "eur"),
            ],
        )
        .await;
        let get = |id: &'static str| async {
            service.get_market(&id.to_string()).await.unwrap().unwrap();
            service.markets.run_pending_tasks();
        };

        get("usdeur").await;
        get("eurusd").await;
        get("btceur").await;
        get("etheur").await;
        // Reading btceur again makes etheur the least recently used entry.
        get("btceur").await;
        get("ltceur").await;
        service.fx_markets.run_pending_tasks();

        assert!(cached(&service, "usdeur") && cached(&service, "eurusd"));
        assert!(cached(&service, "btceur") && cached(&service, "ltceur"));
        assert!(!cached(&service, "etheur"));
        assert_eq!(service.markets.entry_count(), 2);
    }

    /// GIVEN cached FX and non-FX markets, WHEN the price worker sets a new price,
    /// THEN each market keeps its cache and holds the new price in place.
    #[sqlx::test(migrations = "../../migrations")]
    async fn set_price_replaces_the_cached_price_in_place(pool: PgPool) {
        let service = service_with(pool, 10, &[("usd", "eur"), ("btc", "eur")]).await;
        for id in ["usdeur", "btceur"] {
            service.get_market(&id.to_string()).await.unwrap();
        }

        let fresh = Price::new(42., Utc::now());
        assert!(service.set_price(&"usdeur".to_string(), fresh));
        assert!(service.set_price(&"btceur".to_string(), fresh));
        assert!(!service.set_price(&"unknown".to_string(), fresh));

        assert!(
            service.fx_markets.contains_key("usdeur") && !service.markets.contains_key("usdeur")
        );
        assert!(
            service.markets.contains_key("btceur") && !service.fx_markets.contains_key("btceur")
        );
        let market = service
            .get_market(&"usdeur".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(market.price().unwrap().price, 42.);
    }

    /// GIVEN cached markets of both kinds, WHEN reconciliation invalidates the whole market cache
    /// or deleted markets, THEN they are reloaded from the database on the next lookup.
    #[sqlx::test(migrations = "../../migrations")]
    async fn invalidation_clears_both_caches(pool: PgPool) {
        let service = service_with(pool, 10, &[("usd", "eur"), ("btc", "eur")]).await;
        for id in ["usdeur", "btceur"] {
            service.get_market(&id.to_string()).await.unwrap();
        }

        service.invalidate_markets(&["btceur".to_string()]);
        assert!(cached(&service, "usdeur") && !cached(&service, "btceur"));

        service.invalidate_market_cache();
        assert!(!cached(&service, "usdeur"));
        assert!(
            service
                .get_market(&"usdeur".to_string())
                .await
                .unwrap()
                .is_some()
        );
    }

    /// GIVEN a cached market and a synthetic conversion rate that depends on it,
    /// WHEN the deleted market is invalidated,
    /// THEN the market and its dependent conversion rate are no longer cached.
    #[sqlx::test(migrations = "../../migrations")]
    async fn invalidating_a_market_removes_it_and_dependent_conversion_rates(pool: PgPool) {
        let service = service_with(pool, 10, &[("uni", "eur")]).await;
        let market_id = "unieur".to_string();
        let dependency = ("uni".to_string(), "eur".to_string());
        service.get_market(&market_id).await.unwrap();
        service
            .pricers
            .write()
            .insert(dependency.clone(), Some(Price::new(1., Utc::now())));
        service
            .price_deps
            .write()
            .insert(market_id.clone(), vec![dependency.clone()]);

        service.invalidate_markets(std::slice::from_ref(&market_id));

        assert!(!cached(&service, "unieur"));
        assert!(!service.pricers.read().contains_key(&dependency));
        assert!(!service.price_deps.read().contains_key(&market_id));
    }

    /// GIVEN EUR/GBP is converted by triangulating through `eurusd` and `gbpusd`
    /// (no direct, inverse or `usd{quote}` market exists),
    /// WHEN the price worker refreshes `gbpusd`,
    /// THEN the cached EUR/GBP rate that depended on it is dropped.
    #[sqlx::test(migrations = "../../migrations")]
    async fn triangulated_rate_through_the_quote_usd_market_depends_on_both_usd_markets(
        pool: PgPool,
    ) {
        let service = service_with(pool, 10, &[("eur", "usd"), ("gbp", "usd")]).await;
        let query = ConversionRateQuery {
            base: fiat("eur"),
            quote: fiat("gbp"),
        };
        let pair = ("eur".to_string(), "gbp".to_string());
        assert!(service.get_conversion_rate(query).await.unwrap().is_some());
        assert!(service.pricers.read().contains_key(&pair));

        service.get_market(&"gbpusd".to_string()).await.unwrap();
        service.set_price(&"gbpusd".to_string(), Price::new(2., Utc::now()));

        assert!(!service.pricers.read().contains_key(&pair));
    }
}
