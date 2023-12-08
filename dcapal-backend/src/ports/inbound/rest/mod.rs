//! The [`rest`](self) module implements the REST API of the system

use std::time::Duration;

use crate::app::domain::entity::AssetKind;
use crate::app::infra::utils::Expiring;
use crate::app::services::command::{ConversionRateQuery, ImportPortfolioCmd};
use crate::error::{DcaError, Result};
use crate::ports::outbound::repository::ImportedPortfolio;
use crate::{infra::stats, AppContext};

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::{headers::CacheControl, TypedHeader};
use hyper::StatusCode;
use jsonschema::{Draft, JSONSchema};
use lazy_static::lazy_static;
use metrics::increment_counter;
use serde::{Deserialize, Serialize};

static PORTFOLIO_SCHEMA_STR: &str =
    include_str!("../../../../docs/schema/portfolio/v1/schema.json");

lazy_static! {
    static ref ASSETS_CACHE_CONTROL: CacheControl = CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(5 * 60));
    static ref PORTFOLIO_JSON_SCHEMA: JSONSchema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&serde_json::from_str::<serde_json::Value>(PORTFOLIO_SCHEMA_STR).unwrap())
        .unwrap();
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

#[derive(Debug, Serialize)]
pub struct ImportPortfolioResponse {
    pub id: String,
    pub expires_at: String,
}

impl From<ImportedPortfolio> for ImportPortfolioResponse {
    fn from(value: ImportedPortfolio) -> Self {
        Self {
            id: value.id.simple().to_string(),
            expires_at: value.expires_at.to_string(),
        }
    }
}

pub async fn import_portfolio(
    State(ctx): State<AppContext>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;
    let stats_repo = &ctx.repos.stats;

    let cmd = ImportPortfolioCmd::try_new(payload, &PORTFOLIO_JSON_SCHEMA)?;
    let imported = repo.store_portfolio(&cmd.pfolio).await?;

    increment_counter!(stats::IMPORTED_PORTFOLIOS_TOTAL);
    let _ = stats_repo.increase_imported_portfolio_count().await;

    let response = (
        StatusCode::CREATED,
        Json(ImportPortfolioResponse::from(imported)),
    );

    Ok(response.into_response())
}

pub async fn get_imported_portfolio(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;

    match repo.find_portfolio(&id).await? {
        Some(portfolio) => Ok(Json(portfolio).into_response()),
        None => Ok((StatusCode::NOT_FOUND).into_response()),
    }
}
