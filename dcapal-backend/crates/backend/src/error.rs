use std::fmt::Debug;

use axum::response::IntoResponse;
use config::ConfigError;
use hyper::StatusCode;
use tracing::error;

use crate::app::domain::entity::{AssetId, MarketId};

/// Errors returned by the backend application and its integrations.
#[derive(thiserror::Error)]
pub enum DcaError {
    #[error("{0}")]
    Generic(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("{message}")]
    ValidationFailure {
        message: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Application failure")]
    ApplicationFailure {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Price for market '{0}/{1}' not available")]
    PriceNotAvailable(AssetId, AssetId),
    #[error("Price for market '{0}' not available")]
    PriceNotAvailableId(MarketId),
    #[error("Market '{0}' not found")]
    MarketNotFound(MarketId),
    #[error("Failed to store in Repository: {0}")]
    RepositoryStoreFailure(String, #[source] Option<sqlx::Error>),
    #[error("External service died: {0}")]
    ExternalServiceDied(String),
    /// A provider answered with a non-success HTTP status.
    #[error("{service} responded with HTTP {status}")]
    UpstreamStatus {
        service: &'static str,
        status: StatusCode,
    },
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
    #[error("Third-party API reqwest failed")]
    Reqwest(#[from] reqwest::Error),
    #[error("IP2Location internal error")]
    Ip2Location(#[source] ip2location::error::Error),
    #[error(transparent)]
    TypeHeaderError(#[from] axum_extra::typed_header::TypedHeaderRejection),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
    #[error(transparent)]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("Third-party API reqwest failed")]
    Rquest(#[from] rquest::Error),
}

impl Debug for DcaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ErrorChain(self))
    }
}

impl DcaError {
    /// Iterates over the source errors attached to this error.
    pub fn iter_sources(&self) -> ErrorIter<'_> {
        ErrorIter {
            current: (self as &dyn std::error::Error).source(),
        }
    }

    /// Converts a failsafe result into an application error with service context.
    pub fn from_failsafe(e: failsafe::Error<DcaError>, service: &str) -> Self {
        match e {
            failsafe::Error::Inner(e) => e,
            failsafe::Error::Rejected => DcaError::ExternalServiceDied(service.to_string()),
        }
    }
}

/// The backend's application-wide result type.
pub type Result<T> = std::result::Result<T, DcaError>;

impl IntoResponse for DcaError {
    fn into_response(self) -> axum::response::Response {
        error!("{:?}", &self);
        match self {
            DcaError::BadRequest(_) | DcaError::ValidationFailure { .. } => {
                (StatusCode::BAD_REQUEST, format!("{self}")).into_response()
            }
            DcaError::TypeHeaderError(_) | DcaError::JwtError(_) => {
                (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
            }
            DcaError::PriceNotAvailable(_, _) => {
                (StatusCode::NOT_FOUND, format!("{self}")).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response(),
        }
    }
}

/// Renders an error followed by every error in its `source` chain, as
/// `error -- Caused by: cause -- Caused by: root cause`.
///
/// Use it wherever an error is logged or turned into a message, so the underlying
/// cause is never lost to a `Display` that prints only the outermost error.
pub struct ErrorChain<'a>(pub &'a (dyn std::error::Error + 'static));

impl std::fmt::Display for ErrorChain<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        for cause in (ErrorIter {
            current: self.0.source(),
        }) {
            write!(f, " -- Caused by: {cause}")?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
/// Iterator over the source errors attached to a [`DcaError`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("{message}")]
    struct Layer {
        message: &'static str,
        #[source]
        source: Option<Box<Layer>>,
    }

    /// GIVEN an error with two nested causes,
    /// WHEN it is rendered with `ErrorChain`,
    /// THEN every cause follows the outermost message, separated by "-- Caused by:".
    #[test]
    fn error_chain_renders_every_source() {
        let root = Layer {
            message: "root",
            source: None,
        };
        let mid = Layer {
            message: "mid",
            source: Some(Box::new(root)),
        };
        let top = Layer {
            message: "top",
            source: Some(Box::new(mid)),
        };

        assert_eq!(
            ErrorChain(&top).to_string(),
            "top -- Caused by: mid -- Caused by: root"
        );
    }

    /// GIVEN a repository failure carrying a database error,
    /// WHEN it is debug-printed and its source inspected,
    /// THEN the log text includes the database cause and the cause stays available as `source`.
    #[test]
    fn store_failure_keeps_its_database_cause() {
        let error =
            DcaError::RepositoryStoreFailure("btceur".into(), Some(sqlx::Error::PoolClosed));

        assert!(format!("{error:?}").contains("-- Caused by:"));
        assert!(std::error::Error::source(&error).is_some());
    }

    /// GIVEN an IP2Location failure,
    /// WHEN the error is rendered as a chain,
    /// THEN the cause appears exactly once, as the source, not also in the message.
    #[test]
    fn ip2location_cause_is_rendered_once() {
        let error = DcaError::Ip2Location(ip2location::error::Error::RecordNotFound);

        let rendered = ErrorChain(&error).to_string();

        assert_eq!(rendered.matches("RecordNotFound").count(), 1, "{rendered}");
        assert!(rendered.starts_with("IP2Location internal error -- Caused by:"));
    }
}
