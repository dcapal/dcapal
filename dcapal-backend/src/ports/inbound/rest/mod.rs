//! The [`rest`](self) module implements the REST API of the system

use std::{fmt::Display, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use axum_extra::{TypedHeader, headers::CacheControl};
use hyper::StatusCode;
use lazy_static::lazy_static;
use metrics::counter;
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{
    IntoParams, ToSchema,
    openapi::{Info, OpenApi, Paths},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppContext,
    app::{
        domain::entity::AssetKind,
        infra::utils::Expiring,
        services::command::{ConversionRateQuery, ImportPortfolioCmd},
    },
    error::{DcaError, Result},
    infra::stats,
    ports::outbound::repository::ImportedPortfolio,
};

pub mod openapi;
pub mod proxy_types;
pub mod request;
pub mod response;

static PORTFOLIO_SCHEMA_STR: &str =
    include_str!("../../../../docs/schema/portfolio/v1/schema.json");

lazy_static! {
    static ref ASSETS_CACHE_CONTROL: CacheControl = CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(5 * 60));
    static ref PORTFOLIO_JSON_SCHEMA: serde_json::Value =
        serde_json::from_str(PORTFOLIO_SCHEMA_STR).unwrap();
    static ref PORTFOLIO_SCHEMA_VALIDATOR: jsonschema::Validator =
        jsonschema::draft7::new(&PORTFOLIO_JSON_SCHEMA).unwrap();
}

pub fn build_openapi_router() -> (Router<AppContext>, OpenApi) {
    OpenApiRouter::with_openapi(base_openapi())
        .routes(routes!(root))
        .routes(routes!(get_assets_fiat))
        .routes(routes!(get_assets_crypto))
        .routes(routes!(get_assets_data))
        .routes(routes!(get_assets_chart))
        .routes(routes!(get_price))
        .routes(routes!(import_portfolio))
        .routes(routes!(get_imported_portfolio))
        .nest("/v1", build_v1_openapi_router())
        .split_for_parts()
}

fn build_v1_openapi_router() -> OpenApiRouter<AppContext> {
    OpenApiRouter::new().routes(routes!(request::sync_portfolios))
}

fn base_openapi() -> OpenApi {
    OpenApi::new(
        Info::new("DCA-Pal APIs", env!("CARGO_PKG_VERSION")),
        Paths::new(),
    )
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Service greeting", body = String)
    )
)]
pub async fn root() -> &'static str {
    "Greetings from DCA-Pal APIs!"
}

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
pub async fn get_assets_fiat(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Fiat).await;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}

#[utoipa::path(
    get,
    path = "/assets/crypto",
    responses(
        (
            status = 200,
            description = "Crypto assets",
            body = Vec<crate::app::domain::entity::Asset>
        )
    )
)]
pub async fn get_assets_crypto(State(ctx): State<AppContext>) -> Result<Response> {
    let service = &ctx.services.mkt_data;

    let assets = service.get_assets_by_type(AssetKind::Crypto).await;

    let response = (
        TypedHeader(ASSETS_CACHE_CONTROL.clone()),
        Json((*assets).clone()),
    );

    Ok(response.into_response())
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetAssetsQuery {
    /// Asset name query.
    name: String,
}

#[utoipa::path(
    get,
    path = "/assets/search",
    params(GetAssetsQuery),
    responses(
        (
            status = 200,
            description = "Provider payload passthrough",
            body = proxy_types::YahooSearchResponse
        )
    )
)]
pub async fn get_assets_data(
    State(ctx): State<AppContext>,
    Query(params): Query<GetAssetsQuery>,
) -> Result<Response> {
    Ok(ctx.providers.yahoo.search(params.name).await)
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct GetAssetChartQuery {
    /// Chart start timestamp (unix seconds).
    start_period: i64,
    /// Chart end timestamp (unix seconds).
    end_period: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct GetAssetChartPath {
    /// Asset symbol.
    symbol: String,
}

#[utoipa::path(
    get,
    path = "/assets/chart/{symbol}",
    params(GetAssetChartPath, GetAssetChartQuery),
    responses(
        (
            status = 200,
            description = "Provider payload passthrough",
            body = proxy_types::YahooChartResponse,
            example = json!(
                {
                    "chart": {
                        "error": null,
                        "result": [
                            {
                                "meta": {
                                    "currency": "USD"
                                },
                                "indicators": {
                                    "quote": [
                                        {
                                            "close": [65231.12, 65310.44, 65195.87]
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                }
            )
        )
    )
)]
pub async fn get_assets_chart(
    Path(path): Path<GetAssetChartPath>,
    Query(params): Query<GetAssetChartQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    Ok(ctx
        .providers
        .yahoo
        .chart(path.symbol, params.start_period, params.end_period)
        .await)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPriceQuery {
    /// Quote currency.
    quote: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct GetPricePath {
    /// Base asset symbol.
    asset: String,
}

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

fn cache_control<T: Expiring>(t: &T) -> CacheControl {
    CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(t.time_to_live().as_secs()))
}

#[derive(Debug, Serialize, ToSchema)]
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

#[utoipa::path(
    post,
    path = "/import/portfolio",
    request_body(
        content = serde_json::Value,
        content_type = "application/json",
        description = "Portfolio JSON object validated against dcapal-backend/docs/schema/portfolio/v1/schema.json"
    ),
    responses(
        (status = 201, description = "Imported portfolio metadata", body = ImportPortfolioResponse),
        (status = 400, description = "Input portfolio does not match schema requirements")
    )
)]
pub async fn import_portfolio(
    State(ctx): State<AppContext>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;
    let stats_repo = &ctx.repos.stats;

    let cmd = ImportPortfolioCmd::try_new(payload, &PORTFOLIO_SCHEMA_VALIDATOR)?;
    let imported = repo.store_portfolio(&cmd.pfolio).await?;

    counter!(stats::IMPORTED_PORTFOLIOS_TOTAL).increment(1);
    let _ = stats_repo.increase_imported_portfolio_count().await;

    let response = (
        StatusCode::CREATED,
        Json(ImportPortfolioResponse::from(imported)),
    );

    Ok(response.into_response())
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct GetImportedPortfolioPath {
    /// Imported portfolio id.
    id: String,
}

#[utoipa::path(
    get,
    path = "/import/portfolio/{id}",
    params(GetImportedPortfolioPath),
    responses(
        (status = 200, description = "Imported portfolio payload", body = serde_json::Value),
        (status = 404, description = "Portfolio not found or expired")
    )
)]
pub async fn get_imported_portfolio(
    Path(path): Path<GetImportedPortfolioPath>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;

    match repo.find_portfolio(&path.id).await? {
        Some(portfolio) => Ok(Json(portfolio).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FeeStructure {
    #[serde(rename = "zeroFee")]
    ZeroFee,

    #[serde(rename = "fixed")]
    Fixed {
        #[serde(rename = "feeAmount", with = "rust_decimal::serde::float")]
        fee_amount: Decimal,
    },

    #[serde(rename = "variable")]
    Variable {
        #[serde(rename = "feeRate", with = "rust_decimal::serde::float")]
        fee_rate: Decimal,
        #[serde(rename = "minFee", with = "rust_decimal::serde::float")]
        min_fee: Decimal,
        #[serde(
            rename = "maxFee",
            default,
            skip_serializing_if = "Option::is_none",
            with = "rust_decimal::serde::float_option"
        )]
        max_fee: Option<Decimal>,
    },
}

impl Display for FeeStructure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeeStructure::ZeroFee => write!(f, "ZeroFee"),
            FeeStructure::Fixed { .. } => write!(f, "Fixed"),
            FeeStructure::Variable { .. } => write!(f, "Variable"),
        }
    }
}
