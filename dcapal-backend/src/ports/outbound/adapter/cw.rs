use futures::{future, StreamExt};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};
use tracing::{debug, error, info};

use crate::{
    app::domain::entity::{Asset, Crypto, Fiat, Market, MarketId, OHLCFrequency},
    config,
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
    DateTime,
};

#[derive(Clone)]
pub struct CryptoWatchProvider {
    http: reqwest::Client,
    api_key: String,
}

impl CryptoWatchProvider {
    pub fn new(http: reqwest::Client, config: &config::Providers) -> Self {
        Self {
            http,
            api_key: config.cw_api_key.clone(),
        }
    }

    pub async fn fetch_assets(
        &self,
        repo: &MarketDataRepository,
    ) -> Result<(Vec<Asset>, Vec<Market>)> {
        const URL: &str = formatcp!("https://api.cryptowat.ch/markets/kraken");

        // Fetch CW markets
        debug!(url = URL, "Fetching markets from CW");
        let res: CWMarketsResponse = self.fetch_cw_api(URL).await?;
        let market_symbols = res
            .result
            .into_iter()
            .filter_map(|m| m.is_active.then_some(m.symbol))
            .collect::<Vec<String>>();

        // Fetch CW assets data
        let assets_fut = futures::stream::iter(market_symbols)
            .filter(|id| is_new_market(id.clone(), repo))
            .map(|s| self.fetch_asset(s))
            .collect::<Vec<_>>()
            .await;
        let cw_assets = future::join_all(assets_fut).await;
        let cw_assets = cw_assets
            .into_iter()
            .inspect(|r| {
                if let Err(e) = r {
                    error!("Failed to fetch some CW asset data: {}", e);
                }
            })
            .filter_map(|a| a.is_ok().then(|| a.unwrap()))
            .collect::<Vec<(String, CWAssetData)>>();

        // Map assets to entities
        let assets_map = cw_assets
            .iter()
            .flat_map(|(_, a)| [a.base.clone().into(), a.quote.clone().into()])
            .map(|a: Asset| (a.id().clone(), a))
            .collect::<BTreeMap<_, _>>();

        let markets = cw_assets
            .into_iter()
            .map(|(pair, a)| {
                let base = assets_map
                    .get(&a.base.symbol)
                    .expect("Base asset not found");
                let quote = assets_map
                    .get(&a.quote.symbol)
                    .expect("Quote asset not found");

                Market::new(pair, base.clone(), quote.clone())
            })
            .collect::<Vec<Market>>();

        let assets = assets_map.into_values().collect();

        debug!("New assets: {}", serde_json::to_string(&assets).unwrap());
        debug!("New markets: {}", serde_json::to_string(&markets).unwrap());

        Ok((assets, markets))
    }

    async fn fetch_asset(&self, symbol: String) -> Result<(String, CWAssetData)> {
        let url = format!("https://api.cryptowat.ch/pairs/{symbol}");

        debug!(url = url, "Fetching '{}' pair from CW", symbol);
        let res: CWAssetDataResult = self.fetch_cw_api(&url).await?;

        Ok((symbol.to_string(), res.result))
    }

    pub async fn fetch_market_price(&self, mkt: &Market, ts: DateTime) -> Result<Option<f64>> {
        let (px_mins5, px_day) = tokio::join!(
            self.fetch_price(&mkt.id, OHLCFrequency::Minutes5, ts),
            self.fetch_price(&mkt.id, OHLCFrequency::Daily, ts)
        );
        let (px_mins5, px_day) = (px_mins5?, px_day?);
        if let Some(px) = px_mins5 {
            Ok(Some(px))
        } else if let Some(px) = px_day {
            Ok(Some(px))
        } else {
            Ok(None)
        }
    }

    async fn fetch_price(
        &self,
        id: &MarketId,
        freq: OHLCFrequency,
        ts: DateTime,
    ) -> Result<Option<f64>> {
        let periods = get_cw_api_periods(freq);
        let (r_lo, r_hi) = freq.ohlc_range(ts);
        let (after_ts, before_ts) = (r_lo.timestamp(), r_hi.timestamp());
        let url = format!("https://api.cryptowat.ch/markets/kraken/{id}/ohlc?after={after_ts}&before={before_ts}&periods={periods}");

        debug!(
            url = url,
            "Fetching {} OHLC candlestick for market '{}' in range [{}, {}]", freq, id, r_lo, r_hi
        );
        let res: CWOHLCResult = self.fetch_cw_api(&url).await?;
        if res.result.is_empty() || !res.result.contains_key(periods) {
            return Err(DcaError::Generic(format!(
                "Malformed response. Unexpected empty result: {:?}",
                res
            )));
        }

        let csticks = res.result.get(periods).unwrap();
        if csticks.is_empty() {
            return Ok(None);
        }

        let cstick = csticks.last().unwrap();
        if cstick.len() < 5 {
            return Err(DcaError::Generic(format!(
                "Malformed response. Unexpected candlestick length: {:?}",
                csticks
            )));
        }

        Ok(Some(cstick[4]))
    }

    async fn fetch_cw_api<T: DeserializeOwned + Debug>(&self, url: &str) -> Result<T> {
        let res = self.http.get(url).send().await?;
        if res.status().is_success() {
            return Ok(res.json::<T>().await?);
        }

        if res.status() != StatusCode::TOO_MANY_REQUESTS {
            return Err(res.error_for_status().unwrap_err().into());
        }

        // Retry with our API key
        info!(url = url, "CW free plan exhausted. Retrying with API Key");
        let res = self
            .http
            .get(url)
            .header("X-CW-API-Key", &self.api_key)
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
struct CWMarket {
    #[serde(rename = "pair")]
    symbol: String,
    #[serde(rename = "active")]
    is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct CWMarketsResponse {
    result: Vec<CWMarket>,
}

async fn is_new_market(id: MarketId, repo: &MarketDataRepository) -> bool {
    let mkt = repo.find_market(&id).await.unwrap_or_else(|e| {
        error!("Error occurred in searching market '{}': {}", id, e);
        None
    });

    mkt.is_none()
}

fn get_cw_api_periods(freq: OHLCFrequency) -> &'static str {
    match freq {
        OHLCFrequency::Minutes5 => "300",
        OHLCFrequency::Daily => "86400",
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CWAssetsDataCurrency {
    symbol: String,
    name: String,
    #[serde(rename = "fiat")]
    is_fiat: bool,
}

#[allow(clippy::from_over_into)]
impl Into<Asset> for CWAssetsDataCurrency {
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

#[derive(Debug, Clone, Deserialize)]
struct CWAssetData {
    base: CWAssetsDataCurrency,
    quote: CWAssetsDataCurrency,
}

#[derive(Debug, Clone, Deserialize)]
struct CWAssetDataResult {
    result: CWAssetData,
}

#[derive(Debug, Clone, Deserialize)]
struct CWOHLCResult {
    result: HashMap<String, Vec<Vec<f64>>>,
}
