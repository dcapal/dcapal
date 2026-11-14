//! `GET /price/{asset}`: current conversion price.

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;

use crate::{
    AppContext,
    app::services::command::ConversionRateQuery,
    error::{DcaError, Result},
    ports::inbound::rest::types::price::{GetPricePath, GetPriceQuery},
};

use super::cache::cache_control;

#[utoipa::path(
    get,
    path = "/price/{asset}",
    params(GetPricePath, GetPriceQuery),
    responses(
        (
            status = 200,
            description = "Asset conversion price",
            body = crate::app::domain::entity::Price
        ),
        (status = 404, description = "Price not available")
    )
)]
/// Returns the current conversion price for a base asset and quote currency.
pub async fn get_price(
    Path(path): Path<GetPricePath>,
    Query(query): Query<GetPriceQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let repo = &ctx.repos.mkt_data;
    let service = &ctx.services.mkt_data;

    let cmd = ConversionRateQuery::try_new(&path.asset, &query.quote, repo).await?;
    let (base, quote) = (cmd.base.id().clone(), cmd.quote.id().clone());

    let price = service
        .get_conversion_rate(cmd)
        .await?
        .ok_or(DcaError::PriceNotAvailable(base, quote))?;

    let response = (TypedHeader(cache_control(&price)), Json(price));
    Ok(response.into_response())
}
