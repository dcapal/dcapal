//! Wire types of `GET /markets/v2/assets/{provider}/search`.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    app::services::asset_search::{AssetSearch, AssetSearchHit, PriceAvailability, SearchOutcome},
    ports::inbound::rest::types::{
        markets_v2::common::{FailureV2, provider_name},
        v2_common::{v2_asset_class_schema, v2_provider_schema},
    },
};

/// Path parameter of the search endpoint.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct SearchPath {
    /// Provider whose listings are searched: `yf` or `kraken`.
    pub provider: String,
}

/// Query parameters of the search endpoint.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    /// Free text matched against symbols and names, at most 100 characters. Required.
    pub query: Option<String>,
}

/// Whether a price is expected to be available for a found asset.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PriceAvailabilityV2 {
    /// A price is stored.
    Available,
    /// The asset is listed but no price is stored.
    Unavailable,
    /// The search does not say; ask `/markets/v2/prices` to find out.
    Unknown,
}

/// Overall outcome of a search.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchOutcomeV2 {
    /// At least one asset was found.
    Usable,
    /// The provider answered and nothing matches.
    Empty,
    /// The provider could not be reached or answered with an error.
    ProviderError,
    /// The provider answered with an unreadable payload.
    Malformed,
}

/// One asset found by a search; `provider` and `symbol` are its stable identity.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FoundAssetV2 {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub symbol: String,
    pub name: Option<String>,
    /// Exchange or market, when the provider says so.
    pub venue: Option<String>,
    /// Currency prices are expressed in, when the search knows it (Yahoo searches do not).
    pub currency: Option<String>,
    #[schema(schema_with = v2_asset_class_schema)]
    pub aclass: String,
    pub price_availability: PriceAvailabilityV2,
}

/// The answer to a search.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchAssetsV2Response {
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    pub query: String,
    pub outcome: SearchOutcomeV2,
    pub results: Vec<FoundAssetV2>,
    /// Why the provider gave no answer.
    pub error: Option<FailureV2>,
}

impl From<PriceAvailability> for PriceAvailabilityV2 {
    fn from(value: PriceAvailability) -> Self {
        match value {
            PriceAvailability::Available => Self::Available,
            PriceAvailability::Unavailable => Self::Unavailable,
            PriceAvailability::Unknown => Self::Unknown,
        }
    }
}

impl From<SearchOutcome> for SearchOutcomeV2 {
    fn from(value: SearchOutcome) -> Self {
        match value {
            SearchOutcome::Usable => Self::Usable,
            SearchOutcome::Empty => Self::Empty,
            SearchOutcome::ProviderError => Self::ProviderError,
            SearchOutcome::Malformed => Self::Malformed,
        }
    }
}

impl From<AssetSearchHit> for FoundAssetV2 {
    fn from(hit: AssetSearchHit) -> Self {
        Self {
            provider: provider_name(hit.provider),
            symbol: hit.symbol,
            name: hit.name,
            venue: hit.venue,
            currency: hit.currency,
            aclass: hit.asset_class.to_string(),
            price_availability: hit.price_availability.into(),
        }
    }
}

impl From<AssetSearch> for SearchAssetsV2Response {
    fn from(search: AssetSearch) -> Self {
        Self {
            provider: provider_name(search.provider),
            query: search.query,
            outcome: search.outcome.into(),
            results: search.results.into_iter().map(Into::into).collect(),
            error: search.failure.map(|f| FailureV2::new(f, None)),
        }
    }
}
