//! Wire types and schema helpers shared by the v2 REST endpoints.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use utoipa::{
    ToSchema,
    openapi::schema::{Object, ObjectBuilder, Type},
};

use crate::app::domain::portfolio as app;

/// Reference to a provider-listed asset; shared metadata is resolved server-side.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetRefV2 {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    /// Provider symbol; stored trimmed and upper-cased.
    pub symbol: String,
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

/// Validation failure reported by the v2 REST boundary as a 400 response.
#[derive(Debug, Default, thiserror::Error)]
#[error("The Portfolio aggregate is invalid.")]
pub struct V2ValidationError {
    pub errors: Vec<V2FieldError>,
}

/// Error code of every v2 validation failure.
pub(crate) const VALIDATION_CODE: &str = "validation_error";
const VALIDATION_MESSAGE: &str = "The Portfolio aggregate is invalid.";
pub(super) const KNOWN_ASSET_CLASS_MESSAGE: &str = "must be a known Asset Class";

/// Validation message listing the accepted provider identifiers.
pub(super) fn known_provider_message() -> &'static str {
    static MESSAGE: LazyLock<String> =
        LazyLock::new(|| format!("must be one of {}", app::Provider::names()));
    MESSAGE.as_str()
}

impl V2ValidationError {
    /// Converts the validation failure into the wire response body.
    pub fn response(self) -> V2ErrorResponse {
        V2ErrorResponse {
            code: VALIDATION_CODE.to_string(),
            message: VALIDATION_MESSAGE.to_string(),
            errors: self.errors,
        }
    }
}

pub(crate) fn v2_provider_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(
            app::Provider::iter().map(|provider| provider.as_str()),
        ))
        .build()
}

/// Asset-class identifiers in schema order: `other` is documented last.
pub(super) fn asset_class_names() -> impl Iterator<Item = &'static str> {
    app::AssetClass::iter()
        .filter(|class| *class != app::AssetClass::Other)
        .chain([app::AssetClass::Other])
        .map(app::AssetClass::as_str)
}

pub(crate) fn v2_asset_class_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(asset_class_names()))
        .build()
}
