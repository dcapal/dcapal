use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    sync::LazyLock,
};

use failsafe::futures::CircuitBreaker;
use itertools::Itertools;
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use tracing::{debug, error, warn};

use super::{DefaultCircuitBreaker, PriceCallError};
use crate::{
    DateTime,
    app::domain::{
        entity::{Asset, AssetId, Crypto, Fiat, Market, MarketId, OHLCFrequency},
        fiat::{fiat_name, is_fiat},
        market_quote::{FailureKind, PriceFailure},
    },
    app::infra::stats::{MetricProvider, ProviderOp, ProviderOutcome, observe_provider},
    config,
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
};

/// Root URL of the public Kraken API.
const DEFAULT_BASE_URL: &str = "https://api.kraken.com";

#[derive(Clone)]
pub struct KrakenProvider {
    http: reqwest::Client,
    /// Root URL of the Kraken public API, without a trailing slash.
    base_url: String,
    cmc_api_key: Option<String>,
    kraken_circuit_breaker: DefaultCircuitBreaker,
    cmc_circuit_breaker: DefaultCircuitBreaker,
}

impl KrakenProvider {
    pub fn new(http: reqwest::Client, config: &config::Providers) -> Self {
        let kraken_circuit_breaker = failsafe::Config::new().build();
        let cmc_circuit_breaker = failsafe::Config::new().build();

        let base_url = config
            .kraken_base_url
            .as_deref()
            .unwrap_or(DEFAULT_BASE_URL)
            .trim_end_matches('/')
            .to_string();

        Self {
            http,
            base_url,
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
        let url = self.asset_pairs_url();

        // Fetch Kraken markets
        debug!(url = url, "Fetching markets from Kraken");
        let market_symbols = self.fetch_online_market_symbols(&url).await?;

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
        let url = self.asset_pairs_url();

        debug!(url = url, "Fetching market snapshot from Kraken");
        let market_symbols = self.fetch_online_market_symbols(&url).await?;
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

    /// URL listing every Kraken asset pair.
    fn asset_pairs_url(&self) -> String {
        format!("{}/0/public/AssetPairs", self.base_url)
    }

    /// Builds the OHLC URL of a pair, percent-encoding the pair as a query value.
    fn ohlc_url(
        &self,
        pair: &str,
        since: i64,
        interval: &str,
    ) -> std::result::Result<url::Url, String> {
        let mut url = url::Url::parse(&format!("{}/0/public/OHLC", self.base_url))
            .map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("pair", pair)
            .append_pair("since", &since.to_string())
            .append_pair("interval", interval);
        Ok(url)
    }

    /// Fetches the latest price of a market, falling back from 5-minute to daily
    /// candles. Recorded as one `price` provider call (both attempts included).
    ///
    /// Only Kraken is asked: a failure is reported, never answered from another
    /// provider. The daily fallback only follows a 5-minute answer without a price.
    pub async fn fetch_market_price(
        &self,
        mkt: &Market,
        ts: DateTime,
    ) -> std::result::Result<f64, PriceFailure> {
        observe_provider(
            MetricProvider::Kraken,
            ProviderOp::Price,
            self.fetch_market_price_inner(mkt, ts),
            PriceCallError::outcome,
        )
        .await
        .map_err(|e| e.failure)
    }

    async fn fetch_market_price_inner(
        &self,
        mkt: &Market,
        ts: DateTime,
    ) -> std::result::Result<f64, PriceCallError> {
        match self.fetch_price(&mkt.id, OHLCFrequency::Minutes5, ts).await {
            Err(e) if e.failure.kind == FailureKind::Unavailable => {
                self.fetch_price(&mkt.id, OHLCFrequency::Daily, ts).await
            }
            other => other,
        }
    }

    async fn fetch_price(
        &self,
        id: &MarketId,
        freq: OHLCFrequency,
        ts: DateTime,
    ) -> std::result::Result<f64, PriceCallError> {
        let (r_lo, r_hi) = freq.ohlc_range(ts);
        let (after_ts, before_ts) = (r_lo.as_second(), r_hi.as_second());
        let url = self
            .ohlc_url(id, after_ts, get_kraken_api_periods(freq))
            .map_err(PriceFailure::malformed)?;

        debug!(
            url = url.as_str(),
            "Fetching {freq} OHLC candlestick for market '{id}' since {r_lo}"
        );

        let fetched = self.fetch_kraken_api::<OHLCResult>(url.as_str()).await;
        let res = match fetched {
            Ok(Some(res)) => res,
            Ok(None) => {
                return Err(PriceFailure::unavailable("Kraken does not list the market").into());
            }
            Err(e) => return Err(call_error_of(&e)),
        };

        Ok(parse_ohlc_price(res, before_ts)?)
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

    /// Resolves currencies with their catalog or CoinMarketCap metadata.
    ///
    /// Classified like [`fetch_assets`]: catalog fiat is fiat, everything else
    /// (including non-fiat quotes of cross-crypto pairs) is Kraken crypto.
    async fn fetch_assets_cmc(
        &self,
        ccys: &HashSet<AssetId>,
        repo: &MarketDataRepository,
    ) -> BTreeMap<String, Asset> {
        let mut assets = BTreeMap::new();
        let mut to_fetch = Vec::new();
        let ccys = ccys.iter().collect::<Vec<_>>();
        let found = match repo.find_assets(&ccys).await {
            Ok(found) => found,
            Err(e) => {
                error!("Failed to find {} assets: {e}", ccys.len());
                return assets;
            }
        };
        for (ccy, asset) in ccys.into_iter().zip(found) {
            if let Some(a) = asset {
                assets.insert(a.id().clone(), a);
            } else if is_fiat(ccy) {
                assets.insert(
                    ccy.clone(),
                    Asset::Fiat(Fiat::new(
                        ccy.clone(),
                        fiat_name(ccy).unwrap_or(ccy).to_string(),
                    )),
                );
            } else {
                to_fetch.push(ccy);
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
        static CMC_ALIAS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
            HashMap::from([
                ("eth2.s", "eth"),
                ("repv2", "rep"),
                ("luna2", "luna"),
                ("waxl", "axl"),
            ])
        });

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

/// Describes a failed Kraken request as a visible failure: undecodable replies are
/// malformed, everything else (transport, status, open circuit) is a provider error.
fn call_error_of(error: &DcaError) -> PriceCallError {
    match error {
        DcaError::Reqwest(e) if e.is_timeout() => {
            PriceCallError::timeout("Kraken request timed out")
        }
        DcaError::Reqwest(e) if e.is_decode() => {
            PriceFailure::malformed(format!("undecodable Kraken reply: {e}")).into()
        }
        other => PriceFailure::provider_error(format!("Kraken request failed: {other:?}")).into(),
    }
}

/// Extracts the close of the most recent candle at or before `before_ts` from an OHLC reply.
///
/// Kraken's `EQuery:Unknown asset pair` means the market is not listed (unavailable);
/// any other reported error is a provider error. A reply without candles is
/// unavailable; a reply whose structure or close cannot be read is malformed.
fn parse_ohlc_price(mut res: OHLCResult, before_ts: i64) -> std::result::Result<f64, PriceFailure> {
    if !res.error.is_empty() {
        let unknown_pair = res
            .error
            .iter()
            .any(|e| e.starts_with("EQuery:Unknown asset pair"));
        return Err(if unknown_pair {
            PriceFailure::unavailable("Kraken does not list the market")
        } else {
            PriceFailure::provider_error(format!("Kraken reported {:?}", res.error))
        });
    }

    let Some(ref mut result) = res.result else {
        return Err(PriceFailure::malformed("unexpected empty 'result' field"));
    };

    // The reply carries one candle list next to a numeric `last` marker.
    result.remove("last");
    let Some(Payload::CandleSticks(csticks)) = result.values().next() else {
        return Err(PriceFailure::malformed("cannot find candlesticks"));
    };
    let Some(newest) = csticks.0.last() else {
        return Err(PriceFailure::unavailable("Kraken returned no candles"));
    };

    // Most recent candlestick before `before_ts`, or the most recent one.
    let cstick = csticks
        .0
        .iter()
        .rev()
        .find(|c| c[0].as_i64().unwrap_or(i64::MAX) <= before_ts)
        .unwrap_or(newest);

    cstick[4]
        .as_f64()
        .ok_or_else(|| PriceFailure::malformed(format!("cannot parse close '{:?}'", cstick[4])))
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
            // Fiat against fiat pairs are priced by Yahoo, not discovered from Kraken.
            if is_fiat(parsed.base) && is_fiat(parsed.quote) {
                return None;
            }
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

/// Returns the pairs that are not stored as markets yet, found with one lookup.
///
/// When the lookup fails every pair is considered new, so discovery stores them
/// rather than skipping markets.
async fn get_new_markets(
    market_symbols: &[String],
    repo: &MarketDataRepository,
) -> Vec<MarketPair> {
    let pairs = split_base_quote(market_symbols);
    let ids = pairs
        .iter()
        .map(|(base, quote)| format!("{base}{quote}"))
        .collect::<Vec<_>>();

    match repo.find_markets(&ids).await {
        Ok(found) => pairs
            .into_iter()
            .zip(found)
            .filter_map(|(pair, market)| market.is_none().then_some(pair))
            .collect(),
        Err(e) => {
            error!("Error occurred in searching {} markets: {e}", ids.len());
            pairs
        }
    }
}

fn get_unique_ccys(market_pairs: &[MarketPair]) -> HashSet<AssetId> {
    market_pairs
        .iter()
        .flat_map(|(base, quote)| [base.clone(), quote.clone()])
        .collect::<_>()
}

/// Resolves the currencies of discovered Kraken pairs into assets.
///
/// Only the eight catalog fiat currencies are classified as fiat; every other
/// currency is a Kraken crypto asset. This includes the non-fiat quotes of
/// Kraken pairs (`btc` in `ethbtc`, stablecoins such as `usdt`, and currencies
/// such as `zar`, `mxn` or `try`), so those pairs are crypto against crypto
/// markets: both assets are stored as Kraken/Crypto and the market is priced by
/// Kraken, never by Yahoo, which only prices fiat against fiat.
async fn fetch_assets(
    ccys: &HashSet<AssetId>,
    repo: &MarketDataRepository,
) -> BTreeMap<String, Asset> {
    let mut assets = BTreeMap::new();
    let ccys = ccys.iter().collect::<Vec<_>>();
    let found = match repo.find_assets(&ccys).await {
        Ok(found) => found,
        Err(e) => {
            error!("Failed to find {} assets: {e}", ccys.len());
            return assets;
        }
    };

    for (ccy, asset) in ccys.into_iter().zip(found) {
        if let Some(a) = asset {
            assets.insert(a.id().clone(), a);
        } else {
            let a = match is_fiat(ccy) {
                true => Asset::Fiat(Fiat {
                    id: ccy.to_string(),
                    symbol: fiat_name(ccy).unwrap_or(ccy).to_string(),
                }),
                false => Asset::Crypto(Crypto::new_with_id(ccy.clone())),
            };
            assets.insert(ccy.clone(), a);
        }
    }
    assets
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
        app::domain::{
            entity::{Asset, Fiat, Market},
            market_data_utils::price_provider_for,
        },
        config::PriceProvider,
        ports::outbound::repository::market_data::MarketDataRepository,
    };

    use super::{
        AssetPairsResponse, MarketSymbol, Pair, fetch_assets, get_new_markets,
        market_ids_from_symbols, parse_market_symbol, parse_online_market_symbols,
    };

    #[sqlx::test(migrations = "../../migrations")]
    async fn cross_crypto_pairs_are_crypto_markets_priced_by_kraken(pool: sqlx::PgPool) {
        // GIVEN Kraken lists the cross-crypto pair ETH/BTC, whose quote is not fiat,
        let repo = MarketDataRepository::new(pool);

        // WHEN its currencies are classified and the market is built,
        let pairs = get_new_markets(&["eth/btc".to_string()], &repo).await;
        let assets = fetch_assets(&super::get_unique_ccys(&pairs), &repo).await;
        let market = Market::new(
            "ethbtc".to_string(),
            assets["eth"].clone(),
            assets["btc"].clone(),
            None,
        );

        // THEN both assets are crypto, the market is not an FX market, and Kraken
        // (the configured crypto provider) prices it rather than Yahoo.
        assert!(matches!(assets["eth"], Asset::Crypto(_)));
        assert!(matches!(assets["btc"], Asset::Crypto(_)));
        assert!(!market.is_fiat());
        assert!(matches!(
            price_provider_for(&market, PriceProvider::Kraken),
            PriceProvider::Kraken
        ));
    }

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
    fn fiat_against_fiat_pairs_are_not_discovered_from_kraken() {
        // GIVEN Kraken lists a crypto pair and fiat against fiat pairs,
        // WHEN the online pairs are parsed,
        // THEN only the crypto pair is kept, because Yahoo prices FX.
        let pair = |wsname: &str| Pair {
            wsname: wsname.to_string(),
            status: "online".to_string(),
        };
        let response = AssetPairsResponse {
            error: vec![],
            result: HashMap::from([
                ("a".to_string(), pair("EUR/USD")),
                ("b".to_string(), pair("USD/JPY")),
                ("c".to_string(), pair("BTC/EUR")),
            ]),
        };

        let symbols = parse_online_market_symbols(&response).unwrap();

        assert_eq!(symbols, ["btc/eur"]);
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

#[cfg(test)]
mod price_tests {
    use super::*;
    use crate::{
        app::domain::entity::{Crypto, Fiat},
        test_support::{FakeProvider, Reply},
    };

    fn providers_config(base: &str) -> config::Providers {
        serde_json::from_value(serde_json::json!({
            "priceProvider": "kraken",
            "cwApiKey": "",
            "ipApiKey": "",
            "krakenBaseUrl": base,
        }))
        .unwrap()
    }

    /// A fresh provider per test, so circuit-breaker state never leaks between tests.
    fn provider_at(fake: &FakeProvider) -> KrakenProvider {
        crate::test_support::install_crypto_provider();
        KrakenProvider::new(reqwest::Client::new(), &providers_config(&fake.base_url()))
    }

    fn btceur() -> Market {
        Market::new(
            "btceur".into(),
            Asset::Crypto(Crypto::new_with_id("btc".into())),
            Asset::Fiat(Fiat::new("eur".into(), "Euro".into())),
            None,
        )
    }

    fn ohlc(candles: &str) -> String {
        format!(r#"{{"error":[],"result":{{"XXBTZEUR":[{candles}],"last":1700000000}}}}"#)
    }

    async fn price_of(fake: &FakeProvider) -> std::result::Result<f64, PriceFailure> {
        provider_at(fake)
            .fetch_market_price(&btceur(), jiff::Timestamp::now())
            .await
    }

    /// GIVEN Kraken answers an OHLC reply with candles,
    /// WHEN the market price is fetched,
    /// THEN the close of the latest candle is the price, requested for the market's pair.
    #[tokio::test]
    async fn latest_close_is_the_price() {
        let now = jiff::Timestamp::now().as_second();
        let candles = format!(
            r#"[{},"1","2","0.5","40000.5","1","3",5],[{},"1","2","0.5","40010.25","1","3",5]"#,
            now - 600,
            now - 300
        );
        let fake = FakeProvider::start(move |_| Reply::ok(ohlc(&candles))).await;

        assert_eq!(price_of(&fake).await, Ok(40010.25));

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0].starts_with("/0/public/OHLC?pair=btceur&since="),
            "{requests:?}"
        );
        assert!(requests[0].ends_with("&interval=5"));
    }

    /// GIVEN Kraken has no 5-minute candles but has a daily one,
    /// WHEN the market price is fetched,
    /// THEN the daily candle is used.
    #[tokio::test]
    async fn daily_candles_back_up_missing_five_minute_candles() {
        let fake = FakeProvider::start(|target| {
            if target.ends_with("interval=1440") {
                Reply::ok(ohlc(r#"[1700000000,"1","2","0.5","39000","1","3",5]"#))
            } else {
                Reply::ok(ohlc(""))
            }
        })
        .await;

        assert_eq!(price_of(&fake).await, Ok(39000.0));
    }

    /// GIVEN Kraken reports an unknown pair, a service error, an HTTP error, an
    /// undecodable body, or a close that is not a number,
    /// WHEN the market price is fetched,
    /// THEN the unknown pair is unavailable, errors are provider errors and
    /// unreadable payloads are malformed.
    #[tokio::test]
    async fn failures_are_told_apart() {
        let cases = [
            (
                Reply::ok(r#"{"error":["EQuery:Unknown asset pair"]}"#),
                FailureKind::Unavailable,
            ),
            (
                Reply::ok(r#"{"error":["EService:Unavailable"]}"#),
                FailureKind::ProviderError,
            ),
            (Reply::status(500, "oops"), FailureKind::ProviderError),
            (Reply::ok("<html>"), FailureKind::Malformed),
            (
                Reply::ok(ohlc(r#"[1700000000,"1","2","0.5","n/a","1","3",5]"#)),
                FailureKind::Malformed,
            ),
        ];
        for (reply, kind) in cases {
            let fake = FakeProvider::start({
                let reply = reply.clone();
                move |_| reply.clone()
            })
            .await;

            assert_eq!(price_of(&fake).await.unwrap_err().kind, kind, "{reply:?}");
        }
    }

    /// GIVEN a market id with reserved URL characters,
    /// WHEN its price is requested,
    /// THEN the pair is sent as a percent-encoded query value.
    #[tokio::test]
    async fn pair_is_percent_encoded() {
        let fake = FakeProvider::start(|_| Reply::ok(r#"{"error":[]}"#)).await;
        let mut market = btceur();
        market.id = "a&b=c".into();

        let _ = provider_at(&fake)
            .fetch_market_price(&market, jiff::Timestamp::now())
            .await;

        assert!(fake.requests()[0].starts_with("/0/public/OHLC?pair=a%26b%3Dc&"));
    }
}
