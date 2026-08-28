use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::info;
use utoipa::{
    ToSchema,
    openapi::schema::{Object, ObjectBuilder, Type},
};
use uuid::Uuid;

use crate::{AppContext, DateTime, app::infra::claim::Claims, ports::inbound::rest::FeeStructure};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Authenticated request containing local portfolio changes and deletions.
pub struct SyncPortfoliosRequest {
    pub portfolios: Vec<PortfolioRequest>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
/// A portfolio submitted as part of an authenticated synchronization.
pub struct PortfolioRequest {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesRequest>,
    pub assets: Vec<PortfolioAssetRequest>,
    pub last_updated_at: DateTime,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
/// An asset and its allocation values submitted for synchronization.
pub struct PortfolioAssetRequest {
    pub symbol: String,
    pub name: String,
    #[schema(schema_with = v1_asset_class_schema)]
    pub aclass: String,
    pub base_ccy: String,
    #[schema(schema_with = v1_provider_schema)]
    pub provider: String,
    pub qty: Decimal,
    pub target_weight: Decimal,
    pub price: Decimal,
    pub average_buy_price: Decimal,
    pub fees: Option<TransactionFeesRequest>,
}

/// Documents the provider aliases accepted by the v1 synchronization request.
fn v1_provider_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(["DCAPal", "Kraken", "YF", "Yahoo"]))
        .description(Some(
            "Provider matching is case-insensitive; DCAPal and Kraken map to Kraken, while YF and Yahoo map to YF.",
        ))
        .build()
}

/// Documents the Asset Class aliases accepted by the v1 synchronization request.
fn v1_asset_class_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some([
            "EQUITY",
            "BOND",
            "CURRENCY",
            "CASH",
            "CRYPTO",
            "COMMODITY",
            "OTHER",
            "Equity",
            "Bond",
            "Currency",
            "Cash",
            "Crypto",
            "Commodity",
            "Other",
        ]))
        .description(Some(
            "Asset Class matching is case-insensitive; Currency and Cash map to CURRENCY, and unknown values map to OTHER.",
        ))
        .build()
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
/// Portfolio- or asset-level transaction-fee settings.
pub struct TransactionFeesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructure,
}

#[utoipa::path(
    post,
    path = "/sync/portfolios",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token")
    ),
    request_body = SyncPortfoliosRequest,
    responses(
        (
            status = 200,
            description = "Portfolios synchronized",
            body = crate::ports::inbound::rest::response::SyncPortfoliosResponse
        ),
        (status = 400, description = "Bad request")
    )
)]
/// Applies authenticated local portfolio changes and returns the server state.
pub async fn sync_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<SyncPortfoliosRequest>,
) -> crate::error::Result<Response> {
    info!("Syncing portfolios for user_id: {}.", claims.sub);
    let resp = ctx
        .services
        .portfolio
        .sync_portfolios(claims.sub, req)
        .await
        .map_err(crate::error::DcaError::from)?;
    info!(
        "Successfully synced portfolios for user_id: {}.",
        claims.sub
    );
    Ok(Json(resp).into_response())
}

#[test]
fn test_fee_structure_deserialization() {
    let json = r#"{
        "feeStructure": {
            "type": "variable",
            "feeRate": "0.19",
            "minFee": "2.95"
        },
        "maxFeeImpact": "0.5"
    }"#;

    let fees: TransactionFeesRequest = serde_json::from_str(json).unwrap();
    assert!(matches!(fees.fee_structure, FeeStructure::Variable { .. }));
}

#[cfg(test)]
mod sync_error_tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ports::outbound::repository::portfolio::PortfolioRepositoryError;
    use crate::{app::services::portfolio::PortfolioServiceError, error::DcaError};

    #[test]
    fn repository_failure_is_an_internal_server_error() {
        // GIVEN a repository failure, WHEN the REST boundary maps it,
        // THEN it returns HTTP 500 without exposing persistence details.
        let error = DcaError::from(PortfolioServiceError::Repository(
            PortfolioRepositoryError::Database(sqlx::Error::RowNotFound),
        ));
        let source_types = {
            let sources: Vec<_> = error.iter_sources().collect();
            (
                sources.len(),
                sources[0].downcast_ref::<PortfolioServiceError>().is_some(),
                sources[1]
                    .downcast_ref::<PortfolioRepositoryError>()
                    .is_some(),
                sources[2].downcast_ref::<sqlx::Error>().is_some(),
            )
        };
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(source_types, (3, true, true, true));
    }

    #[test]
    fn ownership_failure_is_a_bad_request() {
        // GIVEN a sync update for a portfolio owned by another user, WHEN the REST boundary maps it,
        // THEN it returns HTTP 400 rather than treating the client conflict as a server failure.
        let error = DcaError::from(PortfolioServiceError::CannotUpdate(
            PortfolioRepositoryError::CannotUpdate,
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn response_conversion_failure_is_an_internal_server_error() {
        // GIVEN a missing manual-price compatibility failure, WHEN the REST boundary maps it,
        // THEN it returns HTTP 500 without exposing internal conversion details.
        let error = DcaError::from(PortfolioServiceError::ResponseConversion(
            DcaError::Generic("v1 Portfolio Asset response requires a manual price.".to_string()),
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
