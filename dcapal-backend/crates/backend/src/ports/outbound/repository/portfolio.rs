use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    app::domain::portfolio::{SyncPortfoliosV2Request, SyncPortfoliosV2Response},
    ports::{
        inbound::rest::request::PortfolioRequest,
        outbound::repository::postgres::types::{PortfolioAssetRow, PortfolioRow},
    },
};

/// Errors raised while persisting or decoding portfolio data.
#[derive(Debug, thiserror::Error)]
pub enum PortfolioRepositoryError {
    #[error("portfolio database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("unsupported Portfolio Asset provider: {0}")]
    UnsupportedProvider(String),
    #[error("portfolio cannot be updated")]
    CannotUpdate,
    #[error("v2 portfolio ownership violation for {0:?}")]
    OwnershipConflict(OwnershipConflict),
    #[error("v2 synchronization references an asset that was not onboarded: {0}")]
    AssetNotOnboarded(String),
    #[error("stored portfolio data is corrupt: {0}")]
    CorruptData(String),
}

/// Identifies an existing resource that cannot be used by the requesting user.
///
/// The repository reports stable resource identifiers rather than REST array
/// indexes. The command and REST layers add request locations when they have
/// that context available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipConflict {
    /// A Portfolio row belongs to another user or is unavailable for update.
    Portfolio { id: Uuid },
}

/// The result type exposed by the portfolio persistence port.
pub type Result<T> = std::result::Result<T, PortfolioRepositoryError>;

impl PortfolioRepositoryError {
    /// Returns whether PostgreSQL reported a retryable concurrency conflict.
    pub fn is_retryable_concurrency(&self) -> bool {
        matches!(
            self,
            Self::Database(sqlx::Error::Database(error))
                if matches!(error.code().as_deref(), Some("40001") | Some("40P01"))
        )
    }
}

/// Persistence operations for saved portfolios and their assets.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait PortfolioRepository: Send + Sync {
    /// Returns the owning user of every existing Portfolio among `portfolio_ids`.
    ///
    /// Soft-deleted Portfolios are included. Missing identifiers are omitted so
    /// commands can distinguish new aggregates and tombstones from ownership
    /// violations.
    async fn find_owners(&self, portfolio_ids: &[Uuid]) -> Result<HashMap<Uuid, Uuid>>;

    /// Returns all portfolios owned by a user together with their assets.
    async fn get_user_portfolios_with_assets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(PortfolioRow, Vec<PortfolioAssetRow>)>>;

    /// Marks an owned portfolio as deleted.
    async fn soft_delete(&self, user_id: Uuid, portfolio_id: Uuid) -> Result<()>;

    /// Creates or updates a portfolio and reconciles its asset set atomically.
    async fn upsert(
        &self,
        user_id: Uuid,
        portfolio_req: PortfolioRequest,
    ) -> Result<(PortfolioRow, Vec<PortfolioAssetRow>)>;

    /// Atomically synchronizes complete v2 Portfolio aggregates.
    async fn sync_v2(
        &self,
        user_id: Uuid,
        request: &SyncPortfoliosV2Request,
    ) -> Result<SyncPortfoliosV2Response>;
}
