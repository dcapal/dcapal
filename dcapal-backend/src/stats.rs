use std::{net::SocketAddr, sync::Arc, time::Instant};

use axum::{
    extract::{ConnectInfo, State},
    middleware::Next,
    response::Response,
};
use hyper::Request;
use metrics::{histogram, increment_counter};
use tracing::log::error;

use crate::{adapter::IpApi, error::Result, repository::StatsRepository, AppContext};

const BASE: &str = "dcapal";

pub const VISITORS_TOTAL: &str = concatcp!(BASE, '_', "visitors_total");
pub const REQUESTS_TOTAL: &str = concatcp!(BASE, '_', "requests_total");
pub const LATENCY_SUMMARY: &str = concatcp!(BASE, '_', "latency_summary");

pub async fn latency_stats<B>(req: Request<B>, next: Next<B>) -> Response {
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

pub async fn requests_stats<B>(
    State(state): State<AppContext>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    let path = req.uri().path().to_string();
    if path == "/" {
        return Ok(next.run(req).await);
    }

    // Request stats
    increment_counter!(REQUESTS_TOTAL, &[("path", path)]);

    // Visitors stats
    let repo = &state.repos.stats;

    let ip = if let Some(header) = req.headers().get("X-Real-IP") {
        header
            .to_str()
            .map(|h| h.to_string())
            .unwrap_or_else(|_| addr.ip().to_string())
    } else {
        addr.ip().to_string()
    };

    repo.bump_visit(&ip).await?;

    let repo = repo.clone();
    let ipapi = state.providers.ipapi.clone();
    tokio::spawn(async move { fetch_geo_ip(ip, repo, ipapi).await });

    Ok(next.run(req).await)
}

async fn fetch_geo_ip(ip: String, repo: Arc<StatsRepository>, ipapi: Arc<IpApi>) {
    if let Err(e) = fetch_geo_ip_inner(&ip, repo, ipapi).await {
        error!(
            "Error occurred in fetching GeoIP for {}: {}",
            ip,
            e.msg_chain()
        );
    }
}

async fn fetch_geo_ip_inner(ip: &str, repo: Arc<StatsRepository>, ipapi: Arc<IpApi>) -> Result<()> {
    let geo = repo.find_visitor_ip(ip).await?;
    if geo.is_some() {
        return Ok(());
    }

    let geo = ipapi.fetch_geo(ip).await?;

    if geo.is_none() {
        error!("Failed to fetch visitor ip ({}) from IpApi", ip);
        return Ok(());
    }

    let geo = geo.unwrap();
    increment_counter!(
        VISITORS_TOTAL,
        &[
            ("ip", geo.ip.clone()),
            ("latitude", geo.latitude.clone()),
            ("longitude", geo.longitude.clone()),
            ("city", geo.city.clone()),
            ("country_code", geo.country_code.clone()),
            ("country_name", geo.country_name.clone())
        ]
    );

    let is_stored = repo.store_visitor_ip(ip, &geo).await?;
    if !is_stored {
        error!("Failed to store visitor ip ({}) from IpApi", ip);
    }

    Ok(())
}
