use axum::{
    Json,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;

use crate::{
    app::{
        commands::{
            lookup_assets::LookupAssetsCommandError,
            sync_portfolios_v2::{SyncPortfoliosV2CommandError, too_many_new_assets},
        },
        services::{asset::AssetServiceError, portfolio::PortfolioServiceError},
    },
    ports::inbound::rest::types::sync_portfolio_v2::{
        V2ErrorResponse, V2FieldError, V2ValidationError, VALIDATION_CODE,
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
const LOOKUP_INVALID_MESSAGE: &str = "The asset lookup request is invalid.";
const ASSET_INTERNAL_MESSAGE: &str = "The assets could not be resolved.";
const PROVIDER_UNAVAILABLE_CODE: &str = "provider_unavailable";
const PROVIDER_UNAVAILABLE_MESSAGE: &str =
    "An asset provider is unavailable; retry the request later.";

/// Errors emitted by the v2 HTTP boundary.
#[derive(Debug, thiserror::Error)]
pub enum V2HttpError {
    #[error("v2 validation failed")]
    Validation(#[from] V2ValidationError),
    #[error("v2 command construction failed")]
    Command(#[from] SyncPortfoliosV2CommandError),
    #[error("v2 Portfolio synchronization failed")]
    Service(#[from] PortfolioServiceError),
    #[error("v2 asset lookup request is invalid")]
    LookupCommand(#[from] LookupAssetsCommandError),
    #[error("v2 asset service failed")]
    Asset(#[from] AssetServiceError),
}

/// Maps an asset-service failure to its response.
///
/// `Unresolved` is reported with real field paths by the sync command, so it
/// only reaches here if a caller bypassed that mapping, which is a server bug.
/// `TooManyNewAssets` is a client error reported as a validation failure.
fn asset_failure(error: &AssetServiceError) -> (StatusCode, V2ErrorResponse) {
    match error {
        AssetServiceError::ProviderUnavailable(_) => provider_unavailable(),
        AssetServiceError::TooManyNewAssets { .. } => (
            StatusCode::BAD_REQUEST,
            V2ValidationError::from(too_many_new_assets()).response(),
        ),
        AssetServiceError::Directory(_) | AssetServiceError::Unresolved(_) => asset_internal(),
    }
}

/// Builds a response without field-level errors.
fn plain(status: StatusCode, code: &str, message: &str) -> (StatusCode, V2ErrorResponse) {
    (
        status,
        V2ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            errors: Vec::new(),
        },
    )
}

fn provider_unavailable() -> (StatusCode, V2ErrorResponse) {
    plain(
        StatusCode::BAD_GATEWAY,
        PROVIDER_UNAVAILABLE_CODE,
        PROVIDER_UNAVAILABLE_MESSAGE,
    )
}

fn asset_internal() -> (StatusCode, V2ErrorResponse) {
    plain(
        StatusCode::INTERNAL_SERVER_ERROR,
        INTERNAL_CODE,
        ASSET_INTERNAL_MESSAGE,
    )
}

/// Builds the response of a Portfolio synchronization that failed internally.
fn sync_internal() -> (StatusCode, V2ErrorResponse) {
    plain(
        StatusCode::INTERNAL_SERVER_ERROR,
        INTERNAL_CODE,
        INTERNAL_MESSAGE,
    )
}

/// Builds the 403 response for a resource of another user, naming `field`.
fn ownership(field: String) -> (StatusCode, V2ErrorResponse) {
    (
        StatusCode::FORBIDDEN,
        V2ErrorResponse {
            code: OWNERSHIP_CODE.to_string(),
            message: OWNERSHIP_MESSAGE.to_string(),
            errors: vec![V2FieldError {
                field,
                message: OWNERSHIP_FIELD_MESSAGE.to_string(),
            }],
        },
    )
}

impl IntoResponse for V2HttpError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::Validation(error) => (StatusCode::BAD_REQUEST, error.response()),
            Self::Command(SyncPortfoliosV2CommandError::Validation(error)) => (
                StatusCode::BAD_REQUEST,
                V2ValidationError::from(error).response(),
            ),
            Self::Command(SyncPortfoliosV2CommandError::Ownership(location)) => {
                ownership(location.field_path().render())
            }
            Self::Command(SyncPortfoliosV2CommandError::ProviderUnavailable(_)) => {
                provider_unavailable()
            }
            Self::Command(SyncPortfoliosV2CommandError::Directory(_)) => asset_internal(),
            Self::Service(PortfolioServiceError::Asset(error)) => asset_failure(&error),
            Self::Service(PortfolioServiceError::CannotUpdate(_)) => {
                ownership("portfolios".to_string())
            }
            Self::Service(PortfolioServiceError::Conflict(_)) => {
                plain(StatusCode::CONFLICT, CONFLICT_CODE, CONFLICT_MESSAGE)
            }
            Self::LookupCommand(error) => {
                let (field, message) = match error {
                    LookupAssetsCommandError::Invalid { index, reason } => {
                        (format!("asset[{index}]"), reason.to_string())
                    }
                    other => ("asset".to_string(), other.to_string()),
                };
                (
                    StatusCode::BAD_REQUEST,
                    V2ErrorResponse {
                        code: VALIDATION_CODE.to_string(),
                        message: LOOKUP_INVALID_MESSAGE.to_string(),
                        errors: vec![V2FieldError { field, message }],
                    },
                )
            }
            Self::Asset(error) => asset_failure(&error),
            Self::Command(SyncPortfoliosV2CommandError::Repository(_)) | Self::Service(_) => {
                sync_internal()
            }
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::commands::sync_portfolios_v2::{FieldPath, OwnershipLocation};
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
        let (status, body) = response_body(V2HttpError::Command(
            SyncPortfoliosV2CommandError::Validation(validation),
        ))
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation_error");
        assert_eq!(body["errors"][0]["field"], "portfolios[0].name");
    }

    #[tokio::test]
    async fn too_many_new_assets_is_a_validation_error() {
        // GIVEN an asset-service failure for too many new assets,
        // WHEN the v2 boundary adapts it,
        // THEN the response is a 400 validation_error at `portfolios`.
        let (status, body) =
            response_body(V2HttpError::Asset(AssetServiceError::TooManyNewAssets {
                limit: 50,
            }))
            .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation_error");
        assert_eq!(body["errors"][0]["field"], "portfolios");
    }

    #[tokio::test]
    async fn lookup_validation_points_at_the_offending_asset() {
        // GIVEN an invalid asset reference at index 2, WHEN the lookup command error is
        // adapted, THEN the 400 body names `asset[2]` with the reason.
        let (status, body) = response_body(V2HttpError::LookupCommand(
            LookupAssetsCommandError::Invalid {
                index: 2,
                reason: "provider must be one of yf, kraken",
            },
        ))
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation_error");
        assert_eq!(body["errors"][0]["field"], "asset[2]");
        assert_eq!(
            body["errors"][0]["message"],
            "provider must be one of yf, kraken"
        );

        let (status, body) = response_body(V2HttpError::LookupCommand(
            LookupAssetsCommandError::TooMany,
        ))
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["errors"][0]["field"], "asset");
    }

    #[tokio::test]
    async fn asset_failures_keep_distinct_statuses() {
        // GIVEN a provider outage and a directory failure, raised either by the
        // sync command, by onboarding, or by the lookup endpoint,
        // WHEN each is adapted,
        // THEN outages are 502 provider_unavailable and directory failures are 500.
        use crate::ports::outbound::{
            adapter::AssetProviderError, repository::asset::AssetDirectoryError,
        };

        let outages = [
            V2HttpError::Command(SyncPortfoliosV2CommandError::ProviderUnavailable(
                AssetProviderError::Malformed("x".into()),
            )),
            V2HttpError::Service(PortfolioServiceError::Asset(
                AssetServiceError::ProviderUnavailable(AssetProviderError::Malformed("x".into())),
            )),
            V2HttpError::Asset(AssetServiceError::ProviderUnavailable(
                AssetProviderError::Malformed("x".into()),
            )),
        ];
        for outage in outages {
            let (status, body) = response_body(outage).await;
            assert_eq!(status, StatusCode::BAD_GATEWAY);
            assert_eq!(body["code"], PROVIDER_UNAVAILABLE_CODE);
        }

        let failures = [
            V2HttpError::Command(SyncPortfoliosV2CommandError::Directory(
                AssetDirectoryError::CorruptData("x".into()),
            )),
            V2HttpError::Asset(AssetServiceError::Directory(
                AssetDirectoryError::CorruptData("x".into()),
            )),
        ];
        for failure in failures {
            let (status, body) = response_body(failure).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["code"], INTERNAL_CODE);
        }
    }

    #[tokio::test]
    async fn conflict_and_internal_errors_keep_distinct_statuses() {
        // GIVEN persistence failures at the v2 boundary, WHEN they are adapted,
        // THEN serialization conflicts remain retryable while other failures
        // remain internal errors.
        let (status, body) = response_body(V2HttpError::Service(PortfolioServiceError::Conflict(
            PortfolioRepositoryError::CorruptData("x".into()),
        )))
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], CONFLICT_CODE);

        let (status, body) = response_body(V2HttpError::Service(
            PortfolioServiceError::Repository(PortfolioRepositoryError::CorruptData("x".into())),
        ))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], INTERNAL_CODE);
    }

    #[tokio::test]
    async fn ownership_errors_include_the_offending_field_path() {
        // GIVEN an ownership error for the second submitted Portfolio, WHEN the
        // v2 HTTP boundary converts it, THEN it returns 403 and preserves that
        // path in the structured error body.
        let (status, body) = response_body(V2HttpError::Command(
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::Portfolio { index: 1 }),
        ))
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "ownership_error");
        assert_eq!(body["errors"][0]["field"], "portfolios[1].id");
    }

    #[tokio::test]
    async fn repository_ownership_conflicts_are_forbidden_at_portfolios() {
        // GIVEN the repository refused to update a Portfolio of another user,
        // WHEN the v2 HTTP boundary converts it,
        // THEN it returns 403 ownership_error naming `portfolios`.
        let (status, body) = response_body(V2HttpError::Service(
            PortfolioServiceError::CannotUpdate(PortfolioRepositoryError::CannotUpdate),
        ))
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "ownership_error");
        assert_eq!(body["errors"][0]["field"], "portfolios");
    }
}
