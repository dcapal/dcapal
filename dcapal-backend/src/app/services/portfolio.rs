use crate::error::Result;
use crate::ports::inbound::rest::request::{PortfolioRequest, SyncPortfoliosRequest};
use crate::ports::inbound::rest::response::{PortfolioResponse, SyncPortfoliosResponse};
use crate::ports::outbound::repository::portfolio::PortfolioRepository;
use std::collections::HashMap;
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
        req: SyncPortfoliosRequest,
    ) -> Result<SyncPortfoliosResponse> {
        let db_portfolios = self
            .portfolio_repository
            .get_user_portfolios_with_assets(user_id)
            .await?;

        let client_map: HashMap<Uuid, PortfolioRequest> = req
            .portfolios
            .iter()
            .map(|pf| (pf.id, pf.clone()))
            .collect();

        // Response data
        let mut updated_portfolios: Vec<PortfolioResponse> = Vec::new();
        let mut deleted_portfolios = Vec::new();

        // Process server-side portfolios
        for db_pf in db_portfolios.clone() {
            if let Some(client_pf) = client_map.get(&db_pf.0.id) {
                if db_pf.0.deleted {
                    deleted_portfolios.push(db_pf.0.id);
                } else if db_pf.0.last_updated_at > client_pf.last_updated_at {
                    updated_portfolios.push(db_pf.into());
                }
                // portfolios not on client side
            } else if db_pf.0.deleted {
                deleted_portfolios.push(db_pf.0.id);
            } else {
                updated_portfolios.push(db_pf.into());
            }
        }

        // Process client-side portfolios
        for client_pf in req.portfolios {
            if !db_portfolios
                .clone()
                .iter()
                .any(|db_pf| db_pf.0.id == client_pf.id)
            {
                self.portfolio_repository
                    .upsert(user_id, client_pf.into())
                    .await?;
            }
        }

        Ok(SyncPortfoliosResponse {
            updated_portfolios,
            deleted_portfolios,
        })
    }
}
