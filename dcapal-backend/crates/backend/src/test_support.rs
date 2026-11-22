//! Helpers shared by the backend's in-crate tests.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{Router, http::Uri};
use tokio::net::TcpListener;

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
