use std::{collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use tracing::{debug, error, warn};

use super::asset_provider::{AssetProvider, AssetProviderError, ProviderAsset};
use crate::{
    DateTime,
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::{Market, OHLCFrequency},
        portfolio::{AssetClass, Provider},
    },
    error::{DcaError, Result},
};

/// Sustained and burst budget of requests per second shared by every Yahoo call.
///
/// Yahoo is unofficially said to throttle an IP at ~2500 requests per hour, but
/// production has never been throttled, so this is a generous ceiling that
/// smooths bursts and stops runaway loops. Lower it (for example with
/// `Quota::per_hour(2000).allow_burst(20)`) if Yahoo ever answers 429.
const YAHOO_REQUESTS_PER_SECOND: NonZeroU32 = NonZeroU32::new(10).unwrap();

/// Upper bound on one Yahoo request made by a worker or a proxy endpoint.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// How a Yahoo call behaves when the shared rate limiter has no capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Admission {
    /// Wait until a request is allowed (workers and synchronization lookups).
    Wait,
    /// Fail immediately instead of holding the caller (public proxy endpoints).
    Reject,
}

/// Failure of a rate-limited Yahoo request.
#[derive(Debug, thiserror::Error)]
enum GetError {
    /// The limiter had no capacity and the caller asked not to wait.
    #[error("Yahoo rate limit reached")]
    RateLimited,
    /// The HTTP request failed or timed out.
    #[error(transparent)]
    Request(#[from] rquest::Error),
}

impl From<GetError> for DcaError {
    fn from(error: GetError) -> Self {
        match error {
            GetError::RateLimited => DcaError::Generic(error.to_string()),
            GetError::Request(e) => e.into(),
        }
    }
}

/// Client of the Yahoo Finance endpoints.
///
/// All traffic goes through [`YahooProvider::get`], which applies one rate
/// limiter shared by prices, search, chart proxying and asset lookups.
#[derive(Clone)]
pub struct YahooProvider {
    http: rquest::Client,
    limiter: Arc<DefaultDirectRateLimiter>,
}

impl YahooProvider {
    /// Creates a provider limited to [`YAHOO_REQUESTS_PER_SECOND`].
    pub fn new(http: rquest::Client) -> Self {
        Self::with_quota(http, Quota::per_second(YAHOO_REQUESTS_PER_SECOND))
    }

    /// Creates a provider whose shared rate limiter enforces `quota`.
    fn with_quota(http: rquest::Client, quota: Quota) -> Self {
        Self {
            http,
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// Sends a GET to Yahoo once the rate limiter admits it.
    ///
    /// This is the only place that touches the HTTP client. `timeout` bounds the
    /// request itself, not the wait for the limiter.
    async fn get(
        &self,
        url: &str,
        timeout: Duration,
        admission: Admission,
    ) -> std::result::Result<rquest::Response, GetError> {
        match admission {
            Admission::Wait => self.limiter.until_ready().await,
            Admission::Reject => {
                if self.limiter.check().is_err() {
                    return Err(GetError::RateLimited);
                }
            }
        }
        Ok(self.http.get(url).timeout(timeout).send().await?)
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

        let res = self.get(&url, REQUEST_TIMEOUT, Admission::Wait).await?;
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

    /// Streams the Yahoo reply to the caller, answering 429 when the shared rate
    /// limiter has no capacity.
    async fn forward(&self, url: String) -> Response {
        match self.get(&url, REQUEST_TIMEOUT, Admission::Reject).await {
            Ok(res) => {
                let mut response = Response::builder().status(res.status());

                // Copy headers
                for (key, value) in res.headers() {
                    response = response.header(key, value);
                }

                let body = axum::body::Body::from_stream(res.bytes_stream());
                response.body(body).unwrap().into_response()
            }
            Err(GetError::RateLimited) => {
                warn!("Yahoo proxy request rejected by the rate limiter");
                StatusCode::TOO_MANY_REQUESTS.into_response()
            }
            Err(e) => {
                error!("Proxy error: {e:?}");
                StatusCode::BAD_GATEWAY.into_response()
            }
        }
    }
}

const CHART_BASE_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart/";

/// Upper bound on one Yahoo asset lookup (rate-limiter wait included), so a
/// stalled request cannot hold a synchronization open.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
impl AssetProvider for YahooProvider {
    /// Resolves an asset from the `meta` block of a one-day Yahoo chart.
    async fn lookup(
        &self,
        asset: &AssetRef,
    ) -> std::result::Result<Option<ProviderAsset>, AssetProviderError> {
        let url = lookup_url(asset.symbol())?;
        let (status, body) = tokio::time::timeout(LOOKUP_TIMEOUT, async {
            let response = self
                .get(url.as_str(), LOOKUP_TIMEOUT, Admission::Wait)
                .await
                .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;
            let status = response.status().as_u16();
            let body = response
                .bytes()
                .await
                .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;
            Ok::<_, AssetProviderError>((status, body))
        })
        .await
        .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))??;

        // Yahoo reports real display names, so they may replace cached ones.
        Ok(
            parse_lookup_response(asset, status, &body)?.map(|asset| ProviderAsset {
                asset,
                name_authoritative: true,
            }),
        )
    }
}

/// Builds the chart URL for a symbol, percent-encoding it as a path segment.
fn lookup_url(symbol: &str) -> std::result::Result<url::Url, AssetProviderError> {
    let mut url = url::Url::parse(CHART_BASE_URL)
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
/// HTTP 400, 404 and 422, or a `Not Found` chart error, mean Yahoo does not
/// list the symbol or rejects it. Any other non-success status (including 429
/// and 5xx), undecodable body, or `meta` lacking a currency or name is a
/// provider failure. The currency is kept exactly as Yahoo returns it (trimmed
/// only) because case is significant: `GBp` denotes pence while `GBP` denotes
/// pounds. The requested symbol, not the one
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
    if matches!(status, 400 | 404 | 422) {
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
        .ok_or_else(|| AssetProviderError::Malformed("missing meta.currency".to_string()))?;
    let name = non_blank(meta.long_name)
        .or_else(|| non_blank(meta.short_name))
        .ok_or_else(|| AssetProviderError::Malformed("missing meta.longName".to_string()))?;

    Ok(Some(ResolvedAsset {
        provider: Provider::YF,
        symbol: reference.symbol().to_string(),
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
    fn keeps_the_currency_case_so_pence_codes_survive() {
        // GIVEN a Yahoo chart reply for a London listing quoted in pence ("GBp"),
        // WHEN the reply is parsed,
        // THEN the currency stays "GBp" instead of becoming pounds ("GBP").
        let body = chart_body(
            r#"{"currency":" GBp ","symbol":"VOD.L","instrumentType":"EQUITY","longName":"Vodafone"}"#,
        );

        let asset = parse_lookup_response(&reference("VOD.L"), 200, body.as_bytes())
            .unwrap()
            .unwrap();

        assert_eq!(asset.currency, "GBp");
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
        // GIVEN Yahoo's 400/404/422 replies for a delisted, unknown or rejected symbol,
        // WHEN it is parsed, THEN the symbol is reported as unknown, not as a failure.
        let body = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found, symbol may be delisted"}}}"#;

        for status in [400, 404, 422] {
            assert_eq!(
                parse_lookup_response(&reference("NOPE"), status, body.as_bytes()).unwrap(),
                None,
                "status {status}"
            );
        }
        assert_eq!(
            parse_lookup_response(&reference("NOPE"), 200, body.as_bytes()).unwrap(),
            None
        );
    }

    #[test]
    fn failures_are_provider_errors() {
        // GIVEN a 429/5xx/other status, an undecodable body, and a reply lacking currency,
        // WHEN each is parsed, THEN each is a provider error rather than an unknown ticker.
        for status in [429, 500, 503, 401] {
            assert!(
                matches!(
                    parse_lookup_response(&reference("X"), status, b""),
                    Err(AssetProviderError::Unavailable(_))
                ),
                "status {status}"
            );
        }
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

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    /// Unroutable-in-practice local URL: connecting is refused immediately, so no
    /// test depends on the network.
    const REFUSED_URL: &str = "http://127.0.0.1:1/";

    /// A provider that admits a single request per hour, then nothing.
    fn single_request_provider() -> YahooProvider {
        let quota = Quota::per_hour(NonZeroU32::new(1).unwrap());
        YahooProvider::with_quota(rquest::Client::new(), quota)
    }

    #[tokio::test]
    async fn proxy_gets_too_many_requests_when_the_burst_is_spent() {
        // GIVEN a limiter whose only slot is already used,
        // WHEN the public search and chart proxies are called,
        // THEN both answer 429 without contacting Yahoo.
        let provider = single_request_provider();
        provider.limiter.check().unwrap();

        let search = provider.search("aapl".to_string()).await;
        let chart = provider.chart("AAPL".to_string(), 0, 1).await;

        assert_eq!(search.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(chart.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn rejecting_get_only_fails_once_capacity_is_spent() {
        // GIVEN a limiter with a single slot,
        // WHEN two non-waiting requests are made,
        // THEN the first reaches the (refused) connection and the second is rate limited.
        let provider = single_request_provider();

        let first = provider
            .get(REFUSED_URL, REQUEST_TIMEOUT, Admission::Reject)
            .await;
        let second = provider
            .get(REFUSED_URL, REQUEST_TIMEOUT, Admission::Reject)
            .await;

        assert!(matches!(first, Err(GetError::Request(_))));
        assert!(matches!(second, Err(GetError::RateLimited)));
    }

    #[tokio::test]
    async fn lookup_waits_for_capacity_instead_of_failing() {
        // GIVEN a limiter whose only slot is already used,
        // WHEN an asset lookup is started,
        // THEN it keeps waiting for the limiter rather than returning an error.
        let provider = single_request_provider();
        provider.limiter.check().unwrap();
        let asset = AssetRef::new(Provider::YF, "AAPL");

        let outcome =
            tokio::time::timeout(Duration::from_millis(150), provider.lookup(&asset)).await;

        assert!(outcome.is_err(), "lookup should still be waiting");
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
