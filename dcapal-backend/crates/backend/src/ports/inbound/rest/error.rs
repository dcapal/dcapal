use axum::{
    Json,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;

use crate::{
    app::{
        commands::{
            lookup_assets::LookupAssetsCommandError,
            sync_portfolios_v2::{
                FieldPath, SyncPortfoliosV2CommandError, ValidationError,
                validation::MESSAGE_TOO_MANY_NEW_ASSETS,
            },
        },
        services::{asset::AssetServiceError, portfolio::PortfolioServiceError},
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
const LOOKUP_INVALID_MESSAGE: &str = "The asset lookup request is invalid.";
const ASSET_INTERNAL_MESSAGE: &str = "The assets could not be resolved.";
const PROVIDER_UNAVAILABLE_CODE: &str = "provider_unavailable";
const PROVIDER_UNAVAILABLE_MESSAGE: &str =
    "An asset provider is unavailable; retry the request later.";

/// Errors emitted by the v2 HTTP boundary.
#[derive(Debug, thiserror::Error)]
pub enum V2HttpError {
    #[error("v2 validation failed")]
    Validation(#[source] V2ValidationError),
    #[error("v2 command construction failed")]
    Command(#[source] SyncPortfoliosV2CommandError),
    #[error("v2 Portfolio synchronization failed")]
    Service(#[source] PortfolioServiceError),
    #[error("v2 asset lookup request is invalid")]
    LookupCommand(#[source] LookupAssetsCommandError),
    #[error("v2 asset service failed")]
    Asset(#[source] AssetServiceError),
}

impl From<LookupAssetsCommandError> for V2HttpError {
    fn from(error: LookupAssetsCommandError) -> Self {
        Self::LookupCommand(error)
    }
}

impl From<AssetServiceError> for V2HttpError {
    fn from(error: AssetServiceError) -> Self {
        Self::Asset(error)
    }
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
            V2ValidationError::from(ValidationError::single(
                FieldPath::root().field("portfolios"),
                MESSAGE_TOO_MANY_NEW_ASSETS,
            ))
            .response(),
        ),
        AssetServiceError::Directory(_) | AssetServiceError::Unresolved(_) => asset_internal(),
    }
}

fn provider_unavailable() -> (StatusCode, V2ErrorResponse) {
    (
        StatusCode::BAD_GATEWAY,
        V2ErrorResponse {
            code: PROVIDER_UNAVAILABLE_CODE.to_string(),
            message: PROVIDER_UNAVAILABLE_MESSAGE.to_string(),
            errors: Vec::new(),
        },
    )
}

fn asset_internal() -> (StatusCode, V2ErrorResponse) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        V2ErrorResponse {
            code: INTERNAL_CODE.to_string(),
            message: ASSET_INTERNAL_MESSAGE.to_string(),
            errors: Vec::new(),
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
            Self::Command(SyncPortfoliosV2CommandError::ProviderUnavailable(_)) => {
                provider_unavailable()
            }
            Self::Command(SyncPortfoliosV2CommandError::Directory(_)) => asset_internal(),
            Self::Service(PortfolioServiceError::Asset(error)) => asset_failure(&error),
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
            Self::LookupCommand(error) => {
                let field = match &error {
                    LookupAssetsCommandError::Invalid { index, .. } => format!("asset[{index}]"),
                    _ => "asset".to_string(),
                };
                (
                    StatusCode::BAD_REQUEST,
                    V2ErrorResponse {
                        code: "validation_error".to_string(),
                        message: LOOKUP_INVALID_MESSAGE.to_string(),
                        errors: vec![V2FieldError {
                            field,
                            message: match error {
                                LookupAssetsCommandError::Invalid { reason, .. } => {
                                    reason.to_string()
                                }
                                other => other.to_string(),
                            },
                        }],
                    },
                )
            }
            Self::Asset(error) => asset_failure(&error),
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
        // GIVEN an ownership error for the second submitted Portfolio, WHEN the
        // v2 HTTP boundary converts it, THEN it returns 403 and preserves that
        // path in the structured error body.
        let response = V2HttpError::Service(PortfolioServiceError::Ownership {
            location: crate::app::commands::sync_portfolios_v2::OwnershipLocation::Portfolio {
                index: 1,
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
        assert_eq!(body["errors"][0]["field"], "portfolios[1].id");
    }
}
