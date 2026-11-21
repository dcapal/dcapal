use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
};

use failsafe::futures::CircuitBreaker;
use futures::StreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use tracing::{debug, error, warn};

use super::DefaultCircuitBreaker;
use crate::{
    DateTime,
    app::domain::entity::{Asset, AssetId, Crypto, Fiat, Market, MarketId, OHLCFrequency},
    app::infra::stats::{MetricProvider, ProviderOp, ProviderOutcome, observe_provider},
    config,
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
};

#[derive(Clone)]
pub struct KrakenProvider {
    http: reqwest::Client,
    cmc_api_key: Option<String>,
    kraken_circuit_breaker: DefaultCircuitBreaker,
    cmc_circuit_breaker: DefaultCircuitBreaker,
}

impl KrakenProvider {
    pub fn new(http: reqwest::Client, config: &config::Providers) -> Self {
        let kraken_circuit_breaker = failsafe::Config::new().build();
        let cmc_circuit_breaker = failsafe::Config::new().build();

        Self {
            http,
            cmc_api_key: config.cmc_api_key.clone(),
            kraken_circuit_breaker,
            cmc_circuit_breaker,
        }
    }

    /// Discovers new Kraken markets and their assets. Recorded as one `lookup`
    /// provider call covering the Kraken and CoinMarketCap requests it makes.
    pub async fn fetch_assets(
        &self,
        repo: &MarketDataRepository,
    ) -> Result<(Vec<Asset>, Vec<Market>)> {
        observe_provider(
            MetricProvider::Kraken,
            ProviderOp::Lookup,
            self.fetch_assets_inner(repo),
            outcome_of_result,
        )
        .await
    }

    async fn fetch_assets_inner(
        &self,
        repo: &MarketDataRepository,
    ) -> Result<(Vec<Asset>, Vec<Market>)> {
        static URL: &str = "https://api.kraken.com/0/public/AssetPairs";

        // Fetch Kraken markets
        debug!(url = URL, "Fetching markets from Kraken");
        let market_symbols = self.fetch_online_market_symbols(URL).await?;

        let (markets, assets) = if self.cmc_api_key.is_some() {
            // If CoinMarketCap API key is available, enrich assets data with human-friendly
            // info
            self.resolve_assets_data(&market_symbols, repo).await
        } else {
            // If not, `Asset`s will have same `id` and `symbol`. No big deal, just less
            // fancy
            resolve_assets_data_kraken_only(&market_symbols, repo).await
        };

        debug!("New assets: {}", serde_json::to_string(&assets).unwrap());
        debug!("New markets: {}", serde_json::to_string(&markets).unwrap());

        Ok((assets, markets))
    }

    /// Fetches the complete set of currently online Kraken market ids.
    /// Recorded as one `lookup` provider call.
    pub async fn fetch_tradable_market_ids(&self) -> Result<HashSet<MarketId>> {
        observe_provider(
            MetricProvider::Kraken,
            ProviderOp::Lookup,
            self.fetch_tradable_market_ids_inner(),
            outcome_of_result,
        )
        .await
    }

    async fn fetch_tradable_market_ids_inner(&self) -> Result<HashSet<MarketId>> {
        static URL: &str = "https://api.kraken.com/0/public/AssetPairs";

        debug!(url = URL, "Fetching market snapshot from Kraken");
        let market_symbols = self.fetch_online_market_symbols(URL).await?;
        market_ids_from_symbols(&market_symbols)
    }

    async fn fetch_online_market_symbols(&self, url: &str) -> Result<Vec<String>> {
        let Some(res) = self.fetch_kraken_api::<AssetPairsResponse>(url).await? else {
            return Err(DcaError::Generic("AssetPairs not found".to_string()));
        };

        if !res.error.is_empty() {
            error!("Error occurred while fetching asset pairs: {:?}", res.error);
            return Err(DcaError::Generic(format!("{:?}", res.error)));
        }

        parse_online_market_symbols(&res)
    }

    /// Fetches the latest price of a market, falling back from 5-minute to daily
    /// candles. Recorded as one `price` provider call (both attempts included).
    pub async fn fetch_market_price(&self, mkt: &Market, ts: DateTime) -> Result<Option<f64>> {
        observe_provider(
            MetricProvider::Kraken,
            ProviderOp::Price,
            self.fetch_market_price_inner(mkt, ts),
            |result| match result {
                Ok(Some(_)) => ProviderOutcome::Ok,
                Ok(None) => ProviderOutcome::NotFound,
                Err(e) => outcome_of_error(e),
            },
        )
        .await
    }

    async fn fetch_market_price_inner(&self, mkt: &Market, ts: DateTime) -> Result<Option<f64>> {
        if let Some(px) = self
            .fetch_price(&mkt.id, OHLCFrequency::Minutes5, ts)
            .await?
        {
            return Ok(Some(px));
        }

        if let Some(px) = self.fetch_price(&mkt.id, OHLCFrequency::Daily, ts).await? {
            return Ok(Some(px));
        }

        Ok(None)
    }

    async fn fetch_price(
        &self,
        id: &MarketId,
        freq: OHLCFrequency,
        ts: DateTime,
    ) -> Result<Option<f64>> {
        let (r_lo, r_hi) = freq.ohlc_range(ts);
        let (after_ts, before_ts) = (r_lo.timestamp(), r_hi.timestamp());
        let periods = get_kraken_api_periods(freq);
        let url = format!(
            "https://api.kraken.com/0/public/OHLC?pair={id}&since={after_ts}&interval={periods}"
        );

        debug!(
            url = url,
            "Fetching {freq} OHLC candlestick for market '{id}' since {r_lo}"
        );

        let Some(mut res) = self.fetch_kraken_api::<OHLCResult>(&url).await? else {
            return Ok(None);
        };

        if !res.error.is_empty() {
            error!("Error occurred while fetching '{id}': {:?}", res.error);
            return Err(DcaError::Generic(format!("{:?}", res.error)));
        }

        let Some(ref mut result) = res.result else {
            return Err(DcaError::Generic(
                "Unexpected empty 'result' field".to_string(),
            ));
        };

        if result.is_empty() || result.len() < 2 {
            return Err(DcaError::Generic(format!(
                "Malformed response. Unexpected empty result: {res:?}"
            )));
        }

        result.remove("last"); // Drop fucking weird 'last' entry
        let Payload::CandleSticks(csticks) = result.values().next().unwrap() else {
            return Err(DcaError::Generic(format!(
                "Malformed response. Cannot find candlesticks: {res:?}"
            )));
        };

        if csticks.0.is_empty() {
            return Err(DcaError::Generic(format!(
                "Malformed response. Unexpected empty result: {res:?}"
            )));
        }

        // Get most recent candlestick before `before_ts` or most recent
        let cstick = csticks
            .0
            .iter()
            .rev()
            .find(|c| c[0].as_i64().unwrap_or(i64::MAX) <= before_ts)
            .unwrap_or_else(|| csticks.0.iter().last().unwrap());

        let Some(close_px) = cstick[4].as_f64() else {
            return Err(DcaError::Generic(format!(
                "Cannot parse '{:?}' into f64",
                cstick[4]
            )));
        };

        Ok(Some(close_px))
    }

    async fn resolve_assets_data(
        &self,
        market_symbols: &[String],
        repo: &MarketDataRepository,
    ) -> (Vec<Market>, Vec<Asset>) {
        let pairs = get_new_markets(market_symbols, repo).await;
        let ccys = get_unique_ccys(&pairs);
        let assets_map = self.fetch_assets_cmc(&ccys, repo).await;

        let markets = pairs
            .into_iter()
            .filter_map(|(base_id, quote_id)| {
                let Some(base) = assets_map.get(&base_id) else {
                    error!("Skipping market {base_id}{quote_id}: base asset data not found");
                    return None;
                };

                let Some(quote) = assets_map.get(&quote_id) else {
                    error!("Skipping market {base_id}{quote_id}: quote asset data not found");
                    return None;
                };

                Some(Market::new(
                    format!("{base_id}{quote_id}"),
                    base.clone(),
                    quote.clone(),
                    None,
                ))
            })
            .collect::<Vec<_>>();

        let assets = assets_map.into_values().collect();

        (markets, assets)
    }

    async fn fetch_assets_cmc(
        &self,
        ccys: &HashSet<AssetId>,
        repo: &MarketDataRepository,
    ) -> BTreeMap<String, Asset> {
        let mut assets = BTreeMap::new();
        let mut to_fetch = Vec::new();
        for ccy in ccys {
            match repo.find_asset(ccy).await {
                Ok(asset) => {
                    if let Some(a) = asset {
                        assets.insert(a.id().clone(), a);
                    } else if fiat::is_fiat(ccy) {
                        assets.insert(
                            ccy.clone(),
                            Asset::Fiat(Fiat::new(ccy.clone(), fiat::get_name(ccy).to_string())),
                        );
                    } else {
                        to_fetch.push(ccy);
                    }
                }
                Err(e) => {
                    error!("Failed to find asset '{ccy}': {e}");
                    continue;
                }
            }
        }

        static CHUNK_SIZE: usize = 100;
        let mut start = 0;
        let mut end = CHUNK_SIZE.min(to_fetch.len());

        let next_chunk = |start: &mut usize, end: &mut usize| {
            *start += CHUNK_SIZE;
            *end = (*end + CHUNK_SIZE).min(to_fetch.len());
        };

        while start < end {
            let chunk = &to_fetch[start..end];
            let fetched = self.fetch_crypto_cmc(chunk).await;

            if let Err(e) = fetched {
                warn!("Failed to fetch CMC assets. Filling with default ({chunk:?}): {e:?}");

                for id in chunk {
                    assets.insert(
                        id.to_string(),
                        Asset::Crypto(Crypto::new_with_id(id.to_string())),
                    );
                }

                next_chunk(&mut start, &mut end);
                continue;
            }

            fetched.unwrap().into_iter().for_each(|a| {
                assets.insert(a.id().clone(), a);
            });

            for id in chunk {
                if !assets.contains_key(*id) {
                    warn!("Cannot find CMC asset '{id}' in `assets` map. Filling with default");

                    assets.insert(
                        id.to_string(),
                        Asset::Crypto(Crypto::new_with_id(id.to_string())),
                    );
                }
            }

            next_chunk(&mut start, &mut end);
        }

        assets
    }

    async fn fetch_crypto_cmc(&self, ccys: &[&AssetId]) -> Result<Vec<Asset>> {
        lazy_static! {
            static ref CMC_ALIAS: HashMap<&'static str, &'static str> = HashMap::from([
                ("eth2.s", "eth"),
                ("repv2", "rep"),
                ("luna2", "luna"),
                ("waxl", "axl")
            ]);
        }

        // Skip currencies not known by CMC for now. Better to fallback than send
        // confusing asset names
        let symbols = ccys
            .iter()
            .filter_map(|c| (!CMC_ALIAS.contains_key(c.as_str())).then_some(c.as_str()))
            .join(",");

        let url =
            format!("https://pro-api.coinmarketcap.com/v2/cryptocurrency/info?symbol={symbols}");

        debug!(url = url, "Fetching assets data from CMC");

        let Some(res) = self.fetch_cmc_api::<CMCInfoResult>(&url).await? else {
            return Ok(vec![]);
        };

        if let Some(msg) = res.status.error_message {
            error!("Error occurred: {:?}", msg);
            return Err(DcaError::Generic(format!("{msg:?}")));
        }

        let assets = res
            .data
            .into_values()
            .map(|mut d| d.swap_remove(0))
            .map(|d| {
                Asset::Crypto(Crypto {
                    id: d.symbol.to_lowercase(),
                    symbol: d.name,
                })
            })
            .collect_vec();

        Ok(assets)
    }

    async fn fetch_kraken_api<T: DeserializeOwned + Debug>(&self, url: &str) -> Result<Option<T>> {
        self.kraken_circuit_breaker
            .call(self.fetch_kraken_api_inner::<T>(url))
            .await
            .map_err(|e| DcaError::from_failsafe(e, "KrakenProvider"))
    }

    async fn fetch_kraken_api_inner<T: DeserializeOwned + Debug>(
        &self,
        url: &str,
    ) -> Result<Option<T>> {
        let res = self.http.get(url).send().await?;

        if res.status().is_success() {
            Ok(Some(res.json::<T>().await?))
        } else if let StatusCode::NOT_FOUND = res.status() {
            Ok(None)
        } else {
            Err(res.error_for_status().unwrap_err().into())
        }
    }

    async fn fetch_cmc_api<T: DeserializeOwned + Debug>(&self, url: &str) -> Result<Option<T>> {
        self.cmc_circuit_breaker
            .call(self.fetch_cmc_api_inner::<T>(url))
            .await
            .map_err(|e| DcaError::from_failsafe(e, "CoinMarketCapProvider"))
    }

    async fn fetch_cmc_api_inner<T: DeserializeOwned + Debug>(
        &self,
        url: &str,
    ) -> Result<Option<T>> {
        let res = self
            .http
            .get(url)
            .header("X-CMC_PRO_API_KEY", self.cmc_api_key.as_ref().unwrap())
            .send()
            .await?;

        if res.status().is_success() {
            Ok(Some(res.json::<T>().await?))
        } else if let StatusCode::NOT_FOUND = res.status() {
            Ok(None)
        } else {
            Err(res.error_for_status().unwrap_err().into())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Pair {
    wsname: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetPairsResponse {
    error: Vec<String>,
    result: HashMap<String, Pair>,
}

/// Classifies a failed Kraken call for the provider metrics.
fn outcome_of_error(error: &DcaError) -> ProviderOutcome {
    match error {
        DcaError::Reqwest(e) if e.is_timeout() => ProviderOutcome::Timeout,
        _ => ProviderOutcome::Error,
    }
}

/// Classifies a call that either succeeds or fails, with no "not found" case.
fn outcome_of_result<T>(result: &Result<T>) -> ProviderOutcome {
    match result {
        Ok(_) => ProviderOutcome::Ok,
        Err(e) => outcome_of_error(e),
    }
}

async fn resolve_assets_data_kraken_only(
    market_symbols: &[String],
    repo: &MarketDataRepository,
) -> (Vec<Market>, Vec<Asset>) {
    let pairs = get_new_markets(market_symbols, repo).await;
    let ccys = get_unique_ccys(&pairs);
    let assets_map = fetch_assets(&ccys, repo).await;

    let markets = pairs
        .into_iter()
        .filter_map(|(base_id, quote_id)| {
            let Some(base) = assets_map.get(&base_id) else {
                error!("Skipping market {base_id}{quote_id}: base asset data not found");
                return None;
            };

            let Some(quote) = assets_map.get(&quote_id) else {
                error!("Skipping market {base_id}{quote_id}: quote asset data not found");
                return None;
            };

            Some(Market::new(
                format!("{base_id}{quote_id}"),
                base.clone(),
                quote.clone(),
                None,
            ))
        })
        .collect::<Vec<_>>();

    let assets = assets_map.into_values().collect();

    (markets, assets)
}

fn normalize_symbol(wsname: &str) -> String {
    wsname
        .replace("XBT/", "BTC/")
        .replace("/XBT", "/BTC")
        .to_lowercase()
}

#[derive(Debug, PartialEq, Eq)]
struct MarketSymbol<'a> {
    base: &'a str,
    quote: &'a str,
    venue: Option<&'a str>,
}

/// Parses a Kraken websocket name into base, quote, and an optional venue.
fn parse_market_symbol(symbol: &str) -> Option<MarketSymbol<'_>> {
    let (base, quote_and_venue) = symbol.split_once('/')?;
    let (quote, venue) = match quote_and_venue.split_once(':') {
        Some((quote, venue)) => (quote, Some(venue)),
        None => (quote_and_venue, None),
    };

    let valid_component =
        |component: &str| !component.is_empty() && !component.chars().any(char::is_whitespace);

    if !valid_component(base)
        || base.contains(':')
        || !valid_component(quote)
        || quote.contains('/')
        || venue.is_some_and(|venue| {
            !valid_component(venue) || venue.contains('/') || venue.contains(':')
        })
    {
        return None;
    }

    Some(MarketSymbol { base, quote, venue })
}

fn parse_online_market_symbols(response: &AssetPairsResponse) -> Result<Vec<String>> {
    let symbols = response
        .result
        .values()
        .filter(|pair| pair.status == "online")
        .filter_map(|pair| {
            let symbol = normalize_symbol(&pair.wsname);
            let Some(parsed) = parse_market_symbol(&symbol) else {
                warn!(symbol, "Ignoring malformed online Kraken market");
                return None;
            };
            Some(format!("{}/{}", parsed.base, parsed.quote))
        })
        .collect::<Vec<_>>();

    if symbols.is_empty() {
        return Err(DcaError::Generic(
            "Kraken returned no online markets".to_string(),
        ));
    }

    Ok(symbols)
}

fn market_ids_from_symbols(market_symbols: &[String]) -> Result<HashSet<MarketId>> {
    let ids = market_symbols
        .iter()
        .filter_map(|symbol| {
            let Some(parsed) = parse_market_symbol(symbol) else {
                warn!(symbol, "Ignoring malformed online Kraken market");
                return None;
            };
            Some(format!("{}{}", parsed.base, parsed.quote))
        })
        .collect::<HashSet<_>>();

    if ids.is_empty() {
        return Err(DcaError::Generic(
            "Kraken returned no online markets".to_string(),
        ));
    }

    Ok(ids)
}

type MarketPair = (String, String);

fn split_base_quote(market_symbols: &[String]) -> Vec<MarketPair> {
    market_symbols
        .iter()
        .filter_map(|id| match id.split_once('/') {
            Some((base_id, quote_id)) => Some((base_id.to_string(), quote_id.to_string())),
            None => {
                error!("Malformed market id: {id}");
                None
            }
        })
        .collect()
}

async fn get_new_markets(
    market_symbols: &[String],
    repo: &MarketDataRepository,
) -> Vec<MarketPair> {
    futures::stream::iter(split_base_quote(market_symbols))
        .filter(|(base, quote)| is_new_market(format!("{base}{quote}"), repo))
        .collect::<_>()
        .await
}

fn get_unique_ccys(market_pairs: &[MarketPair]) -> HashSet<AssetId> {
    market_pairs
        .iter()
        .flat_map(|(base, quote)| [base.clone(), quote.clone()])
        .collect::<_>()
}

async fn fetch_assets(
    ccys: &HashSet<AssetId>,
    repo: &MarketDataRepository,
) -> BTreeMap<String, Asset> {
    let mut assets = BTreeMap::new();
    for ccy in ccys {
        match repo.find_asset(ccy).await {
            Ok(asset) => {
                if let Some(a) = asset {
                    assets.insert(a.id().clone(), a);
                } else {
                    let a = match fiat::is_fiat(ccy) {
                        true => Asset::Fiat(Fiat {
                            id: ccy.to_string(),
                            symbol: fiat::get_name(ccy).to_string(),
                        }),
                        false => Asset::Crypto(Crypto::new_with_id(ccy.clone())),
                    };
                    assets.insert(ccy.clone(), a);
                }
            }
            Err(e) => {
                error!("Failed to find asset '{ccy}': {e}");
                continue;
            }
        }
    }
    assets
}

mod fiat {
    use std::collections::HashMap;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref FIAT_IDS: HashMap<&'static str, &'static str> = HashMap::from([
            ("aed", "United Arab Emirates Dirham"),
            ("aud", "Australian Dollar"),
            ("cad", "Canadian Dollar"),
            ("chf", "Swiss franc"),
            ("eur", "Euro"),
            ("gbp", "British Pound"),
            ("jpy", "Japanese Yen"),
            ("usd", "United States Dollar")
        ]);
    }

    pub fn is_fiat(id: &str) -> bool {
        FIAT_IDS.contains_key(id)
    }

    pub fn get_name(id: &str) -> &'static str {
        FIAT_IDS.get(id).unwrap()
    }
}

async fn is_new_market(id: MarketId, repo: &MarketDataRepository) -> bool {
    let mkt = repo.find_market(&id).await.unwrap_or_else(|e| {
        error!("Error occurred in searching market '{}': {}", id, e);
        None
    });

    mkt.is_none()
}

fn get_kraken_api_periods(freq: OHLCFrequency) -> &'static str {
    match freq {
        OHLCFrequency::Minutes5 => "5",
        OHLCFrequency::Daily => "1440",
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OHLCResult {
    error: Vec<String>,
    result: Option<HashMap<String, Payload>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Payload {
    #[allow(dead_code)]
    Last(i64),
    CandleSticks(CandleSticks),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
struct CandleSticks(Vec<[IntOrStr; 8]>);

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum IntOrStr {
    Int(i64),
    Str(String),
}

impl IntOrStr {
    pub fn as_i64(&self) -> Option<i64> {
        if let IntOrStr::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let IntOrStr::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_str().and_then(|s| s.parse::<f64>().ok())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CMCInfoResult {
    status: CMCInfoStatus,
    data: HashMap<String, Vec<CMCInfoData>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CMCInfoStatus {
    error_code: u32,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CMCInfoData {
    id: u64,
    symbol: String,
    name: String,
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use crate::{
        app::domain::entity::{Asset, Fiat, Market},
        ports::outbound::repository::market_data::MarketDataRepository,
    };

    use super::{
        AssetPairsResponse, MarketSymbol, Pair, fetch_assets, get_new_markets,
        market_ids_from_symbols, parse_market_symbol, parse_online_market_symbols,
    };

    #[sqlx::test(migrations = "../../migrations")]
    async fn discovery_selects_a_market_whose_base_asset_is_missing(pool: sqlx::PgPool) {
        // GIVEN only the USD quote asset is in the catalog and PEARL/USD has no market
        let repo = MarketDataRepository::new(pool);
        let quote = Asset::Fiat(Fiat::new("usd".to_string(), "US Dollar".to_string()));
        repo.store_asset(&quote)
            .await
            .expect("USD should be stored");

        // WHEN the current Kraken snapshot is reconciled and missing currencies are loaded
        let pairs = get_new_markets(&["pearl/usd".to_string()], &repo).await;
        let assets = fetch_assets(&super::get_unique_ccys(&pairs), &repo).await;

        // THEN the missing market is selected and its missing base is reconstructed
        assert_eq!(pairs, vec![("pearl".to_string(), "usd".to_string())]);
        assert_eq!(
            assets.get("pearl").map(Asset::id).map(String::as_str),
            Some("pearl")
        );

        // AND storing the discovered assets and market leaves the catalog consistent
        for asset in assets.values() {
            repo.store_asset(asset)
                .await
                .expect("discovered asset should be stored");
        }
        let market = Market::new(
            "pearlusd".to_string(),
            assets.get("pearl").unwrap().clone(),
            assets.get("usd").unwrap().clone(),
            None,
        );
        repo.store_market(&market, crate::config::PriceProvider::Kraken)
            .await
            .expect("market should be stored");
        assert!(
            repo.find_market(&"pearlusd".to_string())
                .await
                .expect("market should resolve")
                .is_some()
        );
    }

    #[test]
    fn online_pair_snapshot_is_normalized_and_filters_offline_pairs() {
        // GIVEN Kraken returns online pairs in an arbitrary map order, including an XBT pair
        let response = AssetPairsResponse {
            error: vec![],
            result: HashMap::from([
                (
                    "xbt-usd".to_string(),
                    Pair {
                        wsname: "XBT/EUR".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "eth-eur".to_string(),
                    Pair {
                        wsname: "ETH/EUR:darkpool".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "ada-usd".to_string(),
                    Pair {
                        wsname: "ADA/USD".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "old".to_string(),
                    Pair {
                        wsname: "BTC/GBP".to_string(),
                        status: "offline".to_string(),
                    },
                ),
            ]),
        };

        // WHEN the online pairs are parsed into a snapshot
        let symbols = parse_online_market_symbols(&response).unwrap();
        let ids = market_ids_from_symbols(&symbols).unwrap();

        // THEN offline pairs are excluded and the result is normalized
        assert_eq!(
            symbols.into_iter().collect::<HashSet<_>>(),
            ["btc/eur", "eth/eur", "ada/usd"]
                .into_iter()
                .map(String::from)
                .collect()
        );
        assert_eq!(
            ids,
            [
                "btceur".to_string(),
                "etheur".to_string(),
                "adausd".to_string()
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn market_symbol_parser_retains_optional_venue_separately() {
        // GIVEN a normalized Kraken symbol with an optional venue
        // WHEN the symbol is parsed
        let parsed = parse_market_symbol("btc/eur:darkpool");

        // THEN the base, quote, and venue are separated
        assert_eq!(
            parsed,
            Some(MarketSymbol {
                base: "btc",
                quote: "eur",
                venue: Some("darkpool")
            })
        );
    }

    #[test]
    fn malformed_online_pair_is_ignored() {
        // GIVEN Kraken returns one valid pair and malformed online pairs
        let response = AssetPairsResponse {
            error: vec![],
            result: HashMap::from([
                (
                    "valid".to_string(),
                    Pair {
                        wsname: "BTC/EUR".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "broken".to_string(),
                    Pair {
                        wsname: "BROKEN".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "empty-venue".to_string(),
                    Pair {
                        wsname: "ETH/EUR:".to_string(),
                        status: "online".to_string(),
                    },
                ),
                (
                    "multiple-venues".to_string(),
                    Pair {
                        wsname: "ETH/EUR:SPOT:EXTRA".to_string(),
                        status: "online".to_string(),
                    },
                ),
            ]),
        };

        // WHEN the online pairs are parsed
        let symbols = parse_online_market_symbols(&response).unwrap();

        // THEN the malformed pair is ignored and the valid pair is retained
        assert_eq!(symbols, ["btc/eur"]);
    }

    #[test]
    fn snapshot_without_usable_online_pairs_is_rejected() {
        // GIVEN Kraken returns only offline or malformed pairs
        let response = AssetPairsResponse {
            error: vec![],
            result: HashMap::from([
                (
                    "offline".to_string(),
                    Pair {
                        wsname: "BTC/EUR".to_string(),
                        status: "offline".to_string(),
                    },
                ),
                (
                    "broken".to_string(),
                    Pair {
                        wsname: "BROKEN".to_string(),
                        status: "online".to_string(),
                    },
                ),
            ]),
        };

        // WHEN the online snapshot is parsed
        let result = parse_online_market_symbols(&response);

        // THEN the empty snapshot is rejected so it cannot delete every market
        assert!(result.is_err());
    }
}
