use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    AppContext, app::commands::lookup_assets::LookupAssetsCommand, app::infra::claim::Claims,
};

use super::{
    error::V2HttpError,
    types::{lookup_assets_v2::LookupAssetsV2Response, sync_portfolio_v2::V2ErrorResponse},
};

/// Documents the repeatable `asset` query parameter of the lookup endpoint.
///
/// The handler reads the raw pairs because repeated keys do not deserialize
/// into a struct field.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[allow(dead_code)]
pub struct LookupAssetsV2Query {
    /// Asset reference as `<provider>:<symbol>` with provider `yf` or `kraken`.
    /// Repeat the parameter for several assets (1 to 20 per call).
    asset: Vec<String>,
}

/// Reads the cached metadata of up to 20 assets without contacting providers.
#[utoipa::path(
    get,
    path = "/assets/lookup",
    operation_id = "lookup_assets_v2",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token"),
        LookupAssetsV2Query
    ),
    responses(
        (status = 200, description = "Cached assets and unknown references", body = LookupAssetsV2Response),
        (status = 400, description = "Invalid asset references", body = V2ErrorResponse),
        (status = 500, description = "Asset lookup failure", body = V2ErrorResponse)
    )
)]
pub async fn lookup_assets(
    State(ctx): State<AppContext>,
    _claims: Claims,
    Query(pairs): Query<Vec<(String, String)>>,
) -> Result<Response, V2HttpError> {
    let raw_assets = pairs
        .into_iter()
        .filter_map(|(key, value)| (key == "asset").then_some(value))
        .collect::<Vec<_>>();
    let command = LookupAssetsCommand::try_new(&raw_assets)?;
    let lookup = ctx.services.asset.lookup_known(command).await?;

    Ok(Json(LookupAssetsV2Response::from(lookup)).into_response())
}
