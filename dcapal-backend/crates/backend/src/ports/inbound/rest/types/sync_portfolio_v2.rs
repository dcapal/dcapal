use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{
    ToSchema,
    openapi::schema::{Object, ObjectBuilder, SchemaType, Type},
};
use uuid::Uuid;

use crate::app::{commands::sync_portfolios_v2::ValidationError, domain::portfolio as app};

mod conversion;

/// Versioned full-aggregate synchronization request.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncPortfoliosV2Request {
    /// The request schema version. v2 requests must use `2`.
    #[schema(schema_with = v2_schema_version_schema)]
    pub schema_version: u16,
    /// Complete Portfolio aggregates to compare with server state.
    pub portfolios: Vec<PortfolioV2Request>,
    /// Portfolio tombstones requested by the client.
    pub deleted_portfolios: Vec<Uuid>,
}

/// A complete saved Portfolio aggregate exchanged by the v2 sync endpoint.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioV2Request {
    pub id: Uuid,
    pub name: String,
    pub quote_currency: String,
    pub allocation_mode: AllocationModeWire,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    #[schema(value_type = Option<String>)]
    pub drift_band: Option<Decimal>,
    pub fees: Option<TransactionFeesV2Request>,
    pub assets: Vec<PortfolioAssetV2Request>,
    pub strategic_allocation: Option<StrategicAllocationV2Request>,
    pub last_updated_at: crate::DateTime,
}

/// The two mutually exclusive Portfolio allocation modes.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AllocationModeWire {
    Simple,
    Strategic,
}

impl AllocationModeWire {
    /// Returns the lower-case API value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Strategic => "strategic",
        }
    }
}

/// Shared Provider metadata carried inside a Portfolio Asset.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SharedAssetV2Request {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    #[schema(schema_with = v2_asset_class_schema)]
    pub aclass: String,
}

/// Portfolio-specific holding and allocation values.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAssetV2Request {
    pub id: Uuid,
    pub asset: SharedAssetV2Request,
    #[serde(default)]
    #[schema(schema_with = v2_optional_asset_class_schema)]
    pub aclass_override: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    #[schema(value_type = Option<String>)]
    pub target_weight: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    #[schema(value_type = Option<String>)]
    pub manual_price: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    #[schema(value_type = Option<String>)]
    pub average_buy_price: Option<Decimal>,
    pub fees: Option<TransactionFeesV2Request>,
}

/// Strategic class and relative Portfolio Asset targets.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAllocationV2Request {
    pub asset_class_targets: Vec<StrategicAssetClassTargetV2Request>,
    pub asset_targets: Vec<StrategicAssetTargetV2Request>,
}

/// An absolute target for one Asset Class.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAssetClassTargetV2Request {
    #[schema(schema_with = v2_asset_class_schema)]
    pub asset_class: String,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub target_weight: Decimal,
}

/// A relative target for one Portfolio Asset inside its effective class.
#[derive(Debug, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAssetTargetV2Request {
    pub portfolio_asset_id: Uuid,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub relative_target_weight: Decimal,
}

/// Portfolio- or asset-level transaction fees in the v2 contract.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesV2Request {
    #[serde(default, with = "rust_decimal::serde::str_option")]
    #[schema(value_type = Option<String>)]
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructureV2,
}

/// Fee policy shape used by the v2 aggregate.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum FeeStructureV2 {
    #[serde(rename = "zeroFee")]
    ZeroFee,
    #[serde(rename = "fixed")]
    Fixed {
        #[serde(rename = "feeAmount", with = "rust_decimal::serde::str")]
        #[schema(value_type = String)]
        fee_amount: Decimal,
    },
    #[serde(rename = "variable")]
    Variable {
        #[serde(rename = "feeRate", with = "rust_decimal::serde::str")]
        #[schema(value_type = String)]
        fee_rate: Decimal,
        #[serde(rename = "minFee", default, with = "rust_decimal::serde::str_option")]
        #[schema(value_type = Option<String>)]
        min_fee: Option<Decimal>,
        #[serde(rename = "maxFee", default, with = "rust_decimal::serde::str_option")]
        #[schema(value_type = Option<String>)]
        max_fee: Option<Decimal>,
    },
}

/// Canonical aggregate synchronization response.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncPortfoliosV2Response {
    pub updated_portfolios: Vec<PortfolioV2Response>,
    pub deleted_portfolios: Vec<Uuid>,
}

/// Canonical Portfolio returned by v2 synchronization.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioV2Response {
    pub id: Uuid,
    pub name: String,
    pub quote_currency: String,
    pub allocation_mode: AllocationModeWire,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::str_option"
    )]
    #[schema(value_type = Option<String>)]
    pub drift_band: Option<Decimal>,
    pub fees: Option<TransactionFeesV2Response>,
    pub assets: Vec<PortfolioAssetV2Response>,
    pub strategic_allocation: Option<StrategicAllocationV2Response>,
    pub last_updated_at: crate::DateTime,
}

/// Canonical Portfolio Asset returned by v2 synchronization.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAssetV2Response {
    pub id: Uuid,
    pub asset: SharedAssetV2Response,
    #[schema(schema_with = v2_optional_asset_class_schema)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aclass_override: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::str_option"
    )]
    #[schema(value_type = Option<String>)]
    pub target_weight: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::str_option"
    )]
    #[schema(value_type = Option<String>)]
    pub manual_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::str_option"
    )]
    #[schema(value_type = Option<String>)]
    pub average_buy_price: Option<Decimal>,
    pub fees: Option<TransactionFeesV2Response>,
}

/// Shared asset metadata returned by v2.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SharedAssetV2Response {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    #[schema(schema_with = v2_asset_class_schema)]
    pub aclass: String,
}

/// Strategic targets returned by v2.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAllocationV2Response {
    pub asset_class_targets: Vec<StrategicAssetClassTargetV2Response>,
    pub asset_targets: Vec<StrategicAssetTargetV2Response>,
}

/// One persisted Strategic Asset Class target returned by v2.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAssetClassTargetV2Response {
    #[schema(schema_with = v2_asset_class_schema)]
    pub asset_class: String,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub target_weight: Decimal,
}

/// One persisted relative Portfolio Asset target returned by v2.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrategicAssetTargetV2Response {
    pub portfolio_asset_id: Uuid,
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String)]
    pub relative_target_weight: Decimal,
}

/// v2 fee settings returned by the API.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesV2Response {
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::str_option"
    )]
    #[schema(value_type = Option<String>)]
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructureV2,
}

/// One readable v2 validation issue.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
pub struct V2FieldError {
    pub field: String,
    pub message: String,
}

/// Structured v2 error response.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
pub struct V2ErrorResponse {
    pub code: String,
    pub message: String,
    pub errors: Vec<V2FieldError>,
}

/// Service-level validation failure used by the v2 REST boundary.
#[derive(Debug, thiserror::Error)]
#[error("{response_message}")]
pub struct V2ValidationError {
    pub response_code: &'static str,
    pub response_message: &'static str,
    pub errors: Vec<V2FieldError>,
}

const VALIDATION_CODE: &str = "validation_error";
const VALIDATION_MESSAGE: &str = "The Portfolio aggregate is invalid.";
const ASSET_CLASS_ENUM_MESSAGE: &str =
    "must be one of equity, bond, cash, crypto, commodity, other";
const KNOWN_ASSET_CLASS_MESSAGE: &str = "must be a known Asset Class";

/// A fallible conversion failure at the REST-to-application boundary.
#[derive(Debug, thiserror::Error)]
#[error("the v2 request contains invalid typed values")]
pub struct V2ConversionError {
    /// Field-level conversion issues with paths relative to the converted value.
    pub errors: Vec<V2FieldError>,
}

impl V2ValidationError {
    /// Creates a validation failure with one field issue.
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            response_code: VALIDATION_CODE,
            response_message: VALIDATION_MESSAGE,
            errors: vec![V2FieldError {
                field: field.into(),
                message: message.into(),
            }],
        }
    }

    /// Converts the service error into the wire response body.
    pub fn response(&self) -> V2ErrorResponse {
        V2ErrorResponse {
            code: self.response_code.to_string(),
            message: self.response_message.to_string(),
            errors: self.errors.clone(),
        }
    }
}

fn v2_provider_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(["yf", "kraken"]))
        .build()
}

fn v2_schema_version_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::Integer)
        .minimum(Some(2))
        .maximum(Some(2))
        .enum_values(Some([2]))
        .build()
}

fn v2_asset_class_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some([
            "equity",
            "bond",
            "cash",
            "crypto",
            "commodity",
            "other",
        ]))
        .build()
}

fn v2_optional_asset_class_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(
            [Type::String, Type::Null]
                .into_iter()
                .collect::<SchemaType>(),
        )
        .enum_values(Some(vec![
            serde_json::Value::String("equity".to_string()),
            serde_json::Value::String("bond".to_string()),
            serde_json::Value::String("cash".to_string()),
            serde_json::Value::String("crypto".to_string()),
            serde_json::Value::String("commodity".to_string()),
            serde_json::Value::String("other".to_string()),
            serde_json::Value::Null,
        ]))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn v2_decimals_are_strings_on_the_wire() {
        // GIVEN a v2 request whose decimals are JSON strings, WHEN it is decoded,
        // THEN the exact decimal values and null overrides are preserved.
        let request = serde_json::json!({
            "schemaVersion": 2,
            "portfolios": [{
                "id": Uuid::nil(),
                "name": "Draft",
                "quoteCurrency": "EUR",
                "allocationMode": "simple",
                "driftBand": "1.25",
                "fees": null,
                "assets": [{
                    "id": Uuid::nil(),
                    "asset": {"provider": "yf", "symbol": "VWCE", "name": "ETF", "currency": "EUR", "aclass": "equity"},
                    "aclassOverride": null,
                    "quantity": "1.20",
                    "targetWeight": "100.00",
                    "manualPrice": "100.10",
                    "averageBuyPrice": null,
                    "fees": null
                }],
                "strategicAllocation": null,
                "lastUpdatedAt": "2026-01-01T00:00:00Z"
            }],
            "deletedPortfolios": []
        });

        let parsed: SyncPortfoliosV2Request = serde_json::from_value(request).unwrap();
        assert_eq!(parsed.schema_version, 2);
        assert_eq!(parsed.portfolios[0].assets[0].quantity, dec!(1.20));
        assert_eq!(parsed.portfolios[0].assets[0].aclass_override, None);
        assert_eq!(parsed.portfolios[0].drift_band, Some(dec!(1.25)));
    }

    #[test]
    fn v2_fee_policy_uses_camel_case_and_rejects_irrelevant_fields() {
        // GIVEN variable and fixed fee policies, WHEN they are decoded and
        // encoded, THEN camelCase keys are used and fields foreign to the type are rejected.
        // GIVEN v2 fee policies use the canonical JSON names, WHEN they are
        // parsed and serialized, THEN optional bounds round-trip and fields
        // from another policy kind are rejected.
        let variable: TransactionFeesV2Request = serde_json::from_value(serde_json::json!({
            "feeStructure": {"type": "variable", "feeRate": "0.19"}
        }))
        .expect("variable fee policy should parse without optional bounds");
        let serialized = serde_json::to_value(&variable).expect("fee policy should serialize");
        assert_eq!(serialized["feeStructure"]["feeRate"], "0.19");
        assert!(serialized["feeStructure"].get("fee_rate").is_none());

        let invalid = serde_json::from_value::<TransactionFeesV2Request>(serde_json::json!({
            "feeStructure": {"type": "fixed", "feeAmount": "1.25", "feeRate": "0.19"}
        }));
        assert!(invalid.is_err());
    }

    #[test]
    fn wire_conversion_reports_typed_field_errors() {
        // GIVEN a wire request with an unknown asset class, WHEN it is converted
        // to the application type, THEN the error names the nested field path.
        // GIVEN a decoded request with an unknown Asset Class, WHEN the REST
        // DTO is converted to application data, THEN conversion fails with
        // the nested field location instead of leaking a wire type inward.
        let mut request: SyncPortfoliosV2Request = serde_json::from_value(serde_json::json!({
            "schemaVersion": 2,
            "portfolios": [{
                "id": Uuid::nil(),
                "name": "Draft",
                "quoteCurrency": "EUR",
                "allocationMode": "simple",
                "fees": null,
                "assets": [{
                    "id": Uuid::nil(),
                    "asset": {"provider": "yf", "symbol": "VWCE", "name": "ETF", "currency": "EUR", "aclass": "equity"},
                    "quantity": "1",
                    "targetWeight": "100",
                    "fees": null
                }],
                "strategicAllocation": null,
                "lastUpdatedAt": "2026-01-01T00:00:00Z"
            }],
            "deletedPortfolios": []
        }))
        .expect("wire request should decode");
        request.portfolios[0].assets[0].asset.aclass = "unknown".to_string();

        let error = app::SyncPortfoliosV2Request::try_from(request)
            .expect_err("unknown class should fail conversion");
        assert_eq!(
            error.errors[0].field,
            "portfolios[0].assets[0].asset.aclass"
        );
    }

    #[test]
    fn unsupported_provider_assets_can_be_filtered_before_semantic_validation() {
        // GIVEN an unsupported Provider Asset with fields that only the
        // synchronization command can validate, WHEN the wire DTO is
        // converted, THEN it remains representable so the command can drop
        // it and its dependent data without rejecting the whole aggregate.
        let asset = PortfolioAssetV2Request {
            id: Uuid::now_v7(),
            asset: SharedAssetV2Request {
                provider: "ibkr".to_string(),
                symbol: String::new(),
                name: String::new(),
                currency: String::new(),
                aclass: "not-a-class".to_string(),
            },
            aclass_override: Some("not-a-class".to_string()),
            quantity: Decimal::NEGATIVE_ONE,
            target_weight: Some(Decimal::from(101)),
            manual_price: Some(Decimal::ZERO),
            average_buy_price: None,
            fees: None,
        };

        let converted = app::PortfolioAssetV2::try_from(asset).expect("unsupported asset");
        assert!(matches!(
            converted.asset.provider,
            app::Provider::Unsupported(_)
        ));
        assert_eq!(converted.asset.asset_class, app::AssetClass::Other);
        assert_eq!(converted.asset_class_override, None);
    }
}
