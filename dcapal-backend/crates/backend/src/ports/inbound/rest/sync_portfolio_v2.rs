use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use tracing::info;

use crate::{
    AppContext,
    app::{
        commands::sync_portfolios_v2::SyncPortfoliosV2Command,
        domain::portfolio::SyncPortfoliosV2Request as AppSyncPortfoliosV2Request,
    },
    infra::claim::Claims,
};

use super::{
    error::V2HttpError,
    types::sync_portfolio_v2::{
        SyncPortfoliosV2Request, SyncPortfoliosV2Response, V2ErrorResponse,
    },
};

/// Applies a complete v2 Portfolio aggregate synchronization.
#[utoipa::path(
    post,
    path = "/sync/portfolios",
    operation_id = "sync_portfolios_v2",
    params(("Authorization" = String, Header, description = "Bearer JWT token")),
    request_body = SyncPortfoliosV2Request,
    responses(
        (status = 200, description = "Portfolios synchronized", body = SyncPortfoliosV2Response),
        (status = 400, description = "Invalid Portfolio aggregate", body = V2ErrorResponse),
        (status = 403, description = "Portfolio ownership violation", body = V2ErrorResponse),
        (status = 409, description = "Serialization conflict", body = V2ErrorResponse),
        (status = 502, description = "Asset provider unavailable", body = V2ErrorResponse),
        (status = 500, description = "Portfolio synchronization failure", body = V2ErrorResponse)
    )
)]
pub async fn sync_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(request): Json<SyncPortfoliosV2Request>,
) -> Result<Response, V2HttpError> {
    info!("Syncing v2 portfolios for user_id: {}.", claims.sub);
    let request = AppSyncPortfoliosV2Request::try_from(request)
        .map_err(|error| V2HttpError::Validation(error.into()))?;
    let command = SyncPortfoliosV2Command::try_new(
        claims.sub,
        request,
        ctx.repos.portfolio.as_ref(),
        ctx.services.asset.as_ref(),
    )
    .await
    .map_err(V2HttpError::from)?;
    let response = ctx
        .services
        .portfolio
        .sync_portfolios_v2(command)
        .await
        .map_err(V2HttpError::from)?;
    Ok(Json(SyncPortfoliosV2Response::from(response)).into_response())
}
