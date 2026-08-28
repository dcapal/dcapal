use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use crate::{AppContext, error::Result, ports::outbound::repository::StatsRepository};
use axum::{
    extract::{ConnectInfo, Request, State},
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
    let path = req.uri().path().to_string();
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
    let path = req.uri().path().to_string();
    if path == "/" {
        return Ok(next.run(req).await);
    }

    // Request stats
    counter!(REQUESTS_TOTAL, &[("path", path)]).increment(1);

    // Visitors stats
    record_visitors_stats(req.headers(), addr, state.repos.stats.clone()).await?;

    Ok(next.run(req).await)
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
