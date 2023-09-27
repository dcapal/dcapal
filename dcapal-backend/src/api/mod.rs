use std::time::Duration;

use crate::domain::command::ConversionRateQuery;
use crate::domain::entity::AssetKind;
use crate::domain::utils::Expiring;
use crate::error::{DcaError, Result};
use crate::AppContext;

use axum::extract::{Path, Query, State};
use axum::headers::CacheControl;
use axum::response::{IntoResponse, Response};
use axum::{Json, TypedHeader};
use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    static ref ASSETS_CACHE_CONTROL: CacheControl = CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(5 * 60));
}

pub async fn get_assets_fiat(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Fiat).await;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}

pub async fn get_assets_crypto(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Crypto).await;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}

#[derive(Debug, Deserialize)]
pub struct GetPriceQuery {
    quote: String,
}

pub async fn get_price(
    Path(asset): Path<String>,
    Query(query): Query<GetPriceQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let repo = &ctx.repos.mkt_data;
    let service = &ctx.services.mkt_data;

    let cmd = ConversionRateQuery::try_new(&asset, &query.quote, repo).await?;
    let (base, quote) = (cmd.base.id().clone(), cmd.quote.id().clone());

    let price = service
        .get_conversion_rate(cmd)
        .await?
        .ok_or(DcaError::PriceNotAvailable(base, quote))?;

    let response = (TypedHeader(cache_control(&price)), Json(price));
    Ok(response.into_response())
}

fn cache_control<T: Expiring>(t: &T) -> CacheControl {
    CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(t.time_to_live().as_secs()))
}
