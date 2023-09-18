use futures::StreamExt;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    time::Duration,
};
use tracing::{debug, error};

use crate::{
    config,
    domain::entity::{Asset, Crypto, Fiat, Market, MarketId, OHLCFrequency},
    error::{DcaError, Result},
    repository::market_data::MarketDataRepository,
    DateTime,
};

#[derive(Clone)]
pub struct KrakenProvider {
    http: reqwest::Client,
    cmc_api_key: Option<String>,
}

impl KrakenProvider {
    pub fn new(http: reqwest::Client, config: &config::Providers) -> Self {
        Self {
            http,
            cmc_api_key: config.cmc_api_key.clone(),
        }
    }

    pub async fn fetch_assets(
        &self,
        repo: &MarketDataRepository,
    ) -> Result<(Vec<Asset>, Vec<Market>)> {
        static URL: &str = "https://api.kraken.com/0/public/AssetPairs";

        // Fetch Kraken markets
        debug!(url = URL, "Fetching markets from CW");
        let res: AssetPairsResponse = self.fetch_kraken_api(URL).await?;
        if !res.error.is_empty() {
            error!("Error occurred while fetching asset pairs: {:?}", res.error);
            return Err(DcaError::Generic(format!("{:?}", res.error)));
        }

        // Filter online market symbols and rename Kraken specific pairs to standard names
        let market_symbols = res
            .result
            .values()
            .filter_map(|p| (p.status == "online").then(|| normalize_symbol(&p.wsname)))
            .collect::<Vec<String>>();

        let (markets, assets) = if self.cmc_api_key.is_some() {
            // If CoinMarketCap API key is available, enrich assets data with human-friendly info
            todo!();
        } else {
            // If not, `Asset`s will have same `id` and `symbol`. No big deal, just less fancy
            resolve_assets_data_kraken_only(&market_symbols, repo).await
        };

        debug!("New assets: {}", serde_json::to_string(&assets).unwrap());
        debug!("New markets: {}", serde_json::to_string(&markets).unwrap());

        Ok((assets, markets))
    }

    pub async fn fetch_market_price(&self, mkt: &Market, ts: DateTime) -> Result<Option<f64>> {
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

        let mut res: OHLCResult = self.fetch_kraken_api(&url).await?;
        if !res.error.is_empty() {
            error!("Error occurred while fetching '{id}': {:?}", res.error);
            return Err(DcaError::Generic(format!("{:?}", res.error)));
        }

        if res.result.is_empty() || res.result.len() < 2 {
            return Err(DcaError::Generic(format!(
                "Malformed response. Unexpected empty result: {res:?}"
            )));
        }

        res.result.remove("last"); // Drop fucking weird 'last' entry
        let Payload::CandleSticks(csticks) = res.result.values().next().unwrap() else {
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

    async fn fetch_kraken_api<T: DeserializeOwned + Debug>(&self, url: &str) -> Result<T> {
        let res = self
            .http
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if res.status().is_success() {
            Ok(res.json::<T>().await?)
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

async fn resolve_assets_data_kraken_only(
    market_symbols: &[String],
    repo: &MarketDataRepository,
) -> (Vec<Market>, Vec<Asset>) {
    let pairs = get_new_markets(market_symbols, repo).await;
    let ccys = get_unique_ccys(&pairs);
    let assets_map = get_assets(&ccys, repo).await;

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

fn get_unique_ccys(market_pairs: &[MarketPair]) -> HashSet<String> {
    market_pairs
        .iter()
        .flat_map(|(base, quote)| [base.clone(), quote.clone()])
        .collect::<_>()
}

async fn get_assets(
    ccys: &HashSet<String>,
    repo: &MarketDataRepository,
) -> BTreeMap<String, Asset> {
    let mut assets = BTreeMap::new();
    for ccy in ccys {
        match repo.find_asset(ccy).await {
            Ok(asset) => {
                if let Some(a) = asset {
                    assets.insert(a.id().clone(), a);
                } else {
                    let is_fiat = is_fiat(ccy);
                    let a = match is_fiat {
                        true => Asset::Fiat(Fiat {
                            id: ccy.to_string(),
                            symbol: ccy.to_uppercase(),
                        }),
                        false => Asset::Crypto(Crypto {
                            id: ccy.to_string(),
                            symbol: ccy.to_uppercase(),
                        }),
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

fn is_fiat(id: &str) -> bool {
    lazy_static! {
        static ref FIAT_IDS: HashSet<&'static str> =
            HashSet::from(["aed", "aud", "cad", "chf", "eur", "gbp", "jpy", "usd"]);
    }

    FIAT_IDS.contains(id)
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

#[derive(Debug, Clone)]
struct KAssetsDataCurrency {
    symbol: String,
    name: String,
    is_fiat: bool,
}

#[allow(clippy::from_over_into)]
impl Into<Asset> for KAssetsDataCurrency {
    fn into(self) -> Asset {
        if self.is_fiat {
            Asset::Fiat(Fiat {
                id: self.symbol,
                symbol: self.name,
            })
        } else {
            Asset::Crypto(Crypto {
                id: self.symbol,
                symbol: self.name,
            })
        }
    }
}

#[derive(Debug, Clone)]
struct KAssetData {
    _base: KAssetsDataCurrency,
    _quote: KAssetsDataCurrency,
}

#[derive(Debug, Clone, Deserialize)]
struct OHLCResult {
    error: Vec<String>,
    result: HashMap<String, Payload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Payload {
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
