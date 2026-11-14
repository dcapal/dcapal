//! `GET /assets/search`: provider-backed asset search proxy.

use axum::{
    extract::{Query, State},
    response::Response,
};

use crate::{
    AppContext,
    error::Result,
    ports::inbound::rest::types::assets::search::{GetAssetsQuery, YahooSearchResponse},
};

#[utoipa::path(
    get,
    path = "/assets/search",
    params(GetAssetsQuery),
    responses(
        (
            status = 200,
            description = "Provider payload passthrough",
            body = YahooSearchResponse
        )
    )
)]
/// Proxies an asset-name search to the configured market-data provider.
pub async fn get_assets_data(
    State(ctx): State<AppContext>,
    Query(params): Query<GetAssetsQuery>,
) -> Result<Response> {
    Ok(ctx.providers.yahoo.search(params.name).await)
}
