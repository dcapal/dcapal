use crate::app::domain::db::fee::FeeStructure;
use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::app::infra::claim::Claims;
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
    pub portfolios: Vec<PortfoliosRequest>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfoliosRequest {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesRequest>,
    pub assets: Vec<PortfolioAssetRequest>,
    pub last_updated_at: DateTime,
}

impl From <PortfoliosRequest> for portfolio::ActiveModel {
    fn from(req: PortfoliosRequest) -> Self {
        portfolio::ActiveModel {
            id: req.id,
            user_id: Default::default(),
            name: req.name,
            currency: req.quote_ccy,
            deleted: false,
            last_updated_at: req.last_updated_at,
            max_fee_impact: req.fees.as_ref().map(|x| x.max_fee_impact).unwrap(),
            fee_structure: req.fees.as_ref().map(|x| x.fee_type),
        }
    }
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

impl From<(PortfoliosRequest, Uuid)> for portfolio::Model {
    fn from((req, user_id): (PortfoliosRequest, Uuid)) -> Self {
        portfolio::Model {
            id: req.id,
            user_id: user_id,
            name: req.name,
            currency: req.quote_ccy,
            deleted: false, // When creating a new portfolio, it is not deleted
            last_updated_at: req.last_updated_at,
            max_fee_impact: req.fees.as_ref().map(|x| x.max_fee_impact).unwrap(),
            fee_structure: req.fees.as_ref().map(|x| x.fee_type),
        }
    }
}

impl From<(PortfolioAssetRequest, Uuid)> for portfolio_asset::Model {
    fn from((req, porfolio_id): (PortfolioAssetRequest, Uuid)) -> Self {
        portfolio_asset::Model {
            id: Default::default(),
            symbol: req.symbol,
            name: req.name,
            asset_class: req.aclass.into(),
            currency: req.base_ccy,
            provider: req.provider,
            quantity: req.qty,
            price: req.price,
            max_fee_impact: req.fees.as_ref().map(|x| x.max_fee_impact),
            target_weight: req.target_weight,
            portfolio_id: porfolio_id,
            fee_structure: req.fees.as_ref().map(|x| x.fee_type),
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
