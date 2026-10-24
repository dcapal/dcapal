//! Wire types of the v2 asset lookup endpoint.

use serde::Serialize;
use utoipa::ToSchema;

use super::sync_portfolio_v2::{SharedAssetV2Response, v2_provider_schema};
use crate::app::{
    domain::asset::{AssetRef, ResolvedAsset},
    services::asset::AssetLookup,
};

/// Reference to an asset that is not in the backend cache.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetRefV2Response {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub symbol: String,
}

/// Cached metadata of the looked-up assets and the references that are not cached.
#[derive(Debug, Serialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LookupAssetsV2Response {
    pub assets: Vec<SharedAssetV2Response>,
    pub not_found: Vec<AssetRefV2Response>,
}

impl From<ResolvedAsset> for SharedAssetV2Response {
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

impl From<AssetRef> for AssetRefV2Response {
    fn from(asset: AssetRef) -> Self {
        Self {
            provider: asset.provider.as_str().to_string(),
            symbol: asset.symbol,
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
