use axum::response::IntoResponse;
use config::ConfigError;
use deadpool_redis::PoolError;
use hyper::StatusCode;
use redis::RedisError;
use tracing::error;

use crate::domain::entity::{AssetId, MarketId};

#[derive(thiserror::Error, Debug)]
pub enum DcaError {
    #[error("{0}")]
    Generic(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Price for market '{0}/{1}' not available")]
    PriceNotAvailable(AssetId, AssetId),
    #[error("Market '{0}' not found")]
    MarketNotFound(MarketId),
    #[error("Failed to store in Repository: {0}")]
    RepositoryStoreFailure(String),
    #[error("{0}")]
    StartupFailure(String, #[source] anyhow::Error),
    #[error("Failed to parse config")]
    Config(#[from] ConfigError),
    #[error("Invalid log file path: {0}")]
    InvalidLogPath(String),
    #[error("Invalid log file path: {0}")]
    InvalidLogPath2(String, #[source] std::io::Error),
    #[error("Failed to obtain Redis connection")]
    RedisPool(#[from] PoolError),
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error("Third-party API reqwest failed")]
    Reqwest(#[from] reqwest::Error),
}

impl DcaError {
    pub fn msg_chain(self) -> String {
        let e = anyhow::Error::from(self);
        build_msg_chain(&e)
    }
}

pub type Result<T> = std::result::Result<T, DcaError>;

impl IntoResponse for DcaError {
    fn into_response(self) -> axum::response::Response {
        let response = match self {
            DcaError::BadRequest(_) => {
                (StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
            }
            DcaError::PriceNotAvailable(_, _) => {
                (StatusCode::NOT_FOUND, format!("{}", self)).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response(),
        };

        let e = anyhow::Error::from(self);
        let msg = build_msg_chain(&e);
        error!("{}{}", e, msg);

        response
    }
}

fn build_msg_chain(e: &anyhow::Error) -> String {
    e.chain()
        .skip(1)
        .map(|c| format!(" -- Caused by: {}", c))
        .collect::<Vec<String>>()
        .join("")
}
