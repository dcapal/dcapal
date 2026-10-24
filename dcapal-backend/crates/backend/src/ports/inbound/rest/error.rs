use axum::{
    Json,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;

use crate::{
    app::{
        commands::sync_portfolios_v2::SyncPortfoliosV2CommandError,
        services::portfolio::PortfolioServiceError,
    },
    ports::inbound::rest::types::sync_portfolio_v2::{
        V2ErrorResponse, V2FieldError, V2ValidationError,
    },
};

const OWNERSHIP_CODE: &str = "ownership_error";
const OWNERSHIP_MESSAGE: &str = "The Portfolio does not belong to the authenticated user.";
const OWNERSHIP_FIELD_MESSAGE: &str = "the resource does not belong to the authenticated user";
const CONFLICT_CODE: &str = "serialization_conflict";
const CONFLICT_MESSAGE: &str =
    "The Portfolio changed while it was being synchronized; retry the request.";
const INTERNAL_CODE: &str = "internal_error";
const INTERNAL_MESSAGE: &str = "The Portfolio could not be synchronized.";

/// Errors emitted by the v2 HTTP boundary.
#[derive(Debug, thiserror::Error)]
pub enum V2HttpError {
    #[error("v2 validation failed")]
    Validation(#[source] V2ValidationError),
    #[error("v2 command construction failed")]
    Command(#[source] SyncPortfoliosV2CommandError),
    #[error("v2 Portfolio synchronization failed")]
    Service(#[source] PortfolioServiceError),
}

impl From<PortfolioServiceError> for V2HttpError {
    fn from(error: PortfolioServiceError) -> Self {
        Self::Service(error)
    }
}

impl From<SyncPortfoliosV2CommandError> for V2HttpError {
    fn from(error: SyncPortfoliosV2CommandError) -> Self {
        Self::Command(error)
    }
}

impl IntoResponse for V2HttpError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::Validation(error) => (StatusCode::BAD_REQUEST, error.response()),
            Self::Command(SyncPortfoliosV2CommandError::Validation(error)) => (
                StatusCode::BAD_REQUEST,
                V2ValidationError::from(error).response(),
            ),
            Self::Command(SyncPortfoliosV2CommandError::Ownership(location)) => (
                StatusCode::FORBIDDEN,
                V2ErrorResponse {
                    code: OWNERSHIP_CODE.to_string(),
                    message: OWNERSHIP_MESSAGE.to_string(),
                    errors: vec![V2FieldError {
                        field: location.field_path().render(),
                        message: OWNERSHIP_FIELD_MESSAGE.to_string(),
                    }],
                },
            ),
            Self::Command(SyncPortfoliosV2CommandError::Repository(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                V2ErrorResponse {
                    code: INTERNAL_CODE.to_string(),
                    message: INTERNAL_MESSAGE.to_string(),
                    errors: Vec::new(),
                },
            ),
            Self::Service(PortfolioServiceError::Validation(error)) => (
                StatusCode::BAD_REQUEST,
                V2ValidationError::from(error).response(),
            ),
            Self::Service(PortfolioServiceError::CannotUpdate(_)) => (
                StatusCode::FORBIDDEN,
                V2ErrorResponse {
                    code: OWNERSHIP_CODE.to_string(),
                    message: OWNERSHIP_MESSAGE.to_string(),
                    errors: vec![V2FieldError {
                        field: "portfolios".to_string(),
                        message: OWNERSHIP_FIELD_MESSAGE.to_string(),
                    }],
                },
            ),
            Self::Service(PortfolioServiceError::Ownership { location }) => (
                StatusCode::FORBIDDEN,
                V2ErrorResponse {
                    code: OWNERSHIP_CODE.to_string(),
                    message: OWNERSHIP_MESSAGE.to_string(),
                    errors: vec![V2FieldError {
                        field: location.field_path().render(),
                        message: OWNERSHIP_FIELD_MESSAGE.to_string(),
                    }],
                },
            ),
            Self::Service(PortfolioServiceError::Conflict(_)) => (
                StatusCode::CONFLICT,
                V2ErrorResponse {
                    code: CONFLICT_CODE.to_string(),
                    message: CONFLICT_MESSAGE.to_string(),
                    errors: Vec::new(),
                },
            ),
            Self::Service(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                V2ErrorResponse {
                    code: INTERNAL_CODE.to_string(),
                    message: INTERNAL_MESSAGE.to_string(),
                    errors: Vec::new(),
                },
            ),
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::commands::sync_portfolios_v2::FieldPath;
    use crate::ports::outbound::repository::portfolio::PortfolioRepositoryError;

    async fn response_body(error: V2HttpError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("structured v2 response body");
        let body = serde_json::from_slice(&body).expect("structured v2 response");
        (status, body)
    }

    #[tokio::test]
    async fn validation_is_a_structured_bad_request() {
        // GIVEN an application validation issue, WHEN the v2 boundary adapts
        // it, THEN the rendered nested field path is retained in the 400 body.
        let validation = crate::app::commands::sync_portfolios_v2::ValidationError::single(
            FieldPath::root().field("portfolios").index(0).field("name"),
            "must not be empty",
        );
        let (status, body) = response_body(V2HttpError::Service(
            PortfolioServiceError::Validation(validation),
        ))
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation_error");
        assert_eq!(body["errors"][0]["field"], "portfolios[0].name");
    }

    #[tokio::test]
    async fn conflict_and_internal_errors_keep_distinct_statuses() {
        // GIVEN persistence failures at the v2 boundary, WHEN they are adapted,
        // THEN serialization conflicts remain retryable while other failures
        // remain internal errors.
        let (status, body) = response_body(V2HttpError::Service(PortfolioServiceError::Conflict(
            PortfolioRepositoryError::UnsupportedV2,
        )))
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], CONFLICT_CODE);

        let (status, body) = response_body(V2HttpError::Service(
            PortfolioServiceError::Repository(PortfolioRepositoryError::UnsupportedV2),
        ))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], INTERNAL_CODE);
    }

    #[tokio::test]
    async fn ownership_errors_include_the_offending_field_path() {
        // GIVEN a repository ownership error with a nested field path, WHEN
        // the v2 HTTP boundary converts it, THEN it returns 403 and preserves
        // that path in the structured error body.
        let response = V2HttpError::Service(PortfolioServiceError::Ownership {
            location: crate::app::commands::sync_portfolios_v2::OwnershipLocation::PortfolioAsset {
                portfolio_index: 1,
                asset_index: 0,
            },
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("ownership response body");
        let body: serde_json::Value =
            serde_json::from_slice(&body).expect("structured ownership response");
        assert_eq!(body["code"], "ownership_error");
        assert_eq!(body["errors"][0]["field"], "portfolios[1].assets[0].id");
    }
}
