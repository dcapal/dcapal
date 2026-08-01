use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::Result,
    ports::{
        inbound::rest::request::PortfolioRequest,
        outbound::repository::postgres::types::{PortfolioAssetRow, PortfolioRow},
    },
};

#[async_trait]
pub trait PortfolioRepository: Send + Sync {
    async fn get_user_portfolios_with_assets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(PortfolioRow, Vec<PortfolioAssetRow>)>>;

    async fn soft_delete(&self, user_id: Uuid, portfolio_id: Uuid) -> Result<()>;

    async fn upsert(
        &self,
        user_id: Uuid,
        portfolio_req: PortfolioRequest,
    ) -> Result<(PortfolioRow, Vec<PortfolioAssetRow>)>;
}
