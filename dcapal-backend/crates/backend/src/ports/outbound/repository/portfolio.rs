use async_trait::async_trait;
use uuid::Uuid;

use crate::ports::{
    inbound::rest::request::PortfolioRequest,
    outbound::repository::postgres::types::{PortfolioAssetRow, PortfolioRow},
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
}

/// The result type exposed by the portfolio persistence port.
pub type Result<T> = std::result::Result<T, PortfolioRepositoryError>;

/// Persistence operations for saved portfolios and their assets.
#[async_trait]
pub trait PortfolioRepository: Send + Sync {
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
}
