use std::fmt::Debug;

use axum::response::IntoResponse;
use config::ConfigError;
use deadpool_redis::PoolError;
use hyper::StatusCode;
use redis::RedisError;
use tracing::error;

use crate::app::domain::entity::{AssetId, MarketId};

#[derive(thiserror::Error)]
pub enum DcaError {
    #[error("{0}")]
    Generic(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Price for market '{0}/{1}' not available")]
    PriceNotAvailable(AssetId, AssetId),
    #[error("Price for market '{0}' not available")]
    PriceNotAvailableId(MarketId),
    #[error("Market '{0}' not found")]
    MarketNotFound(MarketId),
    #[error("Failed to store in Repository: {0}")]
    RepositoryStoreFailure(String),
    #[error("External service died: {0}")]
    ExternalServiceDied(String),
    #[error("{0}")]
    StartupFailure(String, #[source] anyhow::Error),
    #[error("Failed to parse config")]
    Config(#[from] ConfigError),
    #[error("Invalid log file path: {0}")]
    InvalidLogPath(String),
    #[error("Invalid log file path: {0}")]
    InvalidLogPath2(String, #[source] std::io::Error),
    #[error("Failed to deserialized into {1}: {0}")]
    JsonDeserializationFailure(String, String, #[source] serde_json::Error),
    #[error("Failed to obtain Redis connection")]
    RedisPool(#[from] PoolError),
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error("Third-party API reqwest failed")]
    Reqwest(#[from] reqwest::Error),
    #[error("IP2Location internal error: {0:?}")]
    Ip2Location(ip2location::error::Error),
    #[error(transparent)]
    TypeHeaderError(#[from] axum_extra::typed_header::TypedHeaderRejection),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
    #[error(transparent)]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("OpenAI Error: {0}")]
    OpenAIError(#[from] async_openai::error::OpenAIError),
    #[error(transparent)]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Third-party API reqwest failed")]
    Rquest(#[from] rquest::Error),
}

impl Debug for DcaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))?;

        for e in self.iter_sources() {
            f.write_fmt(format_args!(" -- Caused by: {e}"))?;
        }

        Ok(())
    }
}

impl From<sea_orm::TransactionError<DcaError>> for DcaError {
    fn from(err: sea_orm::TransactionError<DcaError>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(e) => DcaError::DatabaseError(e),
            sea_orm::TransactionError::Transaction(e) => e,
        }
    }
}

impl DcaError {
    pub fn iter_sources(&self) -> ErrorIter {
        ErrorIter {
            current: (self as &dyn std::error::Error).source(),
        }
    }

    pub fn from_failsafe(e: failsafe::Error<DcaError>, service: &str) -> Self {
        match e {
            failsafe::Error::Inner(e) => e,
            failsafe::Error::Rejected => DcaError::ExternalServiceDied(service.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DcaError>;

impl IntoResponse for DcaError {
    fn into_response(self) -> axum::response::Response {
        error!("{:?}", &self);
        match self {
            DcaError::BadRequest(_) => (StatusCode::BAD_REQUEST, format!("{self}")).into_response(),
            DcaError::PriceNotAvailable(_, _) => {
                (StatusCode::NOT_FOUND, format!("{self}")).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ErrorIter<'a> {
    current: Option<&'a (dyn std::error::Error + 'static)>,
}

impl<'a> Iterator for ErrorIter<'a> {
    type Item = &'a (dyn std::error::Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(std::error::Error::source);
        current
    }
}

impl From<ip2location::error::Error> for DcaError {
    fn from(e: ip2location::error::Error) -> Self {
        DcaError::Ip2Location(e)
    }
}
