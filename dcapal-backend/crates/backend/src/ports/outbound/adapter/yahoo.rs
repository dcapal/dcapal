use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, LazyLock},
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use tracing::{debug, error, warn};

use super::{
    AssetSearchError, AssetSearchProvider, ListedAsset, LivePrice, LivePriceProvider,
    PriceCallError,
    asset_provider::{AssetProvider, AssetProviderError, ProviderAsset},
};
use crate::{
    DateTime,
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::{Market, OHLCFrequency},
        market_quote::{FailureKind, PriceFailure, validate_price},
        portfolio::{AssetClass, Provider},
    },
    app::infra::stats::{
        MetricProvider, ProviderOp, ProviderOutcome, observe_provider,
        record_yahoo_limiter_rejection, record_yahoo_limiter_wait, record_yahoo_upstream_response,
    },
    error::{DcaError, ErrorChain},
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
    endpoints: Arc<YahooEndpoints>,
}

/// Root URLs of the Yahoo chart and search endpoints, without a trailing slash.
#[derive(Debug, Clone)]
struct YahooEndpoints {
    chart: String,
    search: String,
}

impl Default for YahooEndpoints {
    fn default() -> Self {
        Self {
            chart: "https://query1.finance.yahoo.com/v8/finance/chart".to_string(),
            search: "https://query2.finance.yahoo.com/v1/finance/search".to_string(),
        }
    }
}

impl YahooEndpoints {
    /// Serves both endpoints from one host, such as a local fake or a proxy.
    fn rooted_at(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            chart: format!("{base}/v8/finance/chart"),
            search: format!("{base}/v1/finance/search"),
        }
    }

    /// Builds the chart URL of a symbol, percent-encoding it as one path segment.
    fn chart_url(
        &self,
        symbol: &str,
        params: &[(&str, &str)],
    ) -> std::result::Result<url::Url, String> {
        let mut url = url::Url::parse(&self.chart).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "chart URL cannot be a base".to_string())?
            .pop_if_empty()
            .push(symbol);
        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    /// Builds the search URL, percent-encoding the free-text query.
    fn search_url(&self, query: &str) -> std::result::Result<url::Url, String> {
        let mut url = url::Url::parse(&self.search).map_err(|e| e.to_string())?;
        url.query_pairs_mut().append_pair("q", query);
        Ok(url)
    }
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
            endpoints: Arc::new(YahooEndpoints::default()),
        }
    }

    /// Sends every request to `base` instead of the Yahoo hosts.
    ///
    /// Used by local fakes in tests and by deployments behind a proxy.
    pub fn with_base_url(mut self, base: &str) -> Self {
        self.endpoints = Arc::new(YahooEndpoints::rooted_at(base));
        self
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
            Admission::Wait => {
                let start = std::time::Instant::now();
                self.limiter.until_ready().await;
                record_yahoo_limiter_wait(start.elapsed());
            }
            Admission::Reject => {
                if self.limiter.check().is_err() {
                    record_yahoo_limiter_rejection();
                    return Err(GetError::RateLimited);
                }
            }
        }
        let response = self.http.get(url).timeout(timeout).send().await?;
        record_yahoo_upstream_response(response.status());
        Ok(response)
    }

    /// Fetches the latest price of a market, falling back from 5-minute to daily
    /// candles. Recorded as one `price` provider call (both attempts included).
    ///
    /// The daily fallback only follows a 5-minute answer that carries no price. A
    /// provider error or a malformed reply is reported as is.
    pub async fn fetch_market_price(
        &self,
        mkt: &Market,
        ts: DateTime,
    ) -> std::result::Result<f64, PriceFailure> {
        observe_provider(
            MetricProvider::Yahoo,
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
        let Some(symbol) = mkt.as_yahoo() else {
            warn!(mkt = mkt.id, "No Yahoo ticker for market");
            return Err(PriceFailure::unavailable("Yahoo does not price this market").into());
        };

        match self.fetch_price(&symbol, OHLCFrequency::Minutes5, ts).await {
            Err(e) if e.failure.kind == FailureKind::Unavailable => {
                debug!(
                    "Cannot fetch price for '{}' with '{}' frequency. Retrying with '{}'",
                    mkt.id,
                    OHLCFrequency::Minutes5,
                    OHLCFrequency::Daily
                );
                self.fetch_price(&symbol, OHLCFrequency::Daily, ts).await
            }
            other => other,
        }
    }

    async fn fetch_price(
        &self,
        symbol: &str,
        freq: OHLCFrequency,
        ts: DateTime,
    ) -> std::result::Result<f64, PriceCallError> {
        let interval = get_api_interval(freq);
        let (r_lo, r_hi) = freq.ohlc_range(ts);
        let (period_1, period_2) = (r_lo.as_second().to_string(), r_hi.as_second().to_string());
        let url = self
            .endpoints
            .chart_url(
                symbol,
                &[
                    ("period1", &period_1),
                    ("period2", &period_2),
                    ("interval", interval),
                ],
            )
            .map_err(PriceFailure::malformed)?;

        debug!(
            url = url.as_str(),
            "Fetching {} OHLC candlestick for '{}' in range [{}, {}]", freq, symbol, r_lo, r_hi
        );

        let (status, body) = self.read_body(url.as_str(), REQUEST_TIMEOUT).await?;
        Ok(parse_chart_price(status, &body)?)
    }

    /// Sends a waiting GET and reads the whole reply, mapping transport problems to failures.
    async fn read_body(
        &self,
        url: &str,
        timeout: Duration,
    ) -> std::result::Result<(u16, Vec<u8>), PriceCallError> {
        let response = self
            .get(url, timeout, Admission::Wait)
            .await
            .map_err(|e| call_error_of(&e))?;
        let status = response.status().as_u16();
        let body = response
            .bytes()
            .await
            .map_err(|e| call_error_of(&GetError::Request(e)))?;
        Ok((status, body.to_vec()))
    }

    /// Proxies a free-text search, percent-encoding the query.
    pub async fn search(&self, request_param: String) -> Response {
        match self.endpoints.search_url(&request_param) {
            Ok(url) => self.forward(url.to_string()).await,
            Err(e) => bad_gateway(&e),
        }
    }

    /// Proxies the 5-minute chart of a symbol, percent-encoding it as one path segment.
    pub async fn chart(&self, symbol: String, start_period: i64, end_period: i64) -> Response {
        let (start, end) = (start_period.to_string(), end_period.to_string());
        let url = self.endpoints.chart_url(
            &symbol,
            &[
                ("period1", &start),
                ("period2", &end),
                ("interval", "5m"),
                ("close", "adjusted"),
            ],
        );
        match url {
            Ok(url) => self.forward(url.to_string()).await,
            Err(e) => bad_gateway(&e),
        }
    }

    /// Streams the Yahoo reply to the caller, answering 429 when the shared rate
    /// limiter has no capacity.
    ///
    /// Recorded as one `proxy` provider call, timed until the response headers
    /// arrive (the body is streamed afterwards).
    async fn forward(&self, url: String) -> Response {
        let result = observe_provider(
            MetricProvider::Yahoo,
            ProviderOp::Proxy,
            self.get(&url, REQUEST_TIMEOUT, Admission::Reject),
            |result| match result {
                Ok(res) => ProviderOutcome::from_status(res.status()),
                Err(GetError::RateLimited) => ProviderOutcome::RateLimited,
                Err(GetError::Request(e)) if e.is_timeout() => ProviderOutcome::Timeout,
                Err(GetError::Request(_)) => ProviderOutcome::Error,
            },
        )
        .await;
        match result {
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
                error!("Proxy error: {}", ErrorChain(&e));
                StatusCode::BAD_GATEWAY.into_response()
            }
        }
    }
}

/// Answers 502 for a request that could not even be built.
fn bad_gateway(reason: &str) -> Response {
    error!("Cannot build Yahoo proxy request: {reason}");
    StatusCode::BAD_GATEWAY.into_response()
}

/// Upper bound on one Yahoo asset lookup (rate-limiter wait included), so a
/// stalled request cannot hold a synchronization open.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
impl AssetProvider for YahooProvider {
    /// Resolves an asset from the `meta` block of a one-day Yahoo chart.
    ///
    /// Recorded as one `lookup` provider call that includes the rate-limiter
    /// wait and the response body.
    async fn lookup(
        &self,
        asset: &AssetRef,
    ) -> std::result::Result<Option<ProviderAsset>, AssetProviderError> {
        observe_provider(
            MetricProvider::Yahoo,
            ProviderOp::Lookup,
            self.lookup_inner(asset),
            |result| match result {
                Ok(Some(_)) => ProviderOutcome::Ok,
                Ok(None) => ProviderOutcome::NotFound,
                Err(e) => outcome_of_lookup_error(e),
            },
        )
        .await
    }
}

impl YahooProvider {
    async fn lookup_inner(
        &self,
        asset: &AssetRef,
    ) -> std::result::Result<Option<ProviderAsset>, AssetProviderError> {
        let url = lookup_url(&self.endpoints, asset.symbol())?;
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

/// Describes a failed Yahoo request as a visible provider error.
fn call_error_of(error: &GetError) -> PriceCallError {
    match error {
        GetError::Request(e) if e.is_timeout() => {
            PriceCallError::timeout("Yahoo request timed out")
        }
        GetError::RateLimited => {
            PriceCallError::rate_limited("Yahoo rate limit reached; retry later")
        }
        GetError::Request(e) => {
            PriceFailure::provider_error(format!("Yahoo request failed: {}", ErrorChain(e))).into()
        }
    }
}

/// Extracts the most recent close from a Yahoo chart reply.
///
/// HTTP 404 and a `Not Found` chart error mean Yahoo lists no such symbol
/// (unavailable). Any other non-success status, or a chart error reported with
/// HTTP 200, is a provider error. An undecodable body or a reply without
/// `chart.result` or `indicators.quote` is malformed. A reply whose closes are
/// all null carries no price (unavailable).
fn parse_chart_price(status: u16, body: &[u8]) -> std::result::Result<f64, PriceFailure> {
    if status == 404 {
        return Err(PriceFailure::unavailable("Yahoo does not list the symbol"));
    }
    if !(200..300).contains(&status) {
        return Err(PriceFailure::provider_error(format!(
            "Yahoo answered with HTTP status {status}"
        )));
    }

    let response: chart::ChartResponse = serde_json::from_slice(body)
        .map_err(|e| PriceFailure::malformed(format!("undecodable chart body: {e}")))?;

    if let Some(error) = response.chart.error {
        return Err(if error.code.eq_ignore_ascii_case("not found") {
            PriceFailure::unavailable("Yahoo does not list the symbol")
        } else {
            PriceFailure::provider_error(format!(
                "Yahoo chart error {}: {}",
                error.code,
                error.description.unwrap_or_default()
            ))
        });
    }

    let result = response
        .chart
        .result
        .and_then(|results| results.into_iter().next())
        .ok_or_else(|| PriceFailure::malformed("empty chart.result"))?;
    let quote = result
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or_else(|| PriceFailure::malformed("empty chart.result.indicators.quote"))?;

    match quote {
        chart::QuotesKind::Empty {} => Err(PriceFailure::unavailable("chart has no quotes")),
        chart::QuotesKind::Quotes(q) => q
            .close
            .iter()
            .rev()
            .find_map(|p| *p)
            .ok_or_else(|| PriceFailure::unavailable("chart has no closing price")),
    }
}

/// Classifies a failed lookup for the provider metrics: request or overall
/// lookup timeouts are `timeout`, everything else is `error`.
fn outcome_of_lookup_error(error: &AssetProviderError) -> ProviderOutcome {
    let AssetProviderError::Unavailable(source) = error else {
        return ProviderOutcome::Error;
    };
    let timed_out = source.is::<tokio::time::error::Elapsed>()
        || matches!(
            source.downcast_ref::<GetError>(),
            Some(GetError::Request(e)) if e.is_timeout()
        );
    if timed_out {
        ProviderOutcome::Timeout
    } else {
        ProviderOutcome::Error
    }
}

/// Builds the one-day chart URL used to resolve an asset, percent-encoding the symbol.
fn lookup_url(
    endpoints: &YahooEndpoints,
    symbol: &str,
) -> std::result::Result<url::Url, AssetProviderError> {
    endpoints
        .chart_url(symbol, &[("range", "1d"), ("interval", "1d")])
        .map_err(AssetProviderError::Malformed)
}

/// Maximum number of assets one Yahoo search returns.
const SEARCH_RESULT_LIMIT: u32 = 20;

#[async_trait]
impl AssetSearchProvider for YahooProvider {
    /// Searches Yahoo's listings without waiting for the rate limiter: a saturated
    /// limiter is reported as [`AssetSearchError::RateLimited`].
    ///
    /// Recorded as one `lookup` provider call.
    async fn search_assets(&self, query: &str) -> Result<Vec<ListedAsset>, AssetSearchError> {
        observe_provider(
            MetricProvider::Yahoo,
            ProviderOp::Lookup,
            self.search_assets_inner(query),
            |result| match result {
                Ok(_) => ProviderOutcome::Ok,
                Err(SearchCallError::RateLimited) => ProviderOutcome::RateLimited,
                Err(SearchCallError::Call(e)) => PriceCallError::outcome::<()>(&Err(e.clone())),
            },
        )
        .await
        .map_err(|e| match e {
            SearchCallError::RateLimited => AssetSearchError::RateLimited,
            SearchCallError::Call(e) => AssetSearchError::Failed(e.failure),
        })
    }
}

/// Failure of one Yahoo search request.
enum SearchCallError {
    RateLimited,
    Call(PriceCallError),
}

impl YahooProvider {
    async fn search_assets_inner(&self, query: &str) -> Result<Vec<ListedAsset>, SearchCallError> {
        let mut url = self
            .endpoints
            .search_url(query)
            .map_err(|e| SearchCallError::Call(PriceFailure::malformed(e).into()))?;
        url.query_pairs_mut()
            .append_pair("quotesCount", &SEARCH_RESULT_LIMIT.to_string())
            .append_pair("newsCount", "0");

        let (status, body) = tokio::time::timeout(LIVE_QUOTE_TIMEOUT, async {
            let response = self
                .get(url.as_str(), LIVE_QUOTE_TIMEOUT, Admission::Reject)
                .await
                .map_err(|e| match e {
                    GetError::RateLimited => SearchCallError::RateLimited,
                    other => SearchCallError::Call(call_error_of(&other)),
                })?;
            let status = response.status().as_u16();
            let body = response
                .bytes()
                .await
                .map_err(|e| SearchCallError::Call(call_error_of(&GetError::Request(e))))?;
            Ok::<_, SearchCallError>((status, body))
        })
        .await
        .map_err(|_| SearchCallError::Call(PriceCallError::timeout("Yahoo search timed out")))??;

        parse_search_response(status, &body).map_err(|f| SearchCallError::Call(f.into()))
    }
}

/// Turns a Yahoo search reply into listed assets.
///
/// A non-success status is a provider error; an undecodable body or one without a
/// `quotes` list is malformed. Entries without a symbol are skipped. The asset class
/// comes from `quoteType` with the same conservative mapping as asset lookups.
fn parse_search_response(
    status: u16,
    body: &[u8],
) -> std::result::Result<Vec<ListedAsset>, PriceFailure> {
    if !(200..300).contains(&status) {
        return Err(PriceFailure::provider_error(format!(
            "Yahoo answered with HTTP status {status}"
        )));
    }

    let response: search::Response = serde_json::from_slice(body)
        .map_err(|e| PriceFailure::malformed(format!("undecodable search body: {e}")))?;
    let quotes = response
        .quotes
        .ok_or_else(|| PriceFailure::malformed("missing quotes list"))?;

    Ok(quotes
        .into_iter()
        .filter_map(|quote| {
            let symbol = non_blank(quote.symbol)?;
            Some(ListedAsset {
                symbol,
                name: non_blank(quote.longname).or_else(|| non_blank(quote.shortname)),
                venue: non_blank(quote.exch_disp).or_else(|| non_blank(quote.exchange)),
                asset_class: asset_class_of(quote.quote_type.as_deref()),
            })
        })
        .collect())
}

mod search {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub quotes: Option<Vec<Quote>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Quote {
        pub symbol: Option<String>,
        pub longname: Option<String>,
        pub shortname: Option<String>,
        pub exch_disp: Option<String>,
        pub exchange: Option<String>,
        pub quote_type: Option<String>,
    }
}

/// Upper bound on one live quote, rate-limiter wait included.
const LIVE_QUOTE_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
impl LivePriceProvider for YahooProvider {
    /// Reads the regular-market price from the `meta` block of a one-day chart.
    ///
    /// Recorded as one `price` provider call.
    async fn current_price(&self, symbol: &str) -> std::result::Result<LivePrice, PriceFailure> {
        observe_provider(
            MetricProvider::Yahoo,
            ProviderOp::Price,
            self.current_price_inner(symbol),
            PriceCallError::outcome,
        )
        .await
        .map_err(|e| e.failure)
    }
}

impl YahooProvider {
    /// Fetches a live quote without queueing for the shared limiter, so on-demand price
    /// requests never starve the price worker or lookups: a saturated limiter fails the
    /// quote immediately as a rate-limited provider error.
    async fn current_price_inner(
        &self,
        symbol: &str,
    ) -> std::result::Result<LivePrice, PriceCallError> {
        let url = self
            .endpoints
            .chart_url(symbol, &[("range", "1d"), ("interval", "1d")])
            .map_err(PriceFailure::malformed)?;
        let (status, body) = tokio::time::timeout(LIVE_QUOTE_TIMEOUT, async {
            let response = self
                .get(url.as_str(), LIVE_QUOTE_TIMEOUT, Admission::Reject)
                .await
                .map_err(|e| call_error_of(&e))?;
            let status = response.status().as_u16();
            let body = response
                .bytes()
                .await
                .map_err(|e| call_error_of(&GetError::Request(e)))?;
            Ok::<_, PriceCallError>((status, body.to_vec()))
        })
        .await
        .map_err(|_| PriceCallError::timeout("Yahoo quote timed out"))??;

        Ok(parse_live_price(status, &body)?)
    }
}

/// Reads the current price and currency from a Yahoo chart reply's `meta` block.
///
/// Unknown symbols (404, 400, 422 or a `Not Found` chart error) and a missing or
/// non-positive `regularMarketPrice` are unavailable; other statuses and chart
/// errors are provider errors; an undecodable reply, or one without a currency, is
/// malformed.
fn parse_live_price(status: u16, body: &[u8]) -> std::result::Result<LivePrice, PriceFailure> {
    if matches!(status, 400 | 404 | 422) {
        return Err(PriceFailure::unavailable("Yahoo does not list the symbol"));
    }
    if !(200..300).contains(&status) {
        return Err(PriceFailure::provider_error(format!(
            "Yahoo answered with HTTP status {status}"
        )));
    }

    let response: lookup::Response = serde_json::from_slice(body)
        .map_err(|e| PriceFailure::malformed(format!("undecodable chart body: {e}")))?;
    if let Some(error) = response.chart.error {
        return Err(if error.code.eq_ignore_ascii_case("not found") {
            PriceFailure::unavailable("Yahoo does not list the symbol")
        } else {
            PriceFailure::provider_error(format!(
                "Yahoo chart error {}: {}",
                error.code,
                error.description.unwrap_or_default()
            ))
        });
    }

    let meta = response
        .chart
        .result
        .and_then(|results| results.into_iter().next())
        .map(|result| result.meta)
        .ok_or_else(|| PriceFailure::malformed("empty chart.result"))?;
    let currency =
        non_blank(meta.currency).ok_or_else(|| PriceFailure::malformed("missing meta.currency"))?;
    let price = meta
        .regular_market_price
        .ok_or_else(|| PriceFailure::unavailable("Yahoo reports no market price"))
        .and_then(validate_price)?;

    Ok(LivePrice {
        price,
        currency,
        observed_at: meta
            .regular_market_time
            .and_then(|t| jiff::Timestamp::from_second(t).ok()),
    })
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
        pub regular_market_price: Option<f64>,
        pub regular_market_time: Option<i64>,
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    #[tokio::test]
    async fn lookup_timeouts_and_failures_map_to_provider_outcomes() {
        // GIVEN lookup failures of each kind, WHEN they are classified for the
        // provider metrics, THEN an elapsed lookup deadline is a timeout and
        // other failures (status errors, malformed data) are errors.
        let elapsed = tokio::time::timeout(Duration::ZERO, std::future::pending::<()>())
            .await
            .expect_err("a pending future must time out");
        assert_eq!(
            outcome_of_lookup_error(&AssetProviderError::Unavailable(Box::new(elapsed))),
            ProviderOutcome::Timeout
        );
        assert_eq!(
            outcome_of_lookup_error(&AssetProviderError::Unavailable(Box::new(
                GetError::RateLimited
            ))),
            ProviderOutcome::Error
        );
        assert_eq!(
            outcome_of_lookup_error(&AssetProviderError::Malformed("x".into())),
            ProviderOutcome::Error
        );
    }

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
        let url = lookup_url(&YahooEndpoints::default(), "A/B?C").unwrap();

        assert_eq!(
            url.as_str(),
            "https://query1.finance.yahoo.com/v8/finance/chart/A%2FB%3FC?range=1d&interval=1d"
        );
    }
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;
    use crate::app::infra::stats::{
        YAHOO_LIMITER_REJECTIONS_TOTAL, YAHOO_LIMITER_WAIT_SECONDS,
        YAHOO_UPSTREAM_RATE_LIMITS_TOTAL, test_metrics::TestRecorder,
    };
    use tokio::io::AsyncWriteExt;

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

    #[tokio::test(flavor = "current_thread")]
    async fn rejecting_get_only_fails_once_capacity_is_spent() {
        // GIVEN a limiter with a single slot,
        // WHEN two non-waiting requests are made,
        // THEN the first reaches the (refused) connection and the second is rate limited.
        let recorder = TestRecorder::default();
        let _recorder_guard = metrics::set_default_local_recorder(&recorder);
        let provider = single_request_provider();

        let first = provider
            .get(REFUSED_URL, REQUEST_TIMEOUT, Admission::Reject)
            .await;
        let second = provider
            .get(REFUSED_URL, REQUEST_TIMEOUT, Admission::Reject)
            .await;

        assert!(matches!(first, Err(GetError::Request(_))));
        assert!(matches!(second, Err(GetError::RateLimited)));
        assert_eq!(recorder.counter_value(YAHOO_LIMITER_REJECTIONS_TOTAL), 1);
        assert_eq!(recorder.counter_value(YAHOO_UPSTREAM_RATE_LIMITS_TOTAL), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn waiting_get_records_wait_duration_and_upstream_429_separately() {
        // GIVEN a spent local quota and a local HTTP server that answers 429,
        // WHEN an admitted Yahoo request waits for capacity and receives the reply,
        // THEN its wait is measured and the upstream limit counter increments once.
        let recorder = TestRecorder::default();
        let _recorder_guard = metrics::set_default_local_recorder(&recorder);
        let quota = Quota::with_period(Duration::from_millis(100)).unwrap();
        let provider = YahooProvider::with_quota(rquest::Client::new(), quota);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut request).await;
            socket
                .write_all(
                    b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });

        // Start the refill period only after the listener is ready to accept the request.
        provider.limiter.check().unwrap();
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            provider.get(
                &format!("http://{address}/"),
                REQUEST_TIMEOUT,
                Admission::Wait,
            ),
        )
        .await
        .expect("the limiter should refill promptly")
        .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let wait = recorder.histogram_values(YAHOO_LIMITER_WAIT_SECONDS);
        assert_eq!(wait.len(), 1);
        assert!(wait[0] >= 0.05, "expected limiter wait, got {}s", wait[0]);
        assert_eq!(recorder.counter_value(YAHOO_UPSTREAM_RATE_LIMITS_TOTAL), 1);
        assert_eq!(recorder.counter_value(YAHOO_LIMITER_REJECTIONS_TOTAL), 0);
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

/// Market id and Yahoo ticker of every fiat pair priced by Yahoo.
///
/// Each ticker is `{CCY}USD=X` and quotes the USD value of one unit of the
/// currency. Inverse and cross rates (`usdeur`, `eurgbp`) are not stored: the
/// conversion service derives them from these markets. Yahoo expects the
/// tickers in upper case.
pub const YAHOO_FX_TICKERS: [(&str, &str); 7] = [
    ("eurusd", "EURUSD=X"),
    ("gbpusd", "GBPUSD=X"),
    ("audusd", "AUDUSD=X"),
    ("cadusd", "CADUSD=X"),
    ("chfusd", "CHFUSD=X"),
    ("jpyusd", "JPYUSD=X"),
    ("aedusd", "AEDUSD=X"),
];

/// Returns the Yahoo ticker of a fiat market id, if Yahoo prices that pair.
pub fn yahoo_fx_ticker(market_id: &str) -> Option<&'static str> {
    YAHOO_FX_TICKERS
        .iter()
        .find(|(id, _)| *id == market_id)
        .map(|(_, ticker)| *ticker)
}

pub trait AsYahooMarket {
    /// Returns the Yahoo ticker of the market, or `None` when Yahoo does not
    /// price it (a fiat pair outside [`YAHOO_FX_TICKERS`]).
    fn as_yahoo(&self) -> Option<String>;
}

impl AsYahooMarket for Market {
    fn as_yahoo(&self) -> Option<String> {
        static MAPPINGS: LazyLock<HashMap<&'static str, &'static str>> =
            LazyLock::new(|| [("luna", "luna1")].into_iter().collect());

        if self.is_fiat() {
            return yahoo_fx_ticker(&self.id).map(str::to_string);
        }

        let base = MAPPINGS
            .get(self.base.id().as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.base.id().clone());

        let quote = MAPPINGS
            .get(self.quote.id().as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.quote.id().clone());

        Some(format!("{base}-{quote}"))
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
        #[serde(default)]
        pub description: Option<String>,
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

#[cfg(test)]
mod fx_tests {
    use super::*;
    use crate::app::domain::{
        entity::{Asset, Fiat},
        fiat::{FIAT_CATALOG, fiat_name},
    };

    fn fx_market(base: &str, quote: &str) -> Market {
        let fiat = |id: &str| Asset::Fiat(Fiat::new(id.into(), fiat_name(id).unwrap().into()));
        Market::new(format!("{base}{quote}"), fiat(base), fiat(quote), None)
    }

    #[test]
    fn every_catalog_fiat_except_usd_has_a_usd_ticker() {
        // GIVEN the fiat catalog and the hardcoded Yahoo FX tickers,
        // WHEN each non-USD currency is paired with USD,
        // THEN Yahoo prices the pair under an upper-case `{CCY}USD=X` ticker.
        for (symbol, _) in FIAT_CATALOG.iter().filter(|(symbol, _)| *symbol != "USD") {
            let id = format!("{}usd", symbol.to_lowercase());
            let ticker = yahoo_fx_ticker(&id).unwrap_or_else(|| panic!("no ticker for {id}"));
            assert_eq!(ticker, format!("{symbol}USD=X"));
            assert_eq!(
                fx_market(&symbol.to_lowercase(), "usd")
                    .as_yahoo()
                    .as_deref(),
                Some(ticker)
            );
        }
        assert_eq!(YAHOO_FX_TICKERS.len(), FIAT_CATALOG.len() - 1);
    }

    #[test]
    fn fiat_pairs_outside_the_table_have_no_ticker() {
        // GIVEN fiat pairs that are not stored (inverse and cross rates),
        // WHEN their Yahoo ticker is requested, THEN there is none.
        assert_eq!(fx_market("usd", "eur").as_yahoo(), None);
        assert_eq!(fx_market("eur", "gbp").as_yahoo(), None);
    }

    #[test]
    fn crypto_markets_keep_their_dash_tickers() {
        // GIVEN a crypto market, WHEN its Yahoo ticker is requested,
        // THEN the base and quote ids are joined with a dash.
        let btc = Asset::Crypto(crate::app::domain::entity::Crypto::new_with_id(
            "btc".into(),
        ));
        let eur = Asset::Fiat(Fiat::new("eur".into(), "Euro".into()));
        let market = Market::new("btceur".into(), btc, eur, None);

        assert_eq!(market.as_yahoo().as_deref(), Some("btc-eur"));
    }

    /// Live check against Yahoo: run with `cargo test -- --ignored live_eurusd`.
    #[tokio::test]
    #[ignore = "needs network access to Yahoo"]
    async fn live_eurusd_price_is_fetched_through_the_fx_ticker() {
        // GIVEN the real Yahoo endpoint and the EURUSD market,
        // WHEN its price is fetched,
        // THEN a plausible USD per EUR rate is returned.
        // Yahoo rejects clients without a browser-like user agent, as DcaServer configures.
        let mut headers = rquest::header::HeaderMap::new();
        headers.insert(
            rquest::header::USER_AGENT,
            rquest::header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            ),
        );
        let client = rquest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        let provider = YahooProvider::new(client);

        let price = provider
            .fetch_market_price(&fx_market("eur", "usd"), jiff::Timestamp::now())
            .await
            .expect("Yahoo should price EURUSD=X");

        println!("EURUSD=X -> {price}");
        assert!((0.5..2.0).contains(&price));
    }
}

#[cfg(test)]
mod price_tests {
    use super::*;
    use crate::{
        app::domain::entity::{Asset, Fiat},
        test_support::{FakeProvider, Reply},
    };

    fn eurusd() -> Market {
        let fiat = |id: &str| Asset::Fiat(Fiat::new(id.into(), id.to_uppercase()));
        Market::new("eurusd".into(), fiat("eur"), fiat("usd"), None)
    }

    fn provider_at(fake: &FakeProvider) -> YahooProvider {
        YahooProvider::new(rquest::Client::new()).with_base_url(&fake.base_url())
    }

    fn chart(closes: &str) -> String {
        format!(
            r#"{{"chart":{{"result":[{{"indicators":{{"quote":[{{"close":{closes}}}]}}}}],"error":null}}}}"#
        )
    }

    async fn price_of(fake: &FakeProvider) -> Result<f64, PriceFailure> {
        provider_at(fake)
            .fetch_market_price(&eurusd(), jiff::Timestamp::now())
            .await
    }

    /// GIVEN Yahoo answers a 5-minute chart whose latest close follows a null candle,
    /// WHEN the market price is fetched,
    /// THEN the latest non-null close is the price, requested for the encoded FX ticker.
    #[tokio::test]
    async fn latest_close_is_the_price() {
        let fake = FakeProvider::start(|_| Reply::ok(chart("[1.05,null,1.0725,null]"))).await;

        assert_eq!(price_of(&fake).await, Ok(1.0725));

        let requests = fake.requests();
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0].starts_with("/v8/finance/chart/EURUSD=X?"),
            "{requests:?}"
        );
        assert!(requests[0].contains("interval=5m"));
    }

    /// GIVEN Yahoo has no 5-minute close but a daily one,
    /// WHEN the market price is fetched,
    /// THEN the daily candle is used, from the same provider.
    #[tokio::test]
    async fn daily_candles_back_up_a_missing_five_minute_close() {
        let fake = FakeProvider::start(|target| {
            if target.contains("interval=1d") {
                Reply::ok(chart("[1.1]"))
            } else {
                Reply::ok(chart("[null]"))
            }
        })
        .await;

        assert_eq!(price_of(&fake).await, Ok(1.1));
        assert_eq!(fake.requests().len(), 2);
    }

    /// GIVEN Yahoo answers HTTP 200 carrying a chart error other than "Not Found",
    /// WHEN the market price is fetched,
    /// THEN it is a provider error (not an unavailable price) and no daily fallback is tried.
    #[tokio::test]
    async fn chart_error_in_a_successful_reply_is_a_provider_error() {
        let fake = FakeProvider::start(|_| {
            Reply::ok(r#"{"chart":{"result":null,"error":{"code":"Too Many Requests","description":"slow down"}}}"#)
        })
        .await;

        let failure = price_of(&fake).await.unwrap_err();

        assert_eq!(failure.kind, FailureKind::ProviderError);
        assert!(failure.message.contains("Too Many Requests"), "{failure}");
        assert_eq!(fake.requests().len(), 1);
    }

    /// GIVEN Yahoo does not list the ticker (HTTP 404, or a "Not Found" chart error),
    /// WHEN the market price is fetched,
    /// THEN the price is unavailable.
    #[tokio::test]
    async fn unlisted_tickers_are_unavailable() {
        let not_found =
            r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data"}}}"#;
        for reply in [Reply::status(404, not_found), Reply::ok(not_found)] {
            let fake = FakeProvider::start(move |_| reply.clone()).await;

            assert_eq!(
                price_of(&fake).await.unwrap_err().kind,
                FailureKind::Unavailable
            );
        }
    }

    /// GIVEN Yahoo answers with server errors, HTML, or an empty chart,
    /// WHEN the market price is fetched,
    /// THEN server errors are provider errors and unreadable payloads are malformed.
    #[tokio::test]
    async fn failures_and_malformed_replies_are_told_apart() {
        let cases = [
            (Reply::status(503, "down"), FailureKind::ProviderError),
            (Reply::status(429, ""), FailureKind::ProviderError),
            (Reply::ok("<html>"), FailureKind::Malformed),
            (
                Reply::ok(r#"{"chart":{"result":[],"error":null}}"#),
                FailureKind::Malformed,
            ),
            (
                Reply::ok(r#"{"chart":{"result":[{"indicators":{"quote":[]}}],"error":null}}"#),
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

    /// GIVEN a market Yahoo has no ticker for (a cross rate that is not stored),
    /// WHEN the market price is fetched,
    /// THEN it is unavailable without any request.
    #[tokio::test]
    async fn markets_without_a_ticker_never_reach_yahoo() {
        let fake = FakeProvider::start(|_| Reply::ok(chart("[1.0]"))).await;
        let fiat = |id: &str| Asset::Fiat(Fiat::new(id.into(), id.to_uppercase()));
        let cross = Market::new("eurgbp".into(), fiat("eur"), fiat("gbp"), None);

        let failure = provider_at(&fake)
            .fetch_market_price(&cross, jiff::Timestamp::now())
            .await
            .unwrap_err();

        assert_eq!(failure.kind, FailureKind::Unavailable);
        assert!(fake.requests().is_empty());
    }

    /// GIVEN a search text and a chart symbol with reserved URL characters,
    /// WHEN they are proxied,
    /// THEN the text is a percent-encoded query value and the symbol one encoded path segment.
    #[tokio::test]
    async fn proxied_search_and_chart_requests_are_percent_encoded() {
        let fake = FakeProvider::start(|_| Reply::ok("{}")).await;
        let provider = provider_at(&fake);

        provider.search("a b&c=d".to_string()).await;
        provider.chart("A/B?C".to_string(), 1, 2).await;

        let requests = fake.requests();
        assert_eq!(requests[0], "/v1/finance/search?q=a+b%26c%3Dd");
        assert!(
            requests[1].starts_with("/v8/finance/chart/A%2FB%3FC?period1=1&period2=2"),
            "{requests:?}"
        );
    }

    fn meta_chart(meta: &str) -> String {
        format!(r#"{{"chart":{{"result":[{{"meta":{meta}}}],"error":null}}}}"#)
    }

    /// GIVEN Yahoo answers chart metadata for an equity, a zero price, a reply without
    /// currency, and an unknown symbol,
    /// WHEN current prices are requested,
    /// THEN the first is a price in Yahoo's own currency spelling with its observation
    /// time, the zero price is unavailable, the reply without currency is malformed, and
    /// the unknown symbol is unavailable.
    #[tokio::test]
    async fn live_quotes_read_the_chart_metadata() {
        let fake = FakeProvider::start(|target| {
            if target.contains("/VOD.L") {
                Reply::ok(meta_chart(
                    r#"{"currency":"GBp","regularMarketPrice":71.5,"regularMarketTime":1700000000}"#,
                ))
            } else if target.contains("/ZERO") {
                Reply::ok(meta_chart(r#"{"currency":"USD","regularMarketPrice":0}"#))
            } else if target.contains("/NOCCY") {
                Reply::ok(meta_chart(r#"{"regularMarketPrice":3}"#))
            } else {
                Reply::status(404, "{}")
            }
        })
        .await;
        let provider = provider_at(&fake);

        let live = provider.current_price("VOD.L").await.unwrap();
        assert_eq!((live.price, live.currency.as_str()), (71.5, "GBp"));
        assert_eq!(live.observed_at.map(|t| t.as_second()), Some(1_700_000_000));
        assert_eq!(
            provider.current_price("ZERO").await.unwrap_err().kind,
            FailureKind::Unavailable
        );
        assert_eq!(
            provider.current_price("NOCCY").await.unwrap_err().kind,
            FailureKind::Malformed
        );
        assert_eq!(
            provider.current_price("NOPE").await.unwrap_err().kind,
            FailureKind::Unavailable
        );
        assert!(fake.requests()[0].ends_with("/VOD.L?range=1d&interval=1d"));
    }

    fn search_body(quotes: &str) -> String {
        format!(r#"{{"quotes":{quotes}}}"#)
    }

    /// GIVEN Yahoo answers a search with an ETF, an unnamed entry, and an entry
    /// without symbol,
    /// WHEN the assets are searched,
    /// THEN the listed assets carry symbol, name, venue and class, and the symbol-less
    /// entry is skipped; the query travels percent-encoded.
    #[tokio::test]
    async fn search_lists_typed_assets() {
        let fake = FakeProvider::start(|_| {
            Reply::ok(search_body(
                r#"[{"symbol":"VWCE.DE","longname":"Vanguard FTSE All-World","shortname":"VANGUARD","exchDisp":"XETRA","exchange":"GER","quoteType":"ETF"},
                    {"symbol":"BTC-USD","exchange":"CCC","quoteType":"CRYPTOCURRENCY"},
                    {"longname":"No symbol"}]"#,
            ))
        })
        .await;

        let found = provider_at(&fake).search_assets("vwce & co").await.unwrap();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].symbol, "VWCE.DE");
        assert_eq!(found[0].name.as_deref(), Some("Vanguard FTSE All-World"));
        assert_eq!(found[0].venue.as_deref(), Some("XETRA"));
        assert_eq!(found[0].asset_class, AssetClass::Equity);
        assert_eq!(
            (found[1].name.clone(), found[1].venue.as_deref()),
            (None, Some("CCC"))
        );
        assert_eq!(found[1].asset_class, AssetClass::Crypto);
        assert!(fake.requests()[0].starts_with("/v1/finance/search?q=vwce+%26+co&"));
    }

    /// GIVEN Yahoo answers with an empty list, a server error, and unreadable bodies,
    /// WHEN assets are searched,
    /// THEN an empty list is an answer, a server error is a provider failure, and
    /// bodies without a quotes list are malformed.
    #[tokio::test]
    async fn search_failures_are_told_apart() {
        let cases = [
            (Reply::ok(search_body("[]")), None),
            (Reply::status(503, "down"), Some(FailureKind::ProviderError)),
            (Reply::ok("<html>"), Some(FailureKind::Malformed)),
            (Reply::ok("{}"), Some(FailureKind::Malformed)),
        ];
        for (reply, expected) in cases {
            let fake = FakeProvider::start({
                let reply = reply.clone();
                move |_| reply.clone()
            })
            .await;

            let result = provider_at(&fake).search_assets("x").await;

            match expected {
                None => assert!(result.unwrap().is_empty()),
                Some(kind) => assert_eq!(
                    result.unwrap_err(),
                    AssetSearchError::Failed(PriceFailure::new(kind, result_message(&reply))),
                ),
            }
        }
    }

    /// The message the adapter is expected to give for a canned reply.
    fn result_message(reply: &Reply) -> String {
        match reply.status {
            200 if reply.body == "{}" => "missing quotes list".to_string(),
            200 => format!(
                "undecodable search body: {}",
                serde_json::from_str::<search::Response>(&reply.body).unwrap_err()
            ),
            status => format!("Yahoo answered with HTTP status {status}"),
        }
    }

    /// GIVEN a Yahoo limiter whose only slot is spent,
    /// WHEN assets are searched,
    /// THEN the search is rejected as rate limited without contacting Yahoo.
    #[tokio::test]
    async fn search_is_rejected_when_the_limiter_is_saturated() {
        let fake = FakeProvider::start(|_| Reply::ok(search_body("[]"))).await;
        let quota = Quota::per_hour(NonZeroU32::new(1).unwrap());
        let provider =
            YahooProvider::with_quota(rquest::Client::new(), quota).with_base_url(&fake.base_url());
        provider.limiter.check().unwrap();

        let result = provider.search_assets("x").await;

        assert_eq!(result.unwrap_err(), AssetSearchError::RateLimited);
        assert!(fake.requests().is_empty());
    }

    /// GIVEN a Yahoo limiter whose only slot is spent,
    /// WHEN a live quote is requested,
    /// THEN it fails at once as a provider error without contacting Yahoo, and the call is
    /// recorded as rate limited rather than as a provider timeout or error.
    #[tokio::test]
    async fn live_quotes_do_not_queue_for_the_limiter() {
        let fake = FakeProvider::start(|_| Reply::ok("{}")).await;
        let quota = Quota::per_hour(NonZeroU32::new(1).unwrap());
        let provider =
            YahooProvider::with_quota(rquest::Client::new(), quota).with_base_url(&fake.base_url());
        provider.limiter.check().unwrap();

        let started = std::time::Instant::now();
        let failure = provider.current_price("VWCE.DE").await.unwrap_err();
        let call = provider.current_price_inner("VWCE.DE").await;

        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(failure.kind, FailureKind::ProviderError);
        assert!(fake.requests().is_empty());
        let call = call.unwrap_err();
        assert!(call.rate_limited && !call.timed_out);
        assert_eq!(
            PriceCallError::outcome::<()>(&Err(call)),
            ProviderOutcome::RateLimited
        );
    }
}
