use std::collections::HashMap;

use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::{debug, error, warn};

use super::asset_provider::{AssetProvider, AssetProviderError};
use crate::{
    DateTime,
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::{Market, OHLCFrequency},
        portfolio::{AssetClass, Provider},
    },
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
                let res = res.json::<chart::ChartResponse>().await?;

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

        let res = res.json::<chart::ChartResponse>().await?;

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

    pub async fn search(&self, request_param: String) -> Response {
        let url = format!("https://query2.finance.yahoo.com/v1/finance/search?q={request_param}");
        self.forward(url).await
    }

    pub async fn chart(&self, symbol: String, start_period: i64, end_period: i64) -> Response {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1={start_period}&period2={end_period}&interval=5m&close=adjusted"
        );
        self.forward(url).await
    }

    async fn forward(&self, url: String) -> Response {
        match self.http.get(url).send().await {
            Ok(res) => {
                let mut response = Response::builder().status(res.status());

                // Copy headers
                for (key, value) in res.headers() {
                    response = response.header(key, value);
                }

                let body = axum::body::Body::from_stream(res.bytes_stream());
                response.body(body).unwrap().into_response()
            }
            Err(e) => {
                error!("Proxy error: {e:?}");
                StatusCode::BAD_GATEWAY.into_response()
            }
        }
    }
}

const CHART_BASE_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";

#[async_trait]
impl AssetProvider for YahooProvider {
    /// Resolves an asset from the `meta` block of a one-day Yahoo chart.
    async fn lookup(
        &self,
        asset: &AssetRef,
    ) -> std::result::Result<Option<ResolvedAsset>, AssetProviderError> {
        if asset.provider != Provider::YF {
            return Err(AssetProviderError::UnsupportedProvider(
                asset.provider.to_string(),
            ));
        }

        let url = lookup_url(&asset.symbol)?;
        let response = self
            .http
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;
        let status = response.status().as_u16();
        let body = response
            .bytes()
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;

        parse_lookup_response(asset, status, &body)
    }
}

/// Builds the chart URL for a symbol, percent-encoding it as a path segment.
fn lookup_url(symbol: &str) -> std::result::Result<reqwest::Url, AssetProviderError> {
    let mut url = reqwest::Url::parse(CHART_BASE_URL)
        .map_err(|e| AssetProviderError::Malformed(e.to_string()))?;
    url.path_segments_mut()
        .map_err(|_| AssetProviderError::Malformed("chart URL cannot be a base".to_string()))?
        .pop_if_empty()
        .push(symbol);
    url.query_pairs_mut()
        .append_pair("range", "1d")
        .append_pair("interval", "1d");
    Ok(url)
}

/// Turns a Yahoo chart reply into the metadata of the requested asset.
///
/// A 404 status or a `Not Found` chart error means Yahoo does not list the
/// symbol. Any other non-success status, undecodable body, or `meta` lacking a
/// currency or name is a provider failure. The requested symbol, not the one
/// echoed by Yahoo, is the asset key so aliases stay addressable.
///
/// `instrumentType` maps conservatively: EQUITY, ETF and MUTUALFUND are
/// equity; CRYPTOCURRENCY is crypto; CURRENCY is cash; everything else
/// (INDEX, FUTURE, OPTION, ...) is other, since Yahoo does not distinguish
/// bond or commodity funds.
fn parse_lookup_response(
    reference: &AssetRef,
    status: u16,
    body: &[u8],
) -> std::result::Result<Option<ResolvedAsset>, AssetProviderError> {
    if status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        return Err(AssetProviderError::Unavailable(
            format!("Yahoo answered with HTTP status {status}").into(),
        ));
    }

    let response: lookup::Response = serde_json::from_slice(body)
        .map_err(|e| AssetProviderError::Malformed(format!("undecodable chart body: {e}")))?;

    if let Some(error) = response.chart.error {
        return if error.code.eq_ignore_ascii_case("not found") {
            Ok(None)
        } else {
            Err(AssetProviderError::Malformed(format!(
                "chart error {}: {}",
                error.code,
                error.description.unwrap_or_default()
            )))
        };
    }

    let meta = response
        .chart
        .result
        .and_then(|results| results.into_iter().next())
        .map(|result| result.meta)
        .ok_or_else(|| AssetProviderError::Malformed("empty chart.result".to_string()))?;

    let currency = non_blank(meta.currency)
        .ok_or_else(|| AssetProviderError::Malformed("missing meta.currency".to_string()))?
        .to_ascii_uppercase();
    let name = non_blank(meta.long_name)
        .or_else(|| non_blank(meta.short_name))
        .ok_or_else(|| AssetProviderError::Malformed("missing meta.longName".to_string()))?;

    Ok(Some(ResolvedAsset {
        provider: Provider::YF,
        symbol: reference.symbol.clone(),
        name,
        currency,
        asset_class: asset_class_of(meta.instrument_type.as_deref()),
    }))
}

fn non_blank(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn asset_class_of(instrument_type: Option<&str>) -> AssetClass {
    match instrument_type
        .map(|t| t.trim().to_ascii_uppercase())
        .as_deref()
    {
        Some("EQUITY" | "ETF" | "MUTUALFUND") => AssetClass::Equity,
        Some("CRYPTOCURRENCY") => AssetClass::Crypto,
        Some("CURRENCY") => AssetClass::Cash,
        _ => AssetClass::Other,
    }
}

mod lookup {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub chart: Chart,
    }

    #[derive(Debug, Deserialize)]
    pub struct Chart {
        pub result: Option<Vec<ChartResult>>,
        pub error: Option<ChartError>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ChartError {
        pub code: String,
        pub description: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ChartResult {
        pub meta: Meta,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Meta {
        pub currency: Option<String>,
        pub long_name: Option<String>,
        pub short_name: Option<String>,
        pub instrument_type: Option<String>,
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    fn reference(symbol: &str) -> AssetRef {
        AssetRef::new(Provider::YF, symbol)
    }

    fn chart_body(meta: &str) -> String {
        format!(
            r#"{{"chart":{{"result":[{{"meta":{meta},"timestamp":[1],"indicators":{{"quote":[{{}}]}}}}],"error":null}}}}"#
        )
    }

    #[test]
    fn resolves_etf_in_euro() {
        // GIVEN a Yahoo chart reply for the VWCE.DE ETF quoted in EUR,
        // WHEN the reply is parsed, THEN the asset is equity with the long name and EUR.
        let body = chart_body(
            r#"{"currency":"EUR","symbol":"VWCE.DE","instrumentType":"ETF","longName":"Vanguard FTSE All-World UCITS ETF USD Accumulation","shortName":"VANGUARD FTSE ALL-WLD U.ETF"}"#,
        );

        let asset = parse_lookup_response(&reference("vwce.de"), 200, body.as_bytes())
            .unwrap()
            .unwrap();

        assert_eq!(asset.symbol, "VWCE.DE");
        assert_eq!(asset.currency, "EUR");
        assert_eq!(asset.asset_class, AssetClass::Equity);
        assert_eq!(
            asset.name,
            "Vanguard FTSE All-World UCITS ETF USD Accumulation"
        );
    }

    #[test]
    fn resolves_equity_falling_back_to_short_name() {
        // GIVEN a Yahoo chart reply for AAPL without a long name,
        // WHEN the reply is parsed, THEN the short name is used and the class is equity.
        let body = chart_body(
            r#"{"currency":"USD","symbol":"AAPL","instrumentType":"EQUITY","shortName":"Apple Inc."}"#,
        );

        let asset = parse_lookup_response(&reference("AAPL"), 200, body.as_bytes())
            .unwrap()
            .unwrap();

        assert_eq!(asset.name, "Apple Inc.");
        assert_eq!(asset.currency, "USD");
        assert_eq!(asset.asset_class, AssetClass::Equity);
    }

    #[test]
    fn resolves_cryptocurrency() {
        // GIVEN a Yahoo chart reply for BTC-USD, WHEN it is parsed,
        // THEN the asset is crypto quoted in USD.
        let body = chart_body(
            r#"{"currency":"USD","symbol":"BTC-USD","instrumentType":"CRYPTOCURRENCY","longName":"Bitcoin USD"}"#,
        );

        let asset = parse_lookup_response(&reference("BTC-USD"), 200, body.as_bytes())
            .unwrap()
            .unwrap();

        assert_eq!(asset.asset_class, AssetClass::Crypto);
        assert_eq!(asset.currency, "USD");
    }

    #[test]
    fn maps_instrument_types_conservatively() {
        // GIVEN assorted Yahoo instrument types, WHEN they are mapped,
        // THEN only the documented ones leave the Other class.
        for (kind, class) in [
            (Some("MUTUALFUND"), AssetClass::Equity),
            (Some("CURRENCY"), AssetClass::Cash),
            (Some("INDEX"), AssetClass::Other),
            (Some("FUTURE"), AssetClass::Other),
            (None, AssetClass::Other),
        ] {
            assert_eq!(asset_class_of(kind), class, "{kind:?}");
        }
    }

    #[test]
    fn not_found_body_is_an_unknown_ticker() {
        // GIVEN Yahoo's 404 body for a delisted or unknown symbol,
        // WHEN it is parsed, THEN the symbol is reported as unknown, not as a failure.
        let body = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found, symbol may be delisted"}}}"#;

        assert_eq!(
            parse_lookup_response(&reference("NOPE"), 404, body.as_bytes()).unwrap(),
            None
        );
        assert_eq!(
            parse_lookup_response(&reference("NOPE"), 200, body.as_bytes()).unwrap(),
            None
        );
    }

    #[test]
    fn failures_are_provider_errors() {
        // GIVEN a 5xx status, an undecodable body, and a reply lacking currency,
        // WHEN each is parsed, THEN each is a provider error rather than an unknown ticker.
        assert!(matches!(
            parse_lookup_response(&reference("X"), 503, b""),
            Err(AssetProviderError::Unavailable(_))
        ));
        assert!(matches!(
            parse_lookup_response(&reference("X"), 200, b"<html>"),
            Err(AssetProviderError::Malformed(_))
        ));
        let no_currency = chart_body(r#"{"symbol":"X","instrumentType":"EQUITY","longName":"X"}"#);
        assert!(matches!(
            parse_lookup_response(&reference("X"), 200, no_currency.as_bytes()),
            Err(AssetProviderError::Malformed(_))
        ));
        let no_name = chart_body(r#"{"currency":"USD","symbol":"X","instrumentType":"EQUITY"}"#);
        assert!(matches!(
            parse_lookup_response(&reference("X"), 200, no_name.as_bytes()),
            Err(AssetProviderError::Malformed(_))
        ));
    }

    #[test]
    fn symbol_is_percent_encoded_in_the_url() {
        // GIVEN a symbol containing reserved URL characters,
        // WHEN the lookup URL is built, THEN the symbol stays one encoded path segment.
        let url = lookup_url("A/B?C").unwrap();

        assert_eq!(
            url.as_str(),
            "https://query1.finance.yahoo.com/v8/finance/chart/A%2FB%3FC?range=1d&interval=1d"
        );
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

        format!("{base}-{quote}")
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
