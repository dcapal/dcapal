use std::collections::HashMap;

use tracing::{debug, warn};

use crate::{
    DateTime,
    app::domain::entity::{Market, OHLCFrequency},
    error::{DcaError, Result},
};

#[derive(Clone)]
pub struct YahooProvider {
    http: rquest::Client,
}

impl YahooProvider {
    pub fn new(http: rquest::Client) -> Self {
        Self { http }
    }

    pub async fn fetch_market_price(&self, mkt: &Market, ts: DateTime) -> Result<Option<f64>> {
        if let Some(px) = self.fetch_price(mkt, OHLCFrequency::Minutes5, ts).await? {
            Ok(Some(px))
        } else if let Some(px) = self.fetch_price(mkt, OHLCFrequency::Daily, ts).await? {
            debug!(
                "Cannot fetch price for '{}' with '{}' frequency. Retrying with '{}'",
                mkt.id,
                OHLCFrequency::Minutes5,
                OHLCFrequency::Daily
            );
            Ok(Some(px))
        } else {
            Ok(None)
        }
    }

    async fn fetch_price(
        &self,
        mkt: &Market,
        freq: OHLCFrequency,
        ts: DateTime,
    ) -> Result<Option<f64>> {
        let symbol = mkt.as_yahoo();
        let interval = get_api_interval(freq);
        let (r_lo, r_hi) = freq.ohlc_range(ts);
        let (period_1, period_2) = (r_lo.timestamp(), r_hi.timestamp());
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1={period_1}&period2={period_2}&interval={interval}"
        );

        debug!(
            url = url,
            "Fetching {} OHLC candlestick for market '{}' in range [{}, {}]",
            freq,
            mkt.id,
            r_lo,
            r_hi
        );

        let res = self.http.get(&url).send().await?;
        if !res.status().is_success() {
            if res.status() == reqwest::StatusCode::NOT_FOUND {
                let full = res.bytes().await?;

                let res = serde_json::from_slice::<chart::ChartResponse>(&full).map_err(|_| {
                    DcaError::Generic(format!(
                        "Malformed response. Unexpected empty chart.result: {:?}",
                        full
                    ))
                })?;

                if let Some(e) = res.chart.error {
                    warn!(
                        url = url,
                        "Unsuccessful request. Code: {}. Description: {}", e.code, e.description
                    );
                }
                return Ok(None);
            }

            return Err(res.error_for_status().unwrap_err().into());
        }

        //        let res = res.json::<chart::ChartResponse>().await?;
        let full = res.bytes().await?;
        let res = serde_json::from_slice::<chart::ChartResponse>(&full).map_err(|_| {
            DcaError::Generic(format!(
                "Malformed response. Unexpected empty chart.result: {:?}",
                full
            ))
        })?;

        if let Some(e) = res.chart.error {
            warn!(
                url = url,
                "Unsuccessful request. Code: {}. Description: {}", e.code, e.description
            );
            return Ok(None);
        }

        let result = res.chart.result;
        if result.is_none() || result.as_ref().unwrap().is_empty() {
            return Err(DcaError::Generic(
                "Malformed response. Unexpected empty chart.result".to_owned(),
            ));
        }
        let result = result.unwrap();
        let result = &result[0];
        if result.indicators.quote.is_empty() {
            return Err(DcaError::Generic(
                "Malformed response. Unexpected empty chart.result.indicators.quote".to_owned(),
            ));
        }

        let quote = &result.indicators.quote[0];
        match quote {
            chart::QuotesKind::Empty {} => Ok(None),
            chart::QuotesKind::Quotes(q) => {
                let price = if q.close.is_empty() {
                    None
                } else {
                    q.close.iter().rev().find_map(|p| *p)
                };

                Ok(price)
            }
        }
    }

    pub async fn search(&self, request_param: String) -> String {
        let url = format!("https://query2.finance.yahoo.com/v1/finance/search?q={request_param}");
        self.http
            .get(&url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }

    pub async fn chart(&self, symbol: String, start_period: i64, end_period: i64) -> String {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1={start_period}&period2={end_period}&interval=5m&close=adjusted"
        );
        self.http
            .get(&url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }
}

fn get_api_interval(freq: OHLCFrequency) -> &'static str {
    match freq {
        OHLCFrequency::Minutes5 => "5m",
        OHLCFrequency::Daily => "1d",
    }
}

pub trait AsYahooMarket {
    fn as_yahoo(&self) -> String;
}

impl AsYahooMarket for Market {
    fn as_yahoo(&self) -> String {
        lazy_static::lazy_static! {
            static ref MAPPINGS: HashMap<&'static str, &'static str> = {
                [("luna", ("luna1"))].into_iter().collect()
            };
        }

        if self.is_fiat() {
            if self.base.id() == "usd" {
                return format!("{}=x", self.quote.id());
            } else {
                return format!("{}{}=x", self.base.id(), self.quote.id());
            }
        }

        let base = MAPPINGS
            .get(self.base.id().as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.base.id().clone());

        let quote = MAPPINGS
            .get(self.quote.id().as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.quote.id().clone());

        format!("{}-{}", base, quote)
    }
}

mod chart {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct ChartResponse {
        pub chart: Chart,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Chart {
        pub result: Option<Vec<Candlestick>>,
        pub error: Option<Error>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Error {
        pub code: String,
        pub description: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Candlestick {
        pub indicators: Indicators,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Indicators {
        pub quote: Vec<QuotesKind>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(untagged)]
    pub enum QuotesKind {
        Quotes(Quotes),
        Empty {},
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Quotes {
        pub close: Vec<Option<f64>>,
    }
}
