use crate::domain::command::ConversionRateQuery;
use crate::domain::entity::{Asset, AssetKind, Price};
use crate::error::Result;
use crate::AppContext;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

pub async fn get_assets_fiat(State(ctx): State<AppContext>) -> Result<Json<Vec<Asset>>> {
    let repo = &ctx.repos.mkt_data;

    let assets = repo.load_assets_by_type(AssetKind::Fiat).await?;
    Ok(Json(assets))
}

pub async fn get_assets_crypto(State(ctx): State<AppContext>) -> Result<Json<Vec<Asset>>> {
    let repo = &ctx.repos.mkt_data;

    let assets = repo.load_assets_by_type(AssetKind::Crypto).await?;
    Ok(Json(assets))
}

#[derive(Debug, Deserialize)]
pub struct GetPriceQuery {
    quote: String,
}

pub async fn get_price(
    Path(asset): Path<String>,
    Query(query): Query<GetPriceQuery>,
    State(ctx): State<AppContext>,
) -> Result<Json<Price>> {
    let repo = &ctx.repos.mkt_data;
    let service = &ctx.service;

    let cmd = ConversionRateQuery::try_new(&asset, &query.quote, repo).await?;
    let price = service.get_conversion_rate(cmd).await?;

    Ok(Json(price))
}
