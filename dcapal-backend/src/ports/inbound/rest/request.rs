use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppContext, DateTime, app::infra::claim::Claims, ports::inbound::rest::FeeStructure};

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
    pub qty: Decimal,
    pub target_weight: Decimal,
    pub price: Decimal,
    pub fees: Option<TransactionFeesRequest>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructure,
}

pub async fn sync_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<SyncPortfoliosRequest>,
) -> crate::error::Result<Response> {
    info!("Syncing portfolios for user_id: {}.", claims.sub);
    match &ctx
        .services
        .portfolio
        .sync_portfolios(claims.sub, req)
        .await
    {
        Ok(resp) => {
            info!(
                "Successfully synced portfolios for user_id: {}.",
                claims.sub
            );
            Ok(Json(resp).into_response())
        }
        Err(e) => {
            error!("Failed to sync portfolios: {} due to: {}.", claims.sub, e);
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}

#[test]
fn test_fee_structure_deserialization() {
    let json = r#"{
        "feeStructure": {
            "type": "variable",
            "feeRate": 0.19,
            "minFee": 2.95
        },
        "maxFeeImpact": 0.5
    }"#;

    let fees: TransactionFeesRequest = serde_json::from_str(json).unwrap();
    assert!(matches!(fees.fee_structure, FeeStructure::Variable { .. }));
}
