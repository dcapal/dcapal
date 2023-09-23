pub mod adapter;
pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod maintenance;
pub mod repository;
pub mod stats;

use crate::{
    config::Config,
    error::{DcaError, Result},
};

use adapter::{CryptoWatchProvider, IpApi, KrakenProvider, YahooProvider};
use axum::{
    extract::connect_info::IntoMakeServiceWithConnectInfo, middleware, routing::get, Router,
};
use chrono::prelude::*;
use deadpool_redis::{Pool, Runtime};
use domain::MarketDataService;
use futures::future::BoxFuture;
use hyper::Body;
use metrics::{describe_counter, Unit};
use repository::{market_data::MarketDataRepository, MiscRepository, StatsRepository};
use std::{
    net::{AddrParseError, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[macro_use]
extern crate const_format;

pub type DateTime = chrono::DateTime<Utc>;

pub struct RedisConfig {
    pub hostname: String,
    pub port: u32,
}

#[allow(dead_code)]
pub struct AppContextInner {
    config: Arc<Config>,
    http: reqwest::Client,
    redis: Pool,
    service: Arc<MarketDataService>,
    repos: Arc<Repository>,
    providers: Arc<Provider>,
}

pub type AppContext = Arc<AppContextInner>;

#[derive(Clone)]
struct Repository {
    pub misc: Arc<MiscRepository>,
    pub mkt_data: Arc<MarketDataRepository>,
    pub stats: Arc<StatsRepository>,
}

#[derive(Clone)]
pub struct Provider {
    pub cw: Arc<CryptoWatchProvider>,
    pub kraken: Arc<KrakenProvider>,
    pub yahoo: Arc<YahooProvider>,
    pub ipapi: Arc<IpApi>,
}

pub struct DcaServer {
    addr: SocketAddr,
    app: IntoMakeServiceWithConnectInfo<Router<(), Body>, SocketAddr>,
    ctx: AppContext,
    maintenance_handle: Option<JoinHandle<()>>,
    stop_tx: tokio::sync::watch::Sender<bool>,
}

impl DcaServer {
    pub fn try_new(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        let http = reqwest::Client::builder()
            .gzip(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .build()?;

        let redis = build_redis_pool(&config.server.redis)?;

        let repos = Arc::new(Repository {
            misc: Arc::new(MiscRepository::new(redis.clone())),
            mkt_data: Arc::new(MarketDataRepository::new(redis.clone())),
            stats: Arc::new(StatsRepository::new(redis.clone())),
        });

        let providers = Arc::new(Provider {
            cw: Arc::new(CryptoWatchProvider::new(
                http.clone(),
                &config.app.providers,
            )),
            kraken: Arc::new(KrakenProvider::new(http.clone(), &config.app.providers)),
            yahoo: Arc::new(YahooProvider::new(http.clone())),
            ipapi: Arc::new(IpApi::new(http.clone(), &config.app.providers)),
        });

        let service = Arc::new(MarketDataService::new(
            config.clone(),
            repos.mkt_data.clone(),
            providers.clone(),
        ));

        let ctx = Arc::new(AppContextInner {
            config: config.clone(),
            http,
            redis,
            service,
            repos,
            providers,
        });

        let app = Router::new()
            .route("/", get(|| async { "Greetings from DCA-Pal APIs!" }))
            .route("/assets/fiat", get(api::get_assets_fiat))
            .route("/assets/crypto", get(api::get_assets_crypto))
            .route("/price/:asset", get(api::get_price))
            .route_layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(middleware::from_fn_with_state(
                        ctx.clone(),
                        stats::requests_stats,
                    ))
                    .layer(middleware::from_fn(stats::latency_stats)),
            )
            .with_state(ctx.clone())
            .into_make_service_with_connect_info();

        let (hostname, port) = (&config.server.web.hostname, config.server.web.port);
        let addr = format!("{}:{}", hostname, port)
            .parse()
            .map_err(|e: AddrParseError| {
                DcaError::StartupFailure(
                    format!("Invalid hostname ({}) or port ({})", hostname, port),
                    e.into(),
                )
            })?;

        let (stop_tx, _) = tokio::sync::watch::channel(false);
        Ok(Self {
            addr,
            app,
            ctx,
            maintenance_handle: None,
            stop_tx,
        })
    }

    pub async fn start(&mut self, signal_handler: BoxFuture<'_, ()>) -> Result<()> {
        info!("Initializing metrics");
        self.init_metrics();

        info!("Starting Maintenance task");
        {
            let ctx = self.ctx.clone();
            let stop_rx = self.stop_tx.subscribe();
            let handle = tokio::spawn(async move {
                maintenance::run(ctx, stop_rx).await;
            });
            self.maintenance_handle.replace(handle);
        }

        info!("Starting DcaServer at {}", &self.addr);
        axum::Server::bind(&self.addr)
            .serve(self.app.clone())
            .with_graceful_shutdown(signal_handler)
            .await
            .map_err(|e| DcaError::StartupFailure("Failed to start DcaServer".into(), e.into()))
    }

    pub fn init_metrics(&self) {
        describe_counter!(stats::VISITORS_TOTAL, Unit::Count, "Number of API visitors");
        describe_counter!(
            stats::REQUESTS_TOTAL,
            Unit::Count,
            "Number of requests processed"
        );
        describe_counter!(
            stats::LATENCY_SUMMARY,
            Unit::Microseconds,
            "Summary of endpoint response time"
        );
    }
}

fn build_redis_pool(config: &config::Redis) -> Result<deadpool_redis::Pool> {
    let url = config.connection_url();
    let redis_pool = deadpool_redis::Config::from_url(url)
        .builder()
        .map_err(|e| {
            DcaError::StartupFailure(
                format!(
                    "Error in building Redis poll config (user={}, hostname={}, port={})",
                    &config.user, &config.hostname, &config.port
                ),
                e.into(),
            )
        })?
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| {
            DcaError::StartupFailure("Failed to build Redis connection pool".into(), e.into())
        })?;

    Ok(redis_pool)
}
