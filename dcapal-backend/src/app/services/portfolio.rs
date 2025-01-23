use crate::error::Result;
use crate::ports::inbound::rest::request::SyncPortfoliosRequest;
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
    ) -> Result<Vec<UserPortfoliosResponse>> {

        let all_user_portfolios = self
            .portfolio_repository
            .get_user_portfolios_with_assets(user_id)
            .await?;

        let portfolios_to_sync = portfolio
            .portfolios
            .iter()
            .filter(|p| {
                p.last_updated_at.gt(&all_user_portfolios
                    .iter()
                    .filter(|(p, _)| p.id == p.id)
                    .map(|(p, _)| p.last_updated_at))
            })
            .collect();

        self.portfolio_repository
            .upsert(user_id, portfolios_to_sync)
            .await?;

        self.portfolio_repository
            .soft_delete(user_id, portfolio.deleted_portfolios)
            .await?;
    }
}
