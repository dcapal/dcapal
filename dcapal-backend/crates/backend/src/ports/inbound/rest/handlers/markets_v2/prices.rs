//! `GET /markets/v2/prices`: current prices with provenance.

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;

use super::super::cache::quotes_cache_control;
use crate::{
    AppContext,
    app::{commands::lookup_assets::LookupAssetsCommand, infra::claim::Claims},
    ports::inbound::rest::{
        error::V2HttpError,
        extract::LookupAssetRefs,
        types::{
            markets_v2::prices::{PriceV2, PricesV2Response},
            v2_common::V2ErrorResponse,
        },
    },
};

/// Reports the current price of up to 20 assets in the asset's own currency.
///
/// Each asset is priced by its own provider; nothing is converted or substituted.
/// `Cache-Control` allows caching until the stalest usable price crosses its
/// five-minute window, and forbids reuse when any price is missing.
#[utoipa::path(
    get,
    path = "/prices",
    operation_id = "get_prices_v2",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token"),
        ("asset" = Vec<String>, Query, description = "Asset reference as `<provider>:<symbol>` with provider `yf` or `kraken`. Repeat the parameter for several assets (1 to 20 per call). Kraken assets are priced in USD; Yahoo symbols in their listing currency.")
    ),
    responses(
        (status = 200, description = "One price entry per distinct requested asset", body = PricesV2Response),
        (status = 400, description = "Invalid asset references", body = V2ErrorResponse),
        (status = 500, description = "Price storage failure", body = V2ErrorResponse)
    )
)]
pub async fn get_prices(
    State(ctx): State<AppContext>,
    _claims: Claims,
    LookupAssetRefs(raw_assets): LookupAssetRefs,
) -> Result<Response, V2HttpError> {
    let command = LookupAssetsCommand::try_new(&raw_assets)?;
    let quotes = ctx
        .services
        .mkt_data
        .current_prices(command.assets())
        .await?;

    let cache = quotes_cache_control(
        quotes
            .iter()
            .map(|q| q.price.is_some().then_some(q.fetched_at).flatten()),
    );
    let body = PricesV2Response {
        prices: quotes.into_iter().map(PriceV2::from).collect(),
    };
    Ok((TypedHeader(cache), Json(body)).into_response())
}
