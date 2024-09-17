use crate::app::domain::entity::Portfolio;
use crate::error::Result;
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

    pub async fn save_portfolio(&self, user_id: Uuid, portfolio: Portfolio) -> Result<Portfolio> {
        self.portfolio_repository.save(user_id, portfolio).await
    }

    pub async fn get_portfolios(&self, user_id: Uuid) -> Result<Vec<Portfolio>> {
        self.portfolio_repository.find_all(user_id).await
    }
}
