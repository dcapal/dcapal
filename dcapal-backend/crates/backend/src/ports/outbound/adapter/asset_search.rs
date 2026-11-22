//! Port for providers that search their listed assets by free text.

use async_trait::async_trait;

use crate::app::domain::{market_quote::PriceFailure, portfolio::AssetClass};

/// One asset found by a provider search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedAsset {
    /// Provider symbol, as the provider spells it.
    pub symbol: String,
    /// Display name, when the provider has one.
    pub name: Option<String>,
    /// Exchange or market the asset trades on, when the provider says so.
    pub venue: Option<String>,
    /// Asset class derived from the provider's instrument type.
    pub asset_class: AssetClass,
}

/// Why a provider search produced no answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssetSearchError {
    /// The shared rate limiter had no capacity, so the provider was not asked.
    #[error("the provider rate limit is exhausted")]
    RateLimited,
    /// The provider was asked and failed or answered something unreadable.
    #[error(transparent)]
    Failed(PriceFailure),
}

/// A provider that searches the assets it lists.
///
/// Implementations ask only their own provider and report every failure.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetSearchProvider: Send + Sync {
    /// Searches the provider's listings for `query`.
    async fn search_assets(&self, query: &str) -> Result<Vec<ListedAsset>, AssetSearchError>;
}
