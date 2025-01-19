use crate::app::domain::entity::{FeeStructure, Portfolio, PortfolioAsset};
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
use crate::app::domain::db::fee::FeeStructure;

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
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesRequest>,
    pub assets: Vec<PortfolioAssetRequest>,
    pub last_updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesRequest {
    pub max_fee_impact: BigDecimal,
    pub fee_type: FeeStructure,
}

impl From<PortfoliosRequest> for Portfolio {
    fn from(req: PortfoliosRequest) -> Self {
        Portfolio {
            id: req.id,
            name: req.name,
            currency: req.quote_ccy,
            deleted: false, // When creating a new portfolio, it is not deleted
            last_updated_at: req.last_updated_at,
            max_fee_impact: req.fees.as_ref().map(|x| x.max_fee_impact).unwrap(),
            fee_type: req.fees.as_ref().map(|x| x.fee_type).unwrap(),
            fee_amount: req.fees.as_ref().and_then(|x| x.fee_type.fee_amount()),
            fee_rate: req.fees.as_ref().and_then(|x| x.fee_type.fee_rate()),
            min_fee: req.fees.as_ref().and_then(|x| x.fee_type.min_fee()),
            max_fee: req.fees.as_ref().and_then(|x| x.fee_type.max_fee()),
            assets: req.assets.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<PortfolioAssetRequest> for PortfolioAsset {
    fn from(req: PortfolioAssetRequest) -> Self {
        PortfolioAsset {
            symbol: req.symbol,
            name: req.name,
            asset_class: req.aclass.into(),
            base_ccy: req.base_ccy,
            provider: req.provider,
            quantity: req.qty,
            price: req.price,
            max_fee_impact: req.fees.as_ref().map(|x| x.max_fee_impact),
            fee_type: req.fees.as_ref().map(|x| x.fee_type),
            fee_amount: req.fees.as_ref().and_then(|x| x.fee_type.fee_amount()),
            fee_rate: req.fees.as_ref().and_then(|x| x.fee_type.fee_rate()),
            min_fee: req.fees.as_ref().and_then(|x| x.fee_type.min_fee()),
            target_weight: req.target_weight,
            max_fee: req.fees.as_ref().and_then(|x| x.fee_type.max_fee()),
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
