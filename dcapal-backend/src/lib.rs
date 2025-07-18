#[macro_use]
extern crate const_format;

use std::{
    net::{AddrParseError, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{
    Router,
    extract::connect_info::IntoMakeServiceWithConnectInfo,
    middleware,
    routing::{get, post},
};
use chrono::prelude::*;
use deadpool_redis::{Pool, Runtime};
use futures::future::BoxFuture;
use hyper::header;
use metrics::{Unit, counter, describe_counter, describe_histogram};
use sea_orm::{
    sqlx,
    sqlx::{PgPool, postgres::PgPoolOptions},
};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use crate::{
    app::{
        infra,
        services::{
            ip2location::Ip2LocationService, market_data::MarketDataService,
            portfolio::PortfolioService,
        },
        workers::{market_discovery::MarketDiscoveryWorker, price_updater::PriceUpdaterWorker},
    },
    config::{Config, Postgres},
    error::{DcaError, Result},
    ports::{
        inbound::rest,
        outbound::{
            adapter::{CryptoWatchProvider, IpApi, KrakenProvider, PriceProviders, YahooProvider},
            repository::{
                ImportedRepository, MiscRepository, StatsRepository,
                market_data::MarketDataRepository, portfolio::PortfolioRepository,
                user::UserRepository,
            },
        },
    },
};

pub mod app;
pub mod config;
pub mod error;
pub mod ports;

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
    postgres: PgPool,
    services: Services,
    repos: Arc<Repository>,
    providers: Arc<PriceProviders>,
}

pub type AppContext = Arc<AppContextInner>;

#[derive(Clone)]
struct Services {
    mkt_data: Arc<MarketDataService>,
    ip2location: Option<Arc<Ip2LocationService>>,
    portfolio: Arc<PortfolioService>,
}

#[derive(Clone)]
struct Repository {
    pub misc: Arc<MiscRepository>,
    pub mkt_data: Arc<MarketDataRepository>,
    pub stats: Arc<StatsRepository>,
    pub imported: Arc<ImportedRepository>,
    pub portfolio: Arc<PortfolioRepository>,
    pub user: Arc<UserRepository>,
}

pub struct DcaServer {
    addr: SocketAddr,
    app: IntoMakeServiceWithConnectInfo<Router<()>, SocketAddr>,
    ctx: AppContext,
    worker_handlers: Vec<JoinHandle<()>>,
    stop_tx: tokio::sync::watch::Sender<bool>,
}

impl DcaServer {
    pub async fn try_new(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        let http = reqwest::Client::builder()
            .gzip(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            rquest::header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                                  AppleWebKit/537.36 (KHTML, like Gecko) \
                                  Chrome/122.0.0.0 Safari/537.36",
            ),
        );

        let rquest = rquest::Client::builder().default_headers(headers).build()?;

        let redis = build_redis_pool(&config.server.redis)?;

        let postgres = build_postgres_pool(&config.server.postgres).await?;

        let repos = Arc::new(Repository {
            misc: Arc::new(MiscRepository::new(redis.clone())),
            mkt_data: Arc::new(MarketDataRepository::new(redis.clone())),
            stats: Arc::new(StatsRepository::new(redis.clone())),
            imported: Arc::new(ImportedRepository::new(redis.clone())),
            portfolio: Arc::new(PortfolioRepository::new(postgres.clone())),
            user: Arc::new(UserRepository::new(postgres.clone())),
        });

        let providers = Arc::new(PriceProviders {
            cw: Arc::new(CryptoWatchProvider::new(
                http.clone(),
                &config.app.providers,
            )),
            kraken: Arc::new(KrakenProvider::new(http.clone(), &config.app.providers)),
            yahoo: Arc::new(YahooProvider::new(rquest.clone())),
            ipapi: Arc::new(IpApi::new(http.clone(), &config.app.providers)),
        });

        let ip2location = {
            if let Some(ref service_config) = config.app.services {
                if let Some(ref ip_config) = service_config.ip {
                    Some(Arc::new(Ip2LocationService::try_new(&ip_config.db_path)?))
                } else {
                    None
                }
            } else {
                None
            }
        };

        let services = Services {
            mkt_data: Arc::new(MarketDataService::new(repos.mkt_data.clone())),
            ip2location,
            portfolio: Arc::new(PortfolioService::new(repos.portfolio.clone())),
        };

        let ctx = Arc::new(AppContextInner {
            config: config.clone(),
            http,
            redis,
            postgres,
            services,
            repos,
            providers,
        });

        let open_routes = Router::new()
            .route("/", get(|| async { "Greetings from DCA-Pal APIs!" }))
            .route("/assets/fiat", get(rest::get_assets_fiat))
            .route("/assets/crypto", get(rest::get_assets_crypto))
            .route("/assets/search", get(rest::get_assets_data))
            .route("/assets/chart/{symbol}", get(rest::get_assets_chart))
            .route("/price/{asset}", get(rest::get_price))
            .route("/import/portfolio", post(rest::import_portfolio))
            .route("/import/portfolio/{id}", get(rest::get_imported_portfolio));

        let authenticated_routes = Router::new()
            .route("/v1/sync/portfolios", post(rest::request::sync_portfolios))
            .with_state(ctx.clone());

        let merged_app = Router::new().merge(open_routes).merge(authenticated_routes);

        let app = merged_app
            .route_layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(middleware::from_fn_with_state(
                        ctx.clone(),
                        infra::stats::requests_stats,
                    ))
                    .layer(middleware::from_fn(infra::stats::latency_stats)),
            )
            .with_state(ctx.clone())
            .into_make_service_with_connect_info();

        let (hostname, port) = (&config.server.web.hostname, config.server.web.port);
        let addr = format!("{hostname}:{port}")
            .parse()
            .map_err(|e: AddrParseError| {
                DcaError::StartupFailure(
                    format!("Invalid hostname ({hostname}) or port ({port})"),
                    e.into(),
                )
            })?;

        let (stop_tx, _) = tokio::sync::watch::channel(false);
        Ok(Self {
            addr,
            app,
            ctx,
            worker_handlers: Vec::new(),
            stop_tx,
        })
    }

    pub async fn start(&mut self, _signal_handler: BoxFuture<'_, ()>) -> Result<()> {
        info!("Initializing metrics");
        self.init_metrics().await;

        info!("Starting MarketDiscovery worker");
        {
            let ctx = self.ctx.clone();
            let stop_rx = self.stop_tx.subscribe();
            let handle = tokio::spawn(async move {
                let worker = MarketDiscoveryWorker::new(&ctx);
                worker.run(stop_rx).await;
            });
            self.worker_handlers.push(handle);
        }

        info!("Starting PriceUpdater worker");
        {
            let ctx = self.ctx.clone();
            let stop_rx = self.stop_tx.subscribe();
            let handle = tokio::spawn(async move {
                let worker = PriceUpdaterWorker::new(&ctx, Duration::from_secs(5 * 60));
                worker.run(stop_rx).await;
            });
            self.worker_handlers.push(handle);
        }

        info!("Starting DcaServer at {}", &self.addr);
        let listener = TcpListener::bind(&self.addr)
            .await
            .map_err(|e| DcaError::StartupFailure("Failed to start DcaServer".into(), e.into()))?;

        axum::serve(listener, self.app.clone())
            .await
            .map_err(|e| DcaError::StartupFailure("Failed to start DcaServer".into(), e.into()))?;

        Ok(())
    }

    pub async fn init_metrics(&self) {
        describe_counter!(
            infra::stats::VISITORS_TOTAL,
            Unit::Count,
            "Number of API visitors"
        );
        describe_counter!(
            infra::stats::REQUESTS_TOTAL,
            Unit::Count,
            "Number of requests processed"
        );
        describe_histogram!(
            infra::stats::LATENCY_SUMMARY,
            Unit::Microseconds,
            "Summary of endpoint response time"
        );
        describe_counter!(
            infra::stats::IMPORTED_PORTFOLIOS_TOTAL,
            Unit::Count,
            "Number of portfolios imported"
        );

        // Refresh Prometheus stats
        if let Err(e) = refresh_total_visitors_stats(&self.ctx.repos.stats).await {
            error!(
                "Failed to refresh Prometheus {} metric: {e:?}",
                infra::stats::VISITORS_TOTAL
            );
        }
        if let Err(e) = refresh_imported_portfolios_stats(&self.ctx.repos.stats).await {
            error!(
                "Failed to refresh Prometheus {} metric: {e:?}",
                infra::stats::IMPORTED_PORTFOLIOS_TOTAL
            );
        }
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

async fn build_postgres_pool(config: &Postgres) -> Result<PgPool> {
    let url = config.connection_url();
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&url)
        .await
        .map_err(|e| {
            DcaError::StartupFailure(
                format!(
                    "Error in building Postgres poll config (user={}, hostname={}, port={}, db={})",
                    &config.user, &config.hostname, &config.port, &config.database
                ),
                e.into(),
            )
        })?;

    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&pool)
        .await
        .map_err(|e| DcaError::StartupFailure("Failed to query Postgres".into(), e.into()))?;

    assert_eq!(row.0, 150);

    Ok(pool)
}

async fn refresh_total_visitors_stats(stats_repo: &StatsRepository) -> Result<()> {
    let visitors = stats_repo.fetch_all_visitors().await?;
    for (ip, count) in visitors {
        // Refresh Prometheus visitors location
        let geo = stats_repo.find_visitor_ip(&ip).await?;
        if let Some(geo) = geo {
            counter!(
                infra::stats::VISITORS_TOTAL,
                &[
                    ("ip", geo.ip),
                    ("latitude", geo.latitude),
                    ("longitude", geo.longitude),
                ]
            )
            .increment(count as u64);
        }
    }

    Ok(())
}

async fn refresh_imported_portfolios_stats(stats_repo: &StatsRepository) -> Result<()> {
    let count = stats_repo.get_imported_portfolio_count().await?;
    counter!(infra::stats::IMPORTED_PORTFOLIOS_TOTAL).increment(count as u64);

    Ok(())
}
