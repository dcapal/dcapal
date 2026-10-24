//! Wire types of the v2 asset lookup endpoint.

use serde::Serialize;
use utoipa::ToSchema;

use super::sync_portfolio_v2::{AssetRefV2, v2_asset_class_schema, v2_provider_schema};
use crate::app::{domain::asset::ResolvedAsset, services::asset::AssetLookup};

/// Shared metadata of one cached asset returned by the lookup.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAssetV2Response {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    #[schema(schema_with = v2_asset_class_schema)]
    pub aclass: String,
}

/// Cached metadata of the looked-up assets and the references that are not cached.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LookupAssetsV2Response {
    pub assets: Vec<ResolvedAssetV2Response>,
    pub not_found: Vec<AssetRefV2>,
}

impl From<ResolvedAsset> for ResolvedAssetV2Response {
    fn from(asset: ResolvedAsset) -> Self {
        Self {
            provider: asset.provider.as_str().to_string(),
            symbol: asset.symbol,
            name: asset.name,
            currency: asset.currency,
            aclass: asset.asset_class.as_str().to_string(),
        }
    }
}

impl From<AssetLookup> for LookupAssetsV2Response {
    fn from(lookup: AssetLookup) -> Self {
        Self {
            assets: lookup.assets.into_iter().map(Into::into).collect(),
            not_found: lookup.not_found.into_iter().map(Into::into).collect(),
        }
    }
}
