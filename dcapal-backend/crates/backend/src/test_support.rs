//! Helpers shared by the backend's in-crate tests.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{Router, http::Uri};
use serde_json::json;
use sqlx::{PgPool, postgres::PgConnectOptions};
use tokio::net::TcpListener;

use crate::{DcaServer, config::Config};

/// One canned answer of a [`FakeProvider`].
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    /// A `200 OK` JSON reply.
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
        }
    }

    /// A reply with the given status and body.
    pub fn status(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }
}

/// A local HTTP server that stands in for a market-data provider.
///
/// The responder receives the request path and query (`/v8/finance/chart/EURUSD%3DX?...`)
/// and every request is recorded, so a test can also prove which hosts were not called.
pub struct FakeProvider {
    addr: SocketAddr,
    hits: Arc<Mutex<Vec<String>>>,
}

impl FakeProvider {
    /// Starts a server answering every request with `responder`.
    pub async fn start(responder: impl Fn(&str) -> Reply + Send + Sync + 'static) -> Self {
        let hits = Arc::new(Mutex::new(Vec::new()));
        let recorded = hits.clone();
        let responder = Arc::new(responder);
        let app = Router::new().fallback(move |uri: Uri| {
            let recorded = recorded.clone();
            let responder = responder.clone();
            async move {
                let target = uri
                    .path_and_query()
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                recorded.lock().unwrap().push(target.clone());
                let reply = responder(&target);
                (
                    axum::http::StatusCode::from_u16(reply.status).unwrap(),
                    [("content-type", "application/json")],
                    reply.body,
                )
            }
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        Self { addr, hits }
    }

    /// Root URL of the server, without a trailing slash.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Path and query of every request received so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

/// Installs the process-wide TLS provider that HTTP clients need to be built.
pub fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // A concurrent test may have installed it first; either outcome is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

/// Base URLs that replace the real provider hosts of a test server.
#[derive(Debug, Default, Clone)]
pub struct ProviderUrls {
    /// Replaces the Yahoo hosts.
    pub yahoo: Option<String>,
    /// Replaces the Kraken host.
    pub kraken: Option<String>,
}

/// The shared JWT secret of test servers.
const JWT_SECRET: &str = "secret";

/// Builds a server on the test database and serves it on an ephemeral loopback port.
pub async fn serve(pool: &PgPool, urls: ProviderUrls) -> SocketAddr {
    let options: PgConnectOptions = (*pool.connect_options()).clone();
    let config: Config = serde_json::from_value(json!({
        "app": {
            "log": {},
            "providers": {
                "priceProvider": "kraken", "ipApiKey": "",
                "yahooBaseUrl": urls.yahoo, "krakenBaseUrl": urls.kraken
            },
            "services": null,
            "auth": {"jwtSecret": JWT_SECRET}
        },
        "server": {
            "web": {"hostname": "127.0.0.1", "port": 0},
            "metrics": {"hostname": "127.0.0.1", "port": 0},
            "postgres": {
                "hostname": options.get_host(),
                "port": options.get_port(),
                "user": options.get_username(),
                "password": std::env::var("DATABASE_URL")
                    .ok()
                    .and_then(|u| url::Url::parse(&u).ok())
                    .and_then(|u| u.password().map(str::to_string))
                    .unwrap_or_default(),
                "database": options.get_database().unwrap_or("postgres"),
            }
        }
    }))
    .unwrap();

    let server = DcaServer::try_new(config).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = server.app.clone();
    tokio::spawn(async move { axum::serve(listener, app).await });
    addr
}

/// Mints a valid bearer token for a test user.
pub fn bearer_token() -> String {
    use jsonwebtoken::{EncodingKey, Header, encode};
    use uuid::Uuid;

    use crate::app::infra::claim::{Claims, UserMetadataClaim};

    let claims = Claims {
        iat: 1_700_000_000,
        exp: 4_000_000_000,
        sub: Uuid::from_u128(7),
        session_id: Uuid::from_u128(8),
        role: "authenticated".to_string(),
        aud: "authenticated".to_string(),
        user_metadata: UserMetadataClaim {
            email: "market-v2@example.com".to_string(),
            full_name: None,
        },
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()).unwrap(),
    )
    .unwrap()
}
