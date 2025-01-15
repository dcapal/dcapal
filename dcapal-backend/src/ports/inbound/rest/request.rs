use crate::app::domain::entity::{Portfolio, PortfolioAsset};
use crate::app::infra::claim::Claims;
use crate::AppContext;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct PortfolioHoldingsResponse {
    pub holdings: Vec<PortfolioResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct PortfolioResponse {
    pub name: String,
    pub ticker: String,
    pub price: f64,
    pub quantity: f64,
    pub weight: f64,
    pub total: f64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncPortfoliosRequest {
    pub portfolios: Vec<PortfoliosRequest>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortfoliosRequest {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub assets: Vec<PortfolioHoldingsRequest>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHoldingsRequest {
    pub symbol: String,
    pub name: String,
    pub aclass: String,
    pub base_ccy: String,
    pub provider: String,
    pub qty: BigDecimal,
    pub target_weight: BigDecimal,
    pub price: BigDecimal,
}

impl From<PortfoliosRequest> for Portfolio {
    fn from(req: PortfoliosRequest) -> Self {
        Portfolio {
            id: req.id,
            name: req.name,
            description: req.description,
            currency: req.currency,
            assets: req.assets.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<PortfolioHoldingsRequest> for PortfolioAsset {
    fn from(req: PortfolioHoldingsRequest) -> Self {
        PortfolioAsset {
            symbol: req.symbol,
            name: req.name,
            quantity: req.qty,
            weight: req.target_weight,
            price: req.price,
        }
    }
}

impl From<&PortfolioAsset> for PortfolioHoldingsRequest {
    fn from(holding: &PortfolioAsset) -> Self {
        PortfolioHoldingsRequest {
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            quantity: holding.quantity.clone(),
            price: holding.price.clone(),
        }
    }
}

pub async fn sync_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<SyncPortfoliosRequest>,
) -> crate::error::Result<Response> {
    info!("Update profile user_id: {}.", claims.sub);
    match &ctx
        .services
        .portfolio
        .sync_portfolios(claims.sub, req.into())
        .await
    {
        Ok(resp) => {
            info!("Success update profile user user_id: {}.", claims.sub);
            Ok(Json(resp).into_response())
        }
        Err(e) => {
            info!("Unsuccessful update profile user: {e:?}");
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}
