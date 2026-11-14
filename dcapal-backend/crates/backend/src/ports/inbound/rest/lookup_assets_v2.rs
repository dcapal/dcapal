use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    AppContext, app::commands::lookup_assets::LookupAssetsCommand, app::infra::claim::Claims,
};

use super::{
    error::V2HttpError,
    extract::LookupAssetRefs,
    types::{lookup_assets_v2::LookupAssetsV2Response, sync_portfolio_v2::V2ErrorResponse},
};

/// Reads the cached metadata of up to 20 assets without contacting providers.
#[utoipa::path(
    get,
    path = "/assets/lookup",
    operation_id = "lookup_assets_v2",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token"),
        ("asset" = Vec<String>, Query, description = "Asset reference as `<provider>:<symbol>` with provider `yf` or `kraken`. Repeat the parameter for several assets (1 to 20 per call).")
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
    LookupAssetRefs(raw_assets): LookupAssetRefs,
) -> Result<Response, V2HttpError> {
    let command = LookupAssetsCommand::try_new(&raw_assets)?;
    let lookup = ctx.services.asset.lookup_known(command).await?;

    Ok(Json(LookupAssetsV2Response::from(lookup)).into_response())
}
