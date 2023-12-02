use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use hyper::HeaderMap;
use metrics::{histogram, increment_counter};
use tracing::{info, log::error};

use crate::{
    domain::ip2location::Ip2LocationService, error::Result, repository::StatsRepository, AppContext,
};

const BASE: &str = "dcapal";

pub const VISITORS_TOTAL: &str = concatcp!(BASE, '_', "visitors_total");
pub const REQUESTS_TOTAL: &str = concatcp!(BASE, '_', "requests_total");
pub const LATENCY_SUMMARY: &str = concatcp!(BASE, '_', "latency_summary");

pub async fn latency_stats(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    if path == "/" {
        return next.run(req).await;
    }

    let start = Instant::now();
    let res = next.run(req).await;

    let latency = start.elapsed();
    histogram!(
        LATENCY_SUMMARY,
        latency.as_micros() as f64,
        &[("path", path)]
    );

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
    increment_counter!(REQUESTS_TOTAL, &[("path", path)]);

    // Visitors stats
    record_visitors_stats(
        req.headers(),
        addr,
        state.repos.stats.clone(),
        state.services.ip2location.clone(),
    )
    .await?;

    Ok(next.run(req).await)
}

async fn record_visitors_stats(
    headers: &HeaderMap,
    addr: SocketAddr,
    repo: Arc<StatsRepository>,
    ip_service: Option<Arc<Ip2LocationService>>,
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
    repo.bump_visit(&ip_str).await?;

    let Some(ip_service) = ip_service else {
        return Ok(());
    };

    tokio::spawn(async move { fetch_geo_ip(ip, repo, ip_service).await });

    Ok(())
}

#[allow(dead_code)]
async fn fetch_geo_ip(ip: IpAddr, repo: Arc<StatsRepository>, ip_service: Arc<Ip2LocationService>) {
    if let Err(e) = fetch_geo_ip_inner(ip, repo, ip_service).await {
        error!("Error occurred in fetching GeoIP for {}: {:?}", ip, e);
    }
}

async fn fetch_geo_ip_inner(
    ip: IpAddr,
    repo: Arc<StatsRepository>,
    ip_service: Arc<Ip2LocationService>,
) -> Result<()> {
    if ip.is_loopback() {
        return Ok(());
    }

    let ip_str = ip.to_string();
    if let Some(geo) = repo.find_visitor_ip(&ip_str).await? {
        increment_counter!(
            VISITORS_TOTAL,
            &[
                ("ip", geo.ip),
                ("latitude", geo.latitude),
                ("longitude", geo.longitude),
            ]
        );
        return Ok(());
    }

    let Some(geo) = ip_service.lookup(ip) else {
        error!("Visitor IP not found ({ip})");
        return Ok(());
    };

    increment_counter!(
        VISITORS_TOTAL,
        &[
            ("ip", geo.ip.clone()),
            ("latitude", geo.latitude.clone()),
            ("longitude", geo.longitude.clone()),
        ]
    );

    let is_stored = repo.store_visitor_ip(&ip_str, &geo).await?;
    if !is_stored {
        info!("Visitor ip ({ip}) already exists");
    }

    Ok(())
}
