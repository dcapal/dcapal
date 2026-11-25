//! `GET /assets/crypto`: cached cryptocurrency asset catalog.

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;

use crate::{AppContext, app::domain::entity::AssetKind, error::Result};

use super::super::cache::ASSETS_CACHE_CONTROL;

#[utoipa::path(
    get,
    path = "/assets/crypto",
    responses(
        (
            status = 200,
            description = "Crypto assets",
            body = Vec<crate::app::domain::entity::Asset>
        )
    )
)]
/// Returns the cached cryptocurrency asset catalog.
pub async fn get_assets_crypto(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Crypto).await?;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}
