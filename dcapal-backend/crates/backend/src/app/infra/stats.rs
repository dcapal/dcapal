use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use crate::{AppContext, error::Result, ports::outbound::repository::StatsRepository};
use axum::{
    extract::{ConnectInfo, MatchedPath, Request, State},
    middleware::Next,
    response::Response,
};
use hyper::HeaderMap;
use metrics::{counter, histogram};

const BASE: &str = "dcapal";

/// Total number of visits recorded from non-loopback IP addresses.
pub const VISITORS_TOTAL: &str = concatcp!(BASE, '_', "visitors_total");
/// Total number of distinct IP addresses recorded as visitors.
pub const UNIQUE_VISITORS_TOTAL: &str = concatcp!(BASE, '_', "unique_visitors_total");
pub const REQUESTS_TOTAL: &str = concatcp!(BASE, '_', "requests_total");
pub const LATENCY_SUMMARY: &str = concatcp!(BASE, '_', "latency_summary");
pub const IMPORTED_PORTFOLIOS_TOTAL: &str = concatcp!(BASE, '_', "imported_portfolios_total");

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

pub async fn requests_stats(
    State(state): State<AppContext>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let path = metric_path(&req);
    if path == "/" {
        return Ok(next.run(req).await);
    }

    // Request stats
    counter!(REQUESTS_TOTAL, &[("path", path)]).increment(1);

    // Visitors stats
    record_visitors_stats(req.headers(), addr, state.repos.stats.clone()).await?;

    Ok(next.run(req).await)
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
}
