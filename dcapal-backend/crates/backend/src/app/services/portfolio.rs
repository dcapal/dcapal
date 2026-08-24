use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::error::DcaError;
use crate::ports::{
    inbound::rest::{
        request::{PortfolioRequest, SyncPortfoliosRequest},
        response::{PortfolioResponse, SyncPortfoliosResponse},
    },
    outbound::repository::{
        portfolio::{PortfolioRepository, PortfolioRepositoryError},
        postgres::types::Provider,
    },
};

/// Errors raised while synchronizing portfolios.
#[derive(Debug, thiserror::Error)]
pub enum PortfolioServiceError {
    #[error("unsupported Portfolio Asset provider: {provider}")]
    UnsupportedProvider { provider: String },
    #[error("portfolio cannot be updated")]
    CannotUpdate(#[source] PortfolioRepositoryError),
    #[error("portfolio repository failed")]
    Repository(#[source] PortfolioRepositoryError),
    #[error("portfolio response conversion failed")]
    ResponseConversion(#[source] crate::error::DcaError),
}

impl From<PortfolioRepositoryError> for PortfolioServiceError {
    fn from(error: PortfolioRepositoryError) -> Self {
        match error {
            error @ PortfolioRepositoryError::CannotUpdate => Self::CannotUpdate(error),
            error => Self::Repository(error),
        }
    }
}

impl From<PortfolioServiceError> for DcaError {
    fn from(error: PortfolioServiceError) -> Self {
        match error {
            PortfolioServiceError::UnsupportedProvider { provider } => Self::ValidationFailure {
                message: format!("Unsupported Portfolio Asset provider: {provider}"),
                source: Box::new(PortfolioServiceError::UnsupportedProvider { provider }),
            },
            PortfolioServiceError::CannotUpdate(error) => Self::ValidationFailure {
                message: "Portfolio cannot be updated".to_string(),
                source: Box::new(PortfolioServiceError::CannotUpdate(error)),
            },
            error => Self::ApplicationFailure {
                source: Box::new(error),
            },
        }
    }
}

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
    ) -> std::result::Result<SyncPortfoliosResponse, PortfolioServiceError> {
        validate_sync_request(&req)?;

        let db_portfolios = self
            .portfolio_repository
            .get_user_portfolios_with_assets(user_id)
            .await
            .map_err(PortfolioServiceError::from)?;

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
                    updated_portfolios.push(
                        db_pf
                            .try_into()
                            .map_err(PortfolioServiceError::ResponseConversion)?,
                    );
                }
                // portfolios not on client side
            } else if db_pf.0.deleted {
                deleted_portfolios.push(db_pf.0.id);
            } else {
                updated_portfolios.push(
                    db_pf
                        .try_into()
                        .map_err(PortfolioServiceError::ResponseConversion)?,
                );
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
                        .await
                        .map_err(PortfolioServiceError::from)?;
                }
            } else {
                self.portfolio_repository
                    .upsert(user_id, client_pf.clone())
                    .await
                    .map_err(PortfolioServiceError::from)?;
            }
        }

        // Process deleted portfolios
        for deleted_pf in req.deleted_portfolios {
            self.portfolio_repository
                .soft_delete(user_id, deleted_pf)
                .await
                .map_err(PortfolioServiceError::from)?;
        }

        Ok(SyncPortfoliosResponse {
            updated_portfolios,
            deleted_portfolios,
        })
    }
}

fn validate_sync_request(req: &SyncPortfoliosRequest) -> Result<(), PortfolioServiceError> {
    for portfolio in &req.portfolios {
        for asset in &portfolio.assets {
            if Provider::from_legacy(&asset.provider).is_none() {
                return Err(PortfolioServiceError::UnsupportedProvider {
                    provider: asset.provider.clone(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rust_decimal::Decimal;

    use super::*;
    use crate::ports::inbound::rest::request::PortfolioAssetRequest;

    #[test]
    fn unsupported_provider_is_rejected_before_repository_access() {
        // GIVEN a sync request with an unsupported provider, WHEN it is validated,
        // THEN the service returns a client-validation error with the provider name.
        let request = SyncPortfoliosRequest {
            portfolios: vec![PortfolioRequest {
                id: Uuid::new_v4(),
                name: "Portfolio".to_string(),
                quote_ccy: "EUR".to_string(),
                fees: None,
                assets: vec![PortfolioAssetRequest {
                    symbol: "VWCE".to_string(),
                    name: "Asset".to_string(),
                    aclass: "EQUITY".to_string(),
                    base_ccy: "EUR".to_string(),
                    provider: "IBKR".to_string(),
                    qty: Decimal::ONE,
                    target_weight: Decimal::ONE,
                    price: Decimal::ONE,
                    average_buy_price: Decimal::ONE,
                    fees: None,
                }],
                last_updated_at: Utc::now(),
            }],
            deleted_portfolios: Vec::new(),
        };

        assert!(matches!(
            validate_sync_request(&request),
            Err(PortfolioServiceError::UnsupportedProvider { provider })
                if provider == "IBKR"
        ));
    }
}
