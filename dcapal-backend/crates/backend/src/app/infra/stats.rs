use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{AppContext, error::Result, ports::outbound::repository::StatsRepository};
use axum::{
    extract::{ConnectInfo, MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use hyper::HeaderMap;
use metrics::{counter, histogram};
use strum_macros::IntoStaticStr;

const BASE: &str = "dcapal";

/// Total number of visits recorded from non-loopback IP addresses.
pub const VISITORS_TOTAL: &str = concatcp!(BASE, '_', "visitors_total");
/// Total number of distinct IP addresses recorded as visitors.
pub const UNIQUE_VISITORS_TOTAL: &str = concatcp!(BASE, '_', "unique_visitors_total");
/// Requests processed, labelled by route template (`path`) and response status
/// class (`status`: `2xx`, `4xx`, `5xx` or `other`).
pub const REQUESTS_TOTAL: &str = concatcp!(BASE, '_', "requests_total");
pub const LATENCY_SUMMARY: &str = concatcp!(BASE, '_', "latency_summary");
pub const IMPORTED_PORTFOLIOS_TOTAL: &str = concatcp!(BASE, '_', "imported_portfolios_total");

/// Outbound provider calls, labelled by `provider`, `op` and `outcome`.
pub const PROVIDER_REQUESTS_TOTAL: &str = concatcp!(BASE, '_', "provider_requests_total");
/// Time in seconds spent waiting for Yahoo request capacity.
pub const YAHOO_LIMITER_WAIT_SECONDS: &str = concatcp!(BASE, '_', "yahoo_limiter_wait_seconds");
/// Yahoo proxy calls rejected locally because the shared limiter was full.
pub const YAHOO_LIMITER_REJECTIONS_TOTAL: &str =
    concatcp!(BASE, '_', "yahoo_limiter_rejections_total");
/// HTTP 429 responses received from Yahoo.
pub const YAHOO_UPSTREAM_RATE_LIMITS_TOTAL: &str =
    concatcp!(BASE, '_', "yahoo_upstream_rate_limits_total");
/// Duration in seconds of outbound provider calls, labelled by `provider` and
/// `op` (deliberately without `outcome`).
pub const PROVIDER_REQUEST_SECONDS: &str = concatcp!(BASE, '_', "provider_request_seconds");
/// Duration in seconds of the v2 synchronization stages, labelled by `stage`.
pub const SYNC_STAGE_SECONDS: &str = concatcp!(BASE, '_', "sync_stage_seconds");
/// Notable v2 synchronization events, labelled by `event`.
pub const SYNC_EVENTS_TOTAL: &str = concatcp!(BASE, '_', "sync_events_total");

/// External provider behind an outbound call (`provider` label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum MetricProvider {
    Yahoo,
    Kraken,
}

/// Kind of outbound provider call (`op` label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ProviderOp {
    /// Asset metadata or directory resolution.
    Lookup,
    /// Market price retrieval.
    Price,
    /// Pass-through of a user-facing search or chart request.
    Proxy,
}

/// Result of an outbound provider call (`outcome` label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ProviderOutcome {
    Ok,
    /// The provider does not know the requested symbol or market.
    NotFound,
    /// Our limiter rejected the call, or the provider answered HTTP 429.
    RateLimited,
    Timeout,
    Error,
}

/// Records time spent waiting for an available Yahoo request slot.
pub fn record_yahoo_limiter_wait(elapsed: Duration) {
    histogram!(YAHOO_LIMITER_WAIT_SECONDS).record(elapsed.as_secs_f64());
}

/// Records a Yahoo request rejected by the local rate limiter.
pub fn record_yahoo_limiter_rejection() {
    counter!(YAHOO_LIMITER_REJECTIONS_TOTAL).increment(1);
}

/// Records an HTTP 429 response received from Yahoo, ignoring other replies.
pub fn record_yahoo_upstream_response(status: StatusCode) {
    if status == StatusCode::TOO_MANY_REQUESTS {
        counter!(YAHOO_UPSTREAM_RATE_LIMITS_TOTAL).increment(1);
    }
}

#[cfg(test)]
pub(crate) mod test_metrics {
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };

    use metrics::{
        Counter, CounterFn, Gauge, Histogram, HistogramFn, Key, KeyName, Metadata, Recorder,
        SharedString, Unit,
    };

    use super::*;

    #[derive(Default)]
    pub(crate) struct TestRecorder {
        counters: Mutex<HashMap<String, Arc<TestCounter>>>,
        histograms: Mutex<HashMap<String, Arc<TestHistogram>>>,
    }

    #[derive(Default)]
    struct TestCounter(AtomicU64);

    #[derive(Default)]
    struct TestHistogram(Mutex<Vec<f64>>);

    impl Recorder for TestRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}

        fn register_counter(&self, key: &Key, _: &Metadata<'_>) -> Counter {
            let metric = self
                .counters
                .lock()
                .unwrap()
                .entry(key.name().to_owned())
                .or_default()
                .clone();
            Counter::from_arc(metric)
        }

        fn register_gauge(&self, _: &Key, _: &Metadata<'_>) -> Gauge {
            Gauge::noop()
        }

        fn register_histogram(&self, key: &Key, _: &Metadata<'_>) -> Histogram {
            let metric = self
                .histograms
                .lock()
                .unwrap()
                .entry(key.name().to_owned())
                .or_default()
                .clone();
            Histogram::from_arc(metric)
        }
    }

    impl CounterFn for TestCounter {
        fn increment(&self, value: u64) {
            self.0.fetch_add(value, Ordering::Relaxed);
        }

        fn absolute(&self, value: u64) {
            self.0.fetch_max(value, Ordering::Relaxed);
        }
    }

    impl HistogramFn for TestHistogram {
        fn record(&self, value: f64) {
            self.0.lock().unwrap().push(value);
        }
    }

    impl TestRecorder {
        pub(crate) fn counter_value(&self, name: &str) -> u64 {
            self.counters
                .lock()
                .unwrap()
                .get(name)
                .map_or(0, |counter| counter.0.load(Ordering::Relaxed))
        }

        pub(crate) fn histogram_values(&self, name: &str) -> Vec<f64> {
            self.histograms
                .lock()
                .unwrap()
                .get(name)
                .map_or_else(Vec::new, |histogram| histogram.0.lock().unwrap().clone())
        }
    }

    #[test]
    fn yahoo_limiter_metric_recorders_capture_the_dedicated_signals() {
        // GIVEN an isolated metrics recorder,
        // WHEN Yahoo limiter signals are emitted,
        // THEN each signal is stored under its own metric family.
        let recorder = TestRecorder::default();
        metrics::with_local_recorder(&recorder, || {
            record_yahoo_limiter_wait(Duration::from_millis(250));
            record_yahoo_limiter_rejection();
            record_yahoo_upstream_response(StatusCode::TOO_MANY_REQUESTS);
            record_yahoo_upstream_response(StatusCode::OK);
        });

        assert_eq!(
            recorder.histogram_values(YAHOO_LIMITER_WAIT_SECONDS),
            vec![0.25]
        );
        assert_eq!(recorder.counter_value(YAHOO_LIMITER_REJECTIONS_TOTAL), 1);
        assert_eq!(recorder.counter_value(YAHOO_UPSTREAM_RATE_LIMITS_TOTAL), 1);
    }
}

impl ProviderOutcome {
    /// Classifies an HTTP status answered by a provider: 404 is `not_found`,
    /// 429 is `rate_limited`, other 2xx/3xx are `ok`, everything else is `error`.
    pub fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited,
            s if s.is_success() || s.is_redirection() => Self::Ok,
            _ => Self::Error,
        }
    }
}

/// Records one provider call: the `outcome` counter and the duration histogram.
pub fn record_provider_request(
    provider: MetricProvider,
    op: ProviderOp,
    outcome: ProviderOutcome,
    elapsed: Duration,
) {
    let (provider, op): (&'static str, &'static str) = (provider.into(), op.into());
    counter!(
        PROVIDER_REQUESTS_TOTAL,
        "provider" => provider,
        "op" => op,
        "outcome" => <&'static str>::from(outcome)
    )
    .increment(1);
    histogram!(PROVIDER_REQUEST_SECONDS, "provider" => provider, "op" => op)
        .record(elapsed.as_secs_f64());
}

/// Runs a provider call and records its outcome and duration.
///
/// The duration covers the whole future, including any wait for a rate limiter.
/// `classify` maps the call's result to its outcome.
pub async fn observe_provider<T>(
    provider: MetricProvider,
    op: ProviderOp,
    call: impl Future<Output = T>,
    classify: impl FnOnce(&T) -> ProviderOutcome,
) -> T {
    let start = Instant::now();
    let result = call.await;
    record_provider_request(provider, op, classify(&result), start.elapsed());
    result
}

/// Stage of a v2 synchronization (`stage` label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SyncStage {
    /// Resolving asset references against the directory and providers.
    Resolve,
    /// Writing resolved assets to the directory.
    Onboard,
    /// Loading the stored portfolio snapshot.
    Load,
    /// Applying the computed plan transactionally.
    Apply,
}

impl SyncStage {
    /// Starts timing this stage; the duration is recorded when the guard drops,
    /// including on early returns and errors.
    pub fn timer(self) -> SyncStageTimer {
        SyncStageTimer {
            stage: self,
            start: Instant::now(),
        }
    }
}

/// RAII guard recording a [`SyncStage`] duration into [`SYNC_STAGE_SECONDS`] on drop.
#[must_use = "the stage duration is recorded when the timer is dropped"]
pub struct SyncStageTimer {
    stage: SyncStage,
    start: Instant,
}

impl Drop for SyncStageTimer {
    fn drop(&mut self) {
        let stage: &'static str = self.stage.into();
        histogram!(SYNC_STAGE_SECONDS, "stage" => stage).record(self.start.elapsed().as_secs_f64());
    }
}

/// Notable v2 synchronization event (`event` label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SyncEvent {
    /// A stale snapshot made the plan be recomputed (one per actual retry).
    StaleRetry,
    /// Retries were exhausted and the synchronization failed with a conflict.
    Conflict,
    /// The plan had no writes or tombstones, so nothing was applied.
    PullOnly,
    /// Unknown references that a provider resolved (incremented by their count).
    NewAssetsResolved,
    /// Provider refresh lookups started for client-supplied rows (incremented by their count).
    RefreshAttempted,
    /// A resolution was refused for referencing too many unknown assets.
    TooManyNewAssets,
}

/// Increments [`SYNC_EVENTS_TOTAL`] for `event` by `count`.
pub fn record_sync_event(event: SyncEvent, count: u64) {
    let event: &'static str = event.into();
    counter!(SYNC_EVENTS_TOTAL, "event" => event).increment(count);
}

/// Maps a response status to its low-cardinality class label.
pub fn status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        200..=299 => "2xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    }
}

pub async fn latency_stats(req: Request, next: Next) -> Response {
    let path = metric_path(&req);
    if path == "/" {
        return next.run(req).await;
    }

    let start = Instant::now();
    let res = next.run(req).await;

    let latency = start.elapsed();
    histogram!(LATENCY_SUMMARY, &[("path", path)]).record(latency.as_micros() as f64);

    res
}

/// Counts requests per route template and response status class and records
/// visitor statistics.
///
/// The counter is incremented after the response is known. A failure of the
/// visitor bookkeeping aborts the request with the error's response, which is
/// counted with its own status class.
pub async fn requests_stats(
    State(state): State<AppContext>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let path = metric_path(&req);
    if path == "/" {
        return next.run(req).await;
    }

    // Visitors stats
    let response = match record_visitors_stats(req.headers(), addr, state.repos.stats.clone()).await
    {
        Ok(()) => next.run(req).await,
        Err(e) => e.into_response(),
    };

    counter!(
        REQUESTS_TOTAL,
        "path" => path,
        "status" => status_class(response.status())
    )
    .increment(1);

    response
}

/// Returns the matched route template, or the request path when no template is available.
fn metric_path(req: &Request) -> String {
    req.extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| req.uri().path())
        .to_owned()
}

async fn record_visitors_stats(
    headers: &HeaderMap,
    addr: SocketAddr,
    repo: Arc<StatsRepository>,
) -> Result<()> {
    static IP_HEADERS: [&str; 2] = ["CF-Connecting-IP", "X-Real-IP"];
    static FALLBACK_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    let ip = IP_HEADERS
        .iter()
        .find_map(|header| {
            headers.get(*header).map(|h| {
                h.to_str()
                    .map(|h| h.parse::<IpAddr>().unwrap_or(FALLBACK_IP))
                    .unwrap_or(FALLBACK_IP)
            })
        })
        .unwrap_or(addr.ip());

    if ip.is_loopback() {
        return Ok(());
    }

    let ip_str = ip.to_string();
    let visit_count = repo.bump_visit(&ip_str).await?;
    counter!(VISITORS_TOTAL).increment(1);
    if visit_count == 1 {
        counter!(UNIQUE_VISITORS_TOTAL).increment(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::Response,
        routing::get,
    };
    use tower::ServiceExt;

    use super::*;

    async fn expose_metric_path(req: Request<Body>, next: Next) -> Response {
        let path = metric_path(&req);
        let mut response = next.run(req).await;
        response.headers_mut().insert(
            "x-metric-path",
            path.parse().expect("metric paths are valid header values"),
        );
        response
    }

    async fn matched_metric_path(route: &'static str, request_path: &'static str) -> String {
        let app = Router::new()
            .route(route, get(|| async {}))
            .route_layer(middleware::from_fn(expose_metric_path));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(request_path)
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        response
            .headers()
            .get("x-metric-path")
            .expect("middleware should expose the metric path")
            .to_str()
            .expect("metric path should be valid UTF-8")
            .to_owned()
    }

    #[tokio::test]
    async fn uses_route_templates_for_dynamic_endpoints() {
        // GIVEN requests to parameterized endpoints, WHEN Axum provides their
        // matched paths, THEN metrics use the route templates.
        for (route, request_path) in [
            ("/assets/chart/{symbol}", "/assets/chart/BTC-USD"),
            ("/price/{asset}", "/price/EUR"),
            ("/import/portfolio/{id}", "/import/portfolio/42"),
        ] {
            assert_eq!(matched_metric_path(route, request_path).await, route);
        }
    }

    #[tokio::test]
    async fn uses_static_matched_paths() {
        // GIVEN a request to a static endpoint, WHEN its route is matched,
        // THEN the static route path is used for metrics.
        assert_eq!(
            matched_metric_path("/health", "/health?check=ready").await,
            "/health"
        );
    }

    #[test]
    fn falls_back_to_the_request_path_without_a_matched_path() {
        // GIVEN a request without a MatchedPath extension, WHEN its metric path
        // is resolved, THEN the URI path is used as the fallback.
        let request = Request::builder()
            .uri("/not-routed?check=ready")
            .body(Body::empty())
            .expect("request should be valid");

        assert_eq!(metric_path(&request), "/not-routed");
    }

    #[tokio::test]
    async fn excludes_the_root_path_from_latency_metrics() {
        // GIVEN a request for the root endpoint, WHEN latency middleware runs,
        // THEN it passes through without requiring a metrics recorder.
        let app = Router::new()
            .route("/", get(|| async { StatusCode::NO_CONTENT }))
            .layer(middleware::from_fn(latency_stats));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    /// Renders the Prometheus text produced by `emit` against a local recorder
    /// configured like the production exporter.
    fn render(emit: impl FnOnce()) -> String {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, emit);
        handle.render()
    }

    #[test]
    fn maps_status_codes_to_classes() {
        // GIVEN response statuses of every class, WHEN they are mapped,
        // THEN 2xx, 4xx and 5xx keep their class and the rest is "other".
        assert_eq!(status_class(StatusCode::OK), "2xx");
        assert_eq!(status_class(StatusCode::NO_CONTENT), "2xx");
        assert_eq!(status_class(StatusCode::NOT_FOUND), "4xx");
        assert_eq!(status_class(StatusCode::TOO_MANY_REQUESTS), "4xx");
        assert_eq!(status_class(StatusCode::BAD_GATEWAY), "5xx");
        assert_eq!(status_class(StatusCode::SWITCHING_PROTOCOLS), "other");
        assert_eq!(status_class(StatusCode::FOUND), "other");
    }

    #[test]
    fn maps_provider_http_statuses_to_outcomes() {
        // GIVEN HTTP statuses answered by a provider, WHEN they are classified,
        // THEN 404 is not_found, 429 is rate_limited and failures are errors.
        use ProviderOutcome::*;
        assert_eq!(ProviderOutcome::from_status(StatusCode::OK), Ok);
        assert_eq!(
            ProviderOutcome::from_status(StatusCode::NOT_FOUND),
            NotFound
        );
        assert_eq!(
            ProviderOutcome::from_status(StatusCode::TOO_MANY_REQUESTS),
            RateLimited
        );
        assert_eq!(
            ProviderOutcome::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            Error
        );
    }

    #[test]
    fn label_values_are_snake_case() {
        // GIVEN the label enums, WHEN they are converted to strings, THEN the
        // values match the documented label vocabulary.
        assert_eq!(<&str>::from(ProviderOutcome::NotFound), "not_found");
        assert_eq!(<&str>::from(ProviderOutcome::RateLimited), "rate_limited");
        assert_eq!(<&str>::from(MetricProvider::Kraken), "kraken");
        assert_eq!(<&str>::from(ProviderOp::Proxy), "proxy");
        assert_eq!(
            <&str>::from(SyncEvent::NewAssetsResolved),
            "new_assets_resolved"
        );
        assert_eq!(
            <&str>::from(SyncEvent::TooManyNewAssets),
            "too_many_new_assets"
        );
        assert_eq!(<&str>::from(SyncStage::Onboard), "onboard");
    }

    #[test]
    fn exposes_new_families_with_labels_and_summaries() {
        // GIVEN a recorder configured like the exporter, WHEN a provider call,
        // a sync stage and sync events are recorded, THEN the rendered text has
        // the counters with their labels and the duration families as summaries
        // (quantile lines plus `_sum` and `_count`, no `_bucket` lines), and no
        // outcome label on the provider duration.
        let text = render(|| {
            record_provider_request(
                MetricProvider::Yahoo,
                ProviderOp::Lookup,
                ProviderOutcome::NotFound,
                Duration::from_millis(30),
            );
            drop(SyncStage::Load.timer());
            record_sync_event(SyncEvent::NewAssetsResolved, 3);
            counter!(
                REQUESTS_TOTAL,
                "path" => "/health",
                "status" => status_class(StatusCode::OK)
            )
            .increment(1);
            histogram!(LATENCY_SUMMARY, "path" => "/health").record(1500.0);
        });

        assert!(text.contains(
            "dcapal_provider_requests_total{provider=\"yahoo\",op=\"lookup\",outcome=\"not_found\"} 1"
        ), "{text}");
        for q in ["0", "0.5", "0.9", "0.95", "0.99", "0.999", "1"] {
            let provider = format!(
                "dcapal_provider_request_seconds{{provider=\"yahoo\",op=\"lookup\",quantile=\"{q}\"}}"
            );
            assert!(text.contains(&provider), "{provider}\n{text}");
            let stage = format!("dcapal_sync_stage_seconds{{stage=\"load\",quantile=\"{q}\"}}");
            assert!(text.contains(&stage), "{stage}\n{text}");
        }
        for line in [
            "dcapal_provider_request_seconds_count{provider=\"yahoo\",op=\"lookup\"} 1",
            "dcapal_sync_stage_seconds_count{stage=\"load\"} 1",
        ] {
            assert!(text.contains(line), "{line}\n{text}");
        }
        for prefix in [
            "dcapal_provider_request_seconds_sum{provider=\"yahoo\",op=\"lookup\"}",
            "dcapal_sync_stage_seconds_sum{stage=\"load\"}",
        ] {
            assert!(text.contains(prefix), "{prefix}\n{text}");
        }
        assert!(!text.contains("_bucket"), "{text}");
        assert!(
            text.contains("dcapal_sync_events_total{event=\"new_assets_resolved\"} 3"),
            "{text}"
        );
        assert!(
            text.contains("dcapal_requests_total{path=\"/health\",status=\"2xx\"} 1"),
            "{text}"
        );
        // The pre-existing latency metric is untouched: still a summary.
        assert!(
            text.contains("dcapal_latency_summary{path=\"/health\",quantile=\"0.5\"}"),
            "{text}"
        );
        assert!(!text.contains("dcapal_latency_summary_bucket"), "{text}");
    }

    #[tokio::test]
    async fn observe_provider_records_the_classified_outcome() {
        // GIVEN a provider call that fails, WHEN it is observed, THEN its result
        // is returned unchanged and one error outcome and duration are recorded.
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        let result = {
            let _guard = metrics::set_default_local_recorder(&recorder);
            observe_provider(
                MetricProvider::Kraken,
                ProviderOp::Price,
                async { Err::<(), _>("boom") },
                |r| match r {
                    Ok(_) => ProviderOutcome::Ok,
                    Err(_) => ProviderOutcome::Error,
                },
            )
            .await
        };

        assert_eq!(result, Err("boom"));
        let text = handle.render();
        assert!(text.contains(
            "dcapal_provider_requests_total{provider=\"kraken\",op=\"price\",outcome=\"error\"} 1"
        ), "{text}");
    }
}
