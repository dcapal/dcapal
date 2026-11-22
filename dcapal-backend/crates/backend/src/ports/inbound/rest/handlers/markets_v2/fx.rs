//! `GET /markets/v2/fx`: FX rate with provenance.

use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;

use super::super::cache::quotes_cache_control;
use crate::{
    AppContext,
    app::{commands::market_quotes::FxRateCommand, infra::claim::Claims},
    ports::inbound::rest::{
        error::V2HttpError,
        types::{
            markets_v2::fx::{FxQuery, FxRateV2Response},
            v2_common::V2ErrorResponse,
        },
    },
};

/// Reports the rate between two catalog fiat currencies.
///
/// Clients convert values themselves (price times rate) and should cache each pair
/// for the `Cache-Control` max-age, which matches the price validity window.
#[utoipa::path(
    get,
    path = "/fx",
    operation_id = "get_fx_rate_v2",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token"),
        FxQuery
    ),
    responses(
        (status = 200, description = "FX rate or the reason it is missing", body = FxRateV2Response),
        (status = 400, description = "Missing or unsupported currency", body = V2ErrorResponse),
        (status = 500, description = "Price storage failure", body = V2ErrorResponse)
    )
)]
pub async fn get_fx_rate(
    State(ctx): State<AppContext>,
    _claims: Claims,
    Query(query): Query<FxQuery>,
) -> Result<Response, V2HttpError> {
    let command = FxRateCommand::try_new(query.base.as_deref(), query.quote.as_deref())?;
    let rate = ctx
        .services
        .mkt_data
        .fx_rate(&command.base, &command.quote)
        .await?;

    let cache = quotes_cache_control([rate.rate.and(rate.fetched_at)]);
    Ok((TypedHeader(cache), Json(FxRateV2Response::from(rate))).into_response())
}
