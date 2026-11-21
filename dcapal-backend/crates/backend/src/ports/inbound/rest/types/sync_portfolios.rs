//! Wire types of the legacy v1 `POST /v1/sync/portfolios` endpoint.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{
    ToSchema,
    openapi::schema::{Object, ObjectBuilder, Type},
};
use uuid::Uuid;

use crate::DateTime;

mod conversion;

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
    #[schema(value_type = String, format = DateTime)]
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

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// The result of synchronizing the client's portfolio set.
pub struct SyncPortfoliosResponse {
    /// Portfolios whose server state should be applied by the client.
    pub updated_portfolios: Vec<PortfolioResponse>,
    /// Portfolio identifiers that the client should remove.
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// A portfolio returned by the synchronization endpoint.
pub struct PortfolioResponse {
    /// The portfolio identifier.
    pub id: Uuid,
    /// The portfolio display name.
    pub name: String,
    /// The portfolio quote currency.
    pub quote_ccy: String,
    /// The portfolio-level transaction fee settings.
    pub fees: Option<TransactionFeesResponse>,
    /// The assets held by the portfolio.
    pub assets: Vec<PortfolioAssetResponse>,
    /// The timestamp used to compare this state with a client copy.
    #[schema(value_type = String, format = DateTime)]
    pub last_updated_at: DateTime,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// An asset held by a portfolio.
///
/// Decimal values are serialized as JSON strings to preserve precision.
pub struct PortfolioAssetResponse {
    /// The provider symbol for the asset.
    pub symbol: String,
    /// The asset display name.
    pub name: String,
    /// The asset class.
    #[schema(schema_with = v1_response_asset_class_schema)]
    pub aclass: String,
    /// The asset currency.
    pub base_ccy: String,
    /// The data provider for the asset.
    #[schema(schema_with = v1_response_provider_schema)]
    pub provider: String,
    #[serde(with = "rust_decimal::serde::str")]
    /// The quantity held.
    pub qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The target portfolio weight.
    pub target_weight: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The latest known price.
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The average price paid for the holding.
    pub average_buy_price: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The asset-level transaction fee settings.
    pub fees: Option<TransactionFeesResponse>,
}

/// Documents the canonical provider names emitted by the v1 synchronization response.
fn v1_response_provider_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(["DCAPal", "YF"]))
        .description(Some(
            "Canonical v1 provider name. Kraken-backed assets are returned as DCAPal and Yahoo Finance assets as YF.",
        ))
        .build()
}

/// Documents the canonical Asset Class names emitted by the v1 synchronization response.
fn v1_response_asset_class_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some([
            "EQUITY",
            "BOND",
            "CURRENCY",
            "CRYPTO",
            "COMMODITY",
            "OTHER",
        ]))
        .description(Some(
            "Canonical v1 Asset Class name. Cash and Currency are returned as CURRENCY.",
        ))
        .build()
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Transaction fee settings returned by the API.
pub struct TransactionFeesResponse {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "rust_decimal::serde::str_option"
    )]
    /// The maximum allowed fee impact, when configured.
    pub max_fee_impact: Option<Decimal>,
    /// The fee calculation model.
    pub fee_structure: FeeStructure,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
/// Transaction-fee policy exchanged by the REST API.
///
/// Decimal values are serialized as strings so clients do not lose precision
/// while crossing the JSON boundary.
pub enum FeeStructure {
    #[serde(rename = "zeroFee")]
    ZeroFee,

    #[serde(rename = "fixed")]
    Fixed {
        #[serde(rename = "feeAmount", with = "rust_decimal::serde::str")]
        fee_amount: Decimal,
    },

    #[serde(rename = "variable")]
    Variable {
        #[serde(rename = "feeRate", with = "rust_decimal::serde::str")]
        fee_rate: Decimal,
        #[serde(rename = "minFee", with = "rust_decimal::serde::str")]
        min_fee: Decimal,
        #[serde(
            rename = "maxFee",
            default,
            skip_serializing_if = "Option::is_none",
            with = "rust_decimal::serde::str_option"
        )]
        max_fee: Option<Decimal>,
    },
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
