use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    sync::LazyLock,
    time::Duration,
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

/// Number of symbols requested from CoinMarketCap in one call.
const CMC_CHUNK_SIZE: usize = 100;

/// Pause between consecutive CoinMarketCap chunk requests, so a full discovery
/// sweep does not burst through the provider's rate limit.
const CMC_CHUNK_PACING: Duration = Duration::from_millis(2500);

/// Production CoinMarketCap API root.
const CMC_BASE_URL: &str = "https://pro-api.coinmarketcap.com";

#[derive(Clone)]
pub struct KrakenProvider {
    http: reqwest::Client,
    /// Root URL of the Kraken public API, without a trailing slash.
    base_url: String,
    cmc_api_key: Option<String>,
    cmc_base_url: String,
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
            cmc_base_url: CMC_BASE_URL.to_string(),
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
        let (mut assets_map, unresolved) = self.fetch_assets_cmc(&ccys, repo).await;

        // Placeholder assets filled in after a CMC failure are neither stored nor
        // used by markets, so the next discovery run (which finds their markets
        // still missing) retries them instead of freezing placeholder names.
        if !unresolved.is_empty() {
            warn!(
                count = unresolved.len(),
                "Not storing assets whose CMC metadata could not be fetched; they are retried on the next discovery run"
            );
            assets_map.retain(|id, _| !unresolved.contains(id));
        }

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
    ///
    /// Returns the resolved assets and the ids that only hold a placeholder
    /// because their CoinMarketCap request failed (as opposed to CoinMarketCap
    /// not knowing them); callers must not persist those as provider-confirmed.
    async fn fetch_assets_cmc(
        &self,
        ccys: &HashSet<AssetId>,
        repo: &MarketDataRepository,
    ) -> (BTreeMap<String, Asset>, HashSet<AssetId>) {
        let mut assets = BTreeMap::new();
        let mut to_fetch = Vec::new();
        let ccys = ccys.iter().collect::<Vec<_>>();
        let found = match repo.find_assets(&ccys).await {
            Ok(found) => found,
            Err(e) => {
                error!("Failed to find {} assets: {e}", ccys.len());
                return (assets, HashSet::new());
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

        let unresolved = self
            .resolve_cmc_chunks(&to_fetch, &mut assets, CMC_CHUNK_PACING)
            .await;

        (assets, unresolved)
    }

    /// Fetches CoinMarketCap metadata for `to_fetch` in paced chunks and inserts
    /// the results into `assets`.
    ///
    /// Symbols CoinMarketCap does not know get a default asset. Symbols whose
    /// request failed also get a default asset and are returned, so the caller
    /// can avoid persisting them. Once the CoinMarketCap circuit breaker opens,
    /// no further requests are made and the remaining symbols are reported with
    /// a single summary warning.
    async fn resolve_cmc_chunks(
        &self,
        to_fetch: &[&AssetId],
        assets: &mut BTreeMap<String, Asset>,
        pacing: Duration,
    ) -> HashSet<AssetId> {
        let mut unresolved = HashSet::new();
        let mut breaker_open = false;
        let mut skipped = 0usize;
        let mut requests = 0usize;

        for chunk in to_fetch.chunks(CMC_CHUNK_SIZE) {
            if breaker_open {
                skipped += chunk.len();
                fill_default_assets(chunk, assets, &mut unresolved);
                continue;
            }

            if requests > 0 {
                tokio::time::sleep(pacing).await;
            }
            requests += 1;

            match self.fetch_crypto_cmc(chunk).await {
                Ok(fetched) => {
                    fetched.into_iter().for_each(|a| {
                        assets.insert(a.id().clone(), a);
                    });

                    for id in chunk {
                        if !assets.contains_key(*id) {
                            warn!(
                                "Cannot find CMC asset '{id}' in `assets` map. Filling with default"
                            );

                            assets.insert(
                                id.to_string(),
                                Asset::Crypto(Crypto::new_with_id(id.to_string())),
                            );
                        }
                    }
                }
                Err(e) => {
                    // The underlying error was already logged with its cause chain
                    // by `fetch_cmc_api`.
                    if matches!(e, DcaError::ExternalServiceDied(_)) {
                        breaker_open = true;
                        skipped += chunk.len();
                    } else {
                        warn!(
                            symbols = chunk.len(),
                            "Failed to fetch CMC assets chunk. Filling with default"
                        );
                    }
                    fill_default_assets(chunk, assets, &mut unresolved);
                }
            }
        }

        if skipped > 0 {
            warn!(
                symbols = skipped,
                "CMC circuit breaker is open: skipped remaining CMC requests and filled {skipped} symbols with defaults"
            );
        }

        unresolved
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

        let url = format!(
            "{}/v2/cryptocurrency/info?symbol={symbols}",
            self.cmc_base_url
        );

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
            Err(DcaError::UpstreamStatus {
                service: "Kraken",
                status: res.status(),
            })
        }
    }

    /// Calls CoinMarketCap through its circuit breaker. Recorded as one `lookup`
    /// provider call, including calls rejected by the open breaker.
    async fn fetch_cmc_api<T: DeserializeOwned + Debug>(&self, url: &str) -> Result<Option<T>> {
        observe_provider(
            MetricProvider::CoinMarketCap,
            ProviderOp::Lookup,
            async {
                self.cmc_circuit_breaker
                    .call(self.fetch_cmc_api_inner::<T>(url))
                    .await
                    .map_err(|e| {
                        // Breaker rejections are summarized by the caller; real
                        // failures are logged here with their cause chain.
                        if let failsafe::Error::Inner(inner) = &e {
                            warn!("CoinMarketCap request failed: {inner:?}");
                        }
                        DcaError::from_failsafe(e, "CoinMarketCapProvider")
                    })
            },
            |result| match result {
                Ok(Some(_)) => ProviderOutcome::Ok,
                Ok(None) => ProviderOutcome::NotFound,
                Err(e) => outcome_of_error(e),
            },
        )
        .await
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
            Err(DcaError::UpstreamStatus {
                service: "CoinMarketCap",
                status: res.status(),
            })
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

/// Classifies a failed Kraken or CoinMarketCap call for the provider metrics:
/// an open circuit breaker is `circuit_open`, HTTP 429 is `rate_limited` and
/// timeouts are `timeout`.
fn outcome_of_error(error: &DcaError) -> ProviderOutcome {
    match error {
        DcaError::ExternalServiceDied(_) => ProviderOutcome::CircuitOpen,
        DcaError::UpstreamStatus { status, .. } if *status == StatusCode::TOO_MANY_REQUESTS => {
            ProviderOutcome::RateLimited
        }
        DcaError::Reqwest(e) if e.is_timeout() => ProviderOutcome::Timeout,
        DcaError::Reqwest(e) if e.status() == Some(StatusCode::TOO_MANY_REQUESTS) => {
            ProviderOutcome::RateLimited
        }
        _ => ProviderOutcome::Error,
    }
}

/// Inserts a default (placeholder) crypto asset for each id and records it as
/// unresolved.
fn fill_default_assets(
    ids: &[&AssetId],
    assets: &mut BTreeMap<String, Asset>,
    unresolved: &mut HashSet<AssetId>,
) {
    for id in ids {
        assets.insert(
            id.to_string(),
            Asset::Crypto(Crypto::new_with_id(id.to_string())),
        );
        unresolved.insert(id.to_string());
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
        AssetPairsResponse, KrakenProvider, MarketSymbol, Pair, fetch_assets, get_new_markets,
        market_ids_from_symbols, outcome_of_error, parse_market_symbol,
        parse_online_market_symbols,
    };
    use crate::{app::infra::stats::ProviderOutcome, error::DcaError};
    use reqwest::StatusCode;
    use std::{
        collections::BTreeMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    /// Starts a local CoinMarketCap stub that answers every request with `status`
    /// and, for 200, a body naming each requested symbol `Name<SYMBOL>`.
    /// Returns its base URL and a counter of the requests it received.
    async fn cmc_stub(status: u16) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let counter = counter.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 16 * 1024];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();
                    counter.fetch_add(1, Ordering::SeqCst);
                    let body = if status == 200 {
                        let symbols = request
                            .split("symbol=")
                            .nth(1)
                            .and_then(|r| r.split(' ').next())
                            .unwrap_or("");
                        let data = symbols
                            .split(',')
                            .map(|sym| {
                                format!(
                                    r#""{u}":[{{"id":1,"symbol":"{u}","name":"Name{u}"}}]"#,
                                    u = sym.to_uppercase()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        format!(
                            r#"{{"status":{{"error_code":0,"error_message":null}},"data":{{{data}}}}}"#
                        )
                    } else {
                        "{}".to_string()
                    };
                    let response = format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        (base, hits)
    }

    fn provider(base_url: String) -> KrakenProvider {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        KrakenProvider {
            http: reqwest::Client::new(),
            base_url: super::DEFAULT_BASE_URL.to_string(),
            cmc_api_key: Some("test-key".to_string()),
            cmc_base_url: base_url,
            kraken_circuit_breaker: failsafe::Config::new().build(),
            cmc_circuit_breaker: failsafe::Config::new().build(),
        }
    }

    fn name_of(asset: &Asset) -> &str {
        match asset {
            Asset::Crypto(c) => &c.symbol,
            Asset::Fiat(f) => &f.symbol,
        }
    }

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("c{i:04}")).collect()
    }

    #[tokio::test]
    async fn rate_limited_cmc_stops_after_breaker_opens_and_leaves_assets_unresolved() {
        // GIVEN CoinMarketCap answers HTTP 429 to every request and 800 symbols
        // (8 chunks) need metadata,
        let (base, hits) = cmc_stub(429).await;
        let provider = provider(base);
        let symbols = ids(800);
        let to_fetch = symbols.iter().collect::<Vec<_>>();
        let mut assets = BTreeMap::new();

        // WHEN the chunks are resolved,
        let unresolved = provider
            .resolve_cmc_chunks(&to_fetch, &mut assets, Duration::ZERO)
            .await;

        // THEN requests stop once the breaker opens (5 failures, no 6th request),
        // every symbol gets a placeholder and every symbol is reported unresolved.
        assert_eq!(hits.load(Ordering::SeqCst), 5);
        assert_eq!(assets.len(), 800);
        assert_eq!(unresolved.len(), 800);
        assert_eq!(name_of(&assets["c0000"]), "C0000");
    }

    #[tokio::test]
    async fn successful_cmc_applies_names_and_leaves_nothing_unresolved() {
        // GIVEN CoinMarketCap answers every request successfully,
        let (base, hits) = cmc_stub(200).await;
        let provider = provider(base);
        let symbols = ids(150);
        let to_fetch = symbols.iter().collect::<Vec<_>>();
        let mut assets = BTreeMap::new();

        // WHEN the symbols are resolved (two chunks),
        let unresolved = provider
            .resolve_cmc_chunks(&to_fetch, &mut assets, Duration::ZERO)
            .await;

        // THEN provider names are applied and nothing is flagged unresolved.
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        assert!(unresolved.is_empty());
        assert_eq!(assets.len(), 150);
        assert_eq!(name_of(&assets["c0000"]), "NameC0000");
    }

    #[test]
    fn provider_failures_are_classified_for_metrics() {
        // GIVEN the errors a Kraken or CoinMarketCap call can fail with,
        let rate_limited = DcaError::UpstreamStatus {
            service: "CoinMarketCap",
            status: StatusCode::TOO_MANY_REQUESTS,
        };
        let server_error = DcaError::UpstreamStatus {
            service: "Kraken",
            status: StatusCode::BAD_GATEWAY,
        };
        let breaker = DcaError::ExternalServiceDied("CoinMarketCapProvider".to_string());

        // WHEN they are classified,
        // THEN 429 is rate limited, an open breaker is circuit open, others are errors.
        assert_eq!(
            outcome_of_error(&rate_limited),
            ProviderOutcome::RateLimited
        );
        assert_eq!(outcome_of_error(&breaker), ProviderOutcome::CircuitOpen);
        assert_eq!(outcome_of_error(&server_error), ProviderOutcome::Error);
    }

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
