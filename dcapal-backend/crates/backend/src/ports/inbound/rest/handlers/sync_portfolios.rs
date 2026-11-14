//! `POST /v1/sync/portfolios`: legacy portfolio synchronization.

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use tracing::info;

use crate::{
    AppContext,
    app::infra::claim::Claims,
    ports::inbound::rest::types::sync_portfolios::{SyncPortfoliosRequest, SyncPortfoliosResponse},
};

#[utoipa::path(
    post,
    path = "/sync/portfolios",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token")
    ),
    request_body = SyncPortfoliosRequest,
    responses(
        (
            status = 200,
            description = "Portfolios synchronized",
            body = SyncPortfoliosResponse
        ),
        (status = 400, description = "Bad request")
    )
)]
/// Applies authenticated local portfolio changes and returns the server state.
pub async fn sync_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<SyncPortfoliosRequest>,
) -> crate::error::Result<Response> {
    info!("Syncing portfolios for user_id: {}.", claims.sub);
    let resp = ctx
        .services
        .portfolio
        .sync_portfolios(claims.sub, req)
        .await
        .map_err(crate::error::DcaError::from)?;
    info!(
        "Successfully synced portfolios for user_id: {}.",
        claims.sub
    );
    Ok(Json(resp).into_response())
}

#[cfg(test)]
mod sync_error_tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ports::outbound::repository::portfolio::PortfolioRepositoryError;
    use crate::{app::services::portfolio::PortfolioServiceError, error::DcaError};

    #[test]
    fn repository_failure_is_an_internal_server_error() {
        // GIVEN a repository failure, WHEN the REST boundary maps it,
        // THEN it returns HTTP 500 without exposing persistence details.
        let error = DcaError::from(PortfolioServiceError::Repository(
            PortfolioRepositoryError::Database(sqlx::Error::RowNotFound),
        ));
        let source_types = {
            let sources: Vec<_> = error.iter_sources().collect();
            (
                sources.len(),
                sources[0].downcast_ref::<PortfolioServiceError>().is_some(),
                sources[1]
                    .downcast_ref::<PortfolioRepositoryError>()
                    .is_some(),
                sources[2].downcast_ref::<sqlx::Error>().is_some(),
            )
        };
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(source_types, (3, true, true, true));
    }

    #[test]
    fn ownership_failure_is_a_bad_request() {
        // GIVEN a sync update for a portfolio owned by another user, WHEN the REST boundary maps it,
        // THEN it returns HTTP 400 rather than treating the client conflict as a server failure.
        let error = DcaError::from(PortfolioServiceError::CannotUpdate(
            PortfolioRepositoryError::CannotUpdate,
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn response_conversion_failure_is_an_internal_server_error() {
        // GIVEN a missing manual-price compatibility failure, WHEN the REST boundary maps it,
        // THEN it returns HTTP 500 without exposing internal conversion details.
        let error = DcaError::from(PortfolioServiceError::ResponseConversion(
            DcaError::Generic("v1 Portfolio Asset response requires a manual price.".to_string()),
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
