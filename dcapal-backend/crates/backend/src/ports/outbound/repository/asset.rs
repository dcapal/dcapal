//! Outbound port for the shared, provider-sourced asset metadata cache.

use async_trait::async_trait;

use crate::app::domain::asset::{AssetRef, ResolvedAsset};

/// Errors raised while reading or writing shared asset metadata.
#[derive(Debug, thiserror::Error)]
pub enum AssetDirectoryError {
    #[error("asset directory database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("unsupported asset provider: {0}")]
    UnsupportedProvider(String),
    #[error("stored asset data is corrupt: {0}")]
    CorruptData(String),
}

/// Shared asset metadata cache keyed by provider and upper-case symbol.
///
/// The directory is first-writer-wins: stored metadata is never overwritten.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetDirectory: Send + Sync {
    /// Returns the stored metadata of the referenced assets. References that
    /// are not stored are omitted from the result.
    async fn find_by_refs(
        &self,
        refs: &[AssetRef],
    ) -> Result<Vec<ResolvedAsset>, AssetDirectoryError>;

    /// Stores the assets that are not stored yet and leaves existing ones untouched.
    async fn insert_missing(&self, assets: &[ResolvedAsset]) -> Result<(), AssetDirectoryError>;
}
