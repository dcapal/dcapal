//! `GET /assets/fiat`: cached fiat asset catalog.

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
    path = "/assets/fiat",
    responses(
        (
            status = 200,
            description = "Fiat assets",
            body = Vec<crate::app::domain::entity::Asset>
        )
    )
)]
/// Returns the cached fiat asset catalog.
pub async fn get_assets_fiat(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Fiat).await?;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}
