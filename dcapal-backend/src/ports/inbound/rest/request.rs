use crate::app::infra::claim::Claims;
use crate::ports::inbound::rest::FeeStructure;
use crate::{AppContext, DateTime};
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
    pub portfolios: Vec<PortfolioRequest>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioRequest {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesRequest>,
    pub assets: Vec<PortfolioAssetRequest>,
    pub last_updated_at: DateTime,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAssetRequest {
    pub symbol: String,
    pub name: String,
    pub aclass: String,
    pub base_ccy: String,
    pub provider: String,
    pub qty: BigDecimal,
    pub target_weight: BigDecimal,
    pub price: BigDecimal,
    pub fees: Option<TransactionFeesRequest>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesRequest {
    pub max_fee_impact: BigDecimal,
    pub fee_type: FeeStructure,
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
