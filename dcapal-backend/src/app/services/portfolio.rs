use crate::error::Result;
use crate::ports::inbound::rest::request::{PortfoliosRequest, SyncPortfoliosRequest};
use crate::ports::outbound::repository::portfolio::PortfolioRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct PortfolioService {
    portfolio_repository: Arc<PortfolioRepository>,
}

impl PortfolioService {
    pub fn new(portfolio_repository: Arc<PortfolioRepository>) -> Self {
        Self {
            portfolio_repository,
        }
    }

    pub async fn sync_portfolios(
        &self,
        user_id: Uuid,
        portfolio: SyncPortfoliosRequest,
    ) -> Result<Vec<PortfoliosRequest>> {
        let all_user_portfolios = self.portfolio_repository.find_all_by_user_id(user_id).await?;
        
        self.portfolio_repository.save(user_id, portfolio).await
    }
}
