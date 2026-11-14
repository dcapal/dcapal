//! Outbound port for the shared, provider-sourced asset metadata cache.

use async_trait::async_trait;

use crate::app::domain::asset::{AssetRef, ResolvedAsset, StoredAsset};

/// Errors raised while reading or writing shared asset metadata.
#[derive(Debug, thiserror::Error)]
pub enum AssetDirectoryError {
    #[error("asset directory database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("stored asset data is corrupt: {0}")]
    CorruptData(String),
}

/// Shared asset metadata cache keyed by provider and upper-case symbol.
///
/// Each row records whether its metadata was supplied by a client or confirmed
/// by the provider. Provider-confirmed metadata is never overwritten; metadata
/// that is only client-supplied may be replaced by what the provider says.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetDirectory: Send + Sync {
    /// Returns the stored metadata of the referenced assets with its
    /// provenance. References that are not stored are omitted from the result.
    async fn find_by_refs(
        &self,
        refs: &[AssetRef],
    ) -> Result<Vec<StoredAsset>, AssetDirectoryError>;

    /// Stores provider-resolved metadata, marking it provider-confirmed.
    ///
    /// Missing assets are inserted and client-supplied ones are overwritten;
    /// assets that are already provider-confirmed are left untouched.
    async fn upsert_resolved(&self, assets: &[ResolvedAsset]) -> Result<(), AssetDirectoryError>;

    /// Marks the referenced client-supplied assets as provider-confirmed
    /// without changing their metadata.
    async fn confirm(&self, refs: &[AssetRef]) -> Result<(), AssetDirectoryError>;
}
