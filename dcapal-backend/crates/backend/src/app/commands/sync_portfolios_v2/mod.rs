use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    app::domain::portfolio::{Provider, SyncPortfoliosV2Request},
    ports::outbound::repository::portfolio::{PortfolioRepository, PortfolioRepositoryError},
};

pub mod validation;

pub use validation::{FieldPath, OwnershipLocation, ValidationError, ValidationIssue};

/// Validated and normalized v2 synchronization command.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncPortfoliosV2Command {
    user_id: Uuid,
    request: SyncPortfoliosV2Request,
}

impl SyncPortfoliosV2Command {
    /// Validates semantic rules, resolves ownership, and canonicalizes input.
    pub async fn try_new(
        user_id: Uuid,
        request: SyncPortfoliosV2Request,
        repository: &dyn PortfolioRepository,
    ) -> Result<Self, SyncPortfoliosV2CommandError> {
        let submitted_asset_locations = request
            .portfolios
            .iter()
            .enumerate()
            .flat_map(|(portfolio_index, portfolio)| {
                portfolio
                    .assets
                    .iter()
                    .enumerate()
                    .filter(|(_, asset)| !matches!(asset.asset.provider, Provider::Unsupported(_)))
                    .map(move |(asset_index, asset)| (asset.id, (portfolio_index, asset_index)))
            })
            .collect::<HashMap<_, _>>();
        let request = validation::validate_and_normalize(request)
            .map_err(SyncPortfoliosV2CommandError::Validation)?;

        let portfolio_ids = request
            .portfolios
            .iter()
            .map(|portfolio| portfolio.id)
            .chain(request.deleted_portfolios.iter().copied())
            .collect::<Vec<_>>();
        let asset_ids = request
            .portfolios
            .iter()
            .flat_map(|portfolio| portfolio.assets.iter().map(|asset| asset.id))
            .collect::<Vec<_>>();
        let owners = repository
            .find_owners(&portfolio_ids, &asset_ids)
            .await
            .map_err(SyncPortfoliosV2CommandError::Repository)?;

        for (index, portfolio) in request.portfolios.iter().enumerate() {
            if owners
                .portfolios
                .get(&portfolio.id)
                .is_some_and(|owner| *owner != user_id)
            {
                return Err(SyncPortfoliosV2CommandError::Ownership(
                    OwnershipLocation::Portfolio { index },
                ));
            }
        }
        for (index, portfolio_id) in request.deleted_portfolios.iter().enumerate() {
            if owners
                .portfolios
                .get(portfolio_id)
                .is_some_and(|owner| *owner != user_id)
            {
                return Err(SyncPortfoliosV2CommandError::Ownership(
                    OwnershipLocation::DeletedPortfolio { index },
                ));
            }
        }
        for (portfolio_index, portfolio) in request.portfolios.iter().enumerate() {
            for (asset_index, asset) in portfolio.assets.iter().enumerate() {
                if owners.portfolio_assets.get(&asset.id).is_some_and(|owner| {
                    owner.user_id != user_id || owner.portfolio_id != portfolio.id
                }) {
                    let (source_portfolio_index, source_asset_index) = submitted_asset_locations
                        .get(&asset.id)
                        .copied()
                        .unwrap_or((portfolio_index, asset_index));
                    return Err(SyncPortfoliosV2CommandError::Ownership(
                        OwnershipLocation::PortfolioAsset {
                            portfolio_index: source_portfolio_index,
                            asset_index: source_asset_index,
                        },
                    ));
                }
            }
        }

        Ok(Self { user_id, request })
    }

    /// Borrows the validated request for an outbound operation.
    pub fn request(&self) -> &SyncPortfoliosV2Request {
        &self.request
    }

    /// Returns the authenticated user resolved during command construction.
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Consumes the command and returns its normalized request.
    pub fn into_request(self) -> SyncPortfoliosV2Request {
        self.request
    }
}

/// Failure while constructing a validated v2 synchronization command.
#[derive(Debug, thiserror::Error)]
pub enum SyncPortfoliosV2CommandError {
    /// The submitted aggregate violates semantic rules.
    #[error("{0}")]
    Validation(#[from] ValidationError),
    /// A submitted resource belongs to another user.
    #[error("the submitted resource is owned by another user")]
    Ownership(OwnershipLocation),
    /// Ownership preflight could not be completed.
    #[error("could not resolve Portfolio ownership")]
    Repository(#[source] PortfolioRepositoryError),
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rust_decimal::dec;

    use super::*;
    use crate::app::domain::portfolio::{
        AllocationMode, AssetClass, PortfolioAssetV2, PortfolioV2, Provider, SharedAssetV2,
    };
    use crate::ports::outbound::repository::portfolio::{
        MockPortfolioRepository, PortfolioAssetOwnership, PortfolioOwnership,
    };

    fn id(value: u8) -> Uuid {
        Uuid::parse_str(&format!("00000000-0000-7000-8000-0000000000{value:02}")).unwrap()
    }

    #[tokio::test]
    async fn rejects_a_portfolio_owned_by_another_user_during_preflight() {
        // GIVEN an existing Portfolio owned by another user, WHEN ownership resolves,
        // THEN the source Portfolio index is rejected before writes.
        let mut repository = MockPortfolioRepository::new();
        repository.expect_find_owners().returning(|_, _| {
            let mut ownership = PortfolioOwnership::default();
            ownership.portfolios.insert(id(10), Uuid::new_v4());
            Box::pin(async move { Ok(ownership) })
        });

        let error =
            SyncPortfoliosV2Command::try_new(Uuid::nil(), request(vec![asset(id(1))]), &repository)
                .await
                .expect_err("foreign Portfolio should be rejected");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::Portfolio { index: 0 })
        ));
    }

    #[tokio::test]
    async fn rejects_a_portfolio_asset_owned_by_another_user_during_preflight() {
        // GIVEN an existing Portfolio Asset owned by another user, WHEN ownership resolves,
        // THEN the nested source location is rejected before writes.
        let mut repository = MockPortfolioRepository::new();
        repository.expect_find_owners().returning(|_, _| {
            let mut ownership = PortfolioOwnership::default();
            ownership.portfolio_assets.insert(
                id(1),
                PortfolioAssetOwnership {
                    user_id: Uuid::new_v4(),
                    portfolio_id: id(11),
                },
            );
            Box::pin(async move { Ok(ownership) })
        });

        let error =
            SyncPortfoliosV2Command::try_new(Uuid::nil(), request(vec![asset(id(1))]), &repository)
                .await
                .expect_err("foreign Portfolio Asset should be rejected");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::PortfolioAsset {
                portfolio_index: 0,
                asset_index: 0,
            })
        ));
    }

    #[tokio::test]
    async fn rejects_a_portfolio_asset_owned_by_another_portfolio_during_preflight() {
        // GIVEN a relationship owned by the authenticated user but attached to
        // another Portfolio, WHEN ownership resolves, THEN the submitted
        // relationship cannot be reassigned through this aggregate.
        let mut repository = MockPortfolioRepository::new();
        repository.expect_find_owners().returning(|_, _| {
            let mut ownership = PortfolioOwnership::default();
            ownership.portfolio_assets.insert(
                id(1),
                PortfolioAssetOwnership {
                    user_id: Uuid::nil(),
                    portfolio_id: id(11),
                },
            );
            Box::pin(async move { Ok(ownership) })
        });

        let error =
            SyncPortfoliosV2Command::try_new(Uuid::nil(), request(vec![asset(id(1))]), &repository)
                .await
                .expect_err("relationship from another Portfolio should be rejected");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::PortfolioAsset {
                portfolio_index: 0,
                asset_index: 0,
            })
        ));
    }

    fn asset(asset_id: Uuid) -> PortfolioAssetV2 {
        PortfolioAssetV2 {
            id: asset_id,
            asset: SharedAssetV2 {
                provider: Provider::YF,
                symbol: "VWCE".to_string(),
                name: "ETF".to_string(),
                currency: "EUR".to_string(),
                asset_class: AssetClass::Equity,
            },
            asset_class_override: None,
            quantity: dec!(1),
            target_weight: Some(dec!(100)),
            manual_price: None,
            average_buy_price: None,
            fees: None,
        }
    }

    fn request(assets: Vec<PortfolioAssetV2>) -> SyncPortfoliosV2Request {
        SyncPortfoliosV2Request {
            schema_version: 2,
            portfolios: vec![PortfolioV2 {
                id: id(10),
                name: "Draft".to_string(),
                quote_currency: "EUR".to_string(),
                allocation_mode: AllocationMode::Simple,
                drift_band: None,
                fees: None,
                assets,
                strategic_allocation: None,
                last_updated_at: Utc::now(),
            }],
            deleted_portfolios: Vec::new(),
        }
    }
}
