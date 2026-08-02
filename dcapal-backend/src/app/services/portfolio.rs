use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::{
    error::Result,
    ports::{
        inbound::rest::{
            request::{PortfolioRequest, SyncPortfoliosRequest},
            response::{PortfolioResponse, SyncPortfoliosResponse},
        },
        outbound::repository::portfolio::PortfolioRepository,
    },
};

/// Coordinates bidirectional portfolio synchronization between clients and storage.
pub struct PortfolioService {
    portfolio_repository: Arc<dyn PortfolioRepository>,
}

impl PortfolioService {
    /// Creates a portfolio service using the supplied persistence port.
    pub fn new(portfolio_repository: Arc<dyn PortfolioRepository>) -> Self {
        Self {
            portfolio_repository,
        }
    }

    /// Synchronizes a user's local portfolios with the server state.
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
                    updated_portfolios.push(db_pf.try_into()?);
                }
                // portfolios not on client side
            } else if db_pf.0.deleted {
                deleted_portfolios.push(db_pf.0.id);
            } else {
                updated_portfolios.push(db_pf.try_into()?);
            }
        }

        // Process client-side portfolios
        for client_pf in req.portfolios {
            // Check if portfolio exists in db, if so, update if client data is newer
            if let Some(db_pf) = db_portfolios.iter().find(|pf| pf.0.id == client_pf.id) {
                if db_pf.0.deleted {
                    deleted_portfolios.push(db_pf.0.id);
                } else if client_pf.last_updated_at > db_pf.0.last_updated_at {
                    self.portfolio_repository
                        .upsert(user_id, client_pf.clone())
                        .await?;
                }
            } else {
                self.portfolio_repository
                    .upsert(user_id, client_pf.clone())
                    .await?;
            }
        }

        // Process deleted portfolios
        for deleted_pf in req.deleted_portfolios {
            self.portfolio_repository
                .soft_delete(user_id, deleted_pf)
                .await?;
        }

        Ok(SyncPortfoliosResponse {
            updated_portfolios,
            deleted_portfolios,
        })
    }
}
