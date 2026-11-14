use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    app::domain::portfolio_sync::{SyncPlan, SyncSnapshot},
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
    /// The stored Portfolios no longer match the snapshot a plan was derived
    /// from, so the plan must not be applied. The caller re-plans.
    #[error("the stored portfolios changed since the synchronization snapshot was read")]
    StaleSnapshot,
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
    /// Returns whether the failure is a retryable concurrency conflict: a
    /// PostgreSQL serialization failure or deadlock, or a stale snapshot.
    pub fn is_retryable_concurrency(&self) -> bool {
        match self {
            Self::StaleSnapshot => true,
            Self::Database(sqlx::Error::Database(error)) => {
                matches!(error.code().as_deref(), Some("40001") | Some("40P01"))
            }
            _ => false,
        }
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

    /// Reads every Portfolio of the user (tombstones included) with its
    /// content and version in one consistent read, without locking.
    ///
    /// Stored data that cannot be decoded is reported as
    /// [`PortfolioRepositoryError::CorruptData`].
    async fn load_sync_snapshot(&self, user_id: Uuid) -> Result<SyncSnapshot>;

    /// Atomically applies a v2 synchronization plan in one short transaction.
    ///
    /// The rows named by [`SyncPlan::expected`] are locked and compared with
    /// the versions the plan was derived from; any difference (or a row
    /// appearing for an identifier expected to be absent) fails the whole
    /// application with [`PortfolioRepositoryError::StaleSnapshot`] and writes
    /// nothing. A row owned by another user is an
    /// [`PortfolioRepositoryError::OwnershipConflict`]. The adapter performs
    /// the plan as given and takes no synchronization decisions.
    async fn apply_sync_plan(&self, user_id: Uuid, plan: &SyncPlan) -> Result<()>;
}
