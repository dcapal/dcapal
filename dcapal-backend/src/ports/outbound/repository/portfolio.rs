use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::Result,
    ports::{
        inbound::rest::request::PortfolioRequest,
        outbound::repository::postgres::types::{PortfolioAssetRow, PortfolioRow},
    },
};

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
