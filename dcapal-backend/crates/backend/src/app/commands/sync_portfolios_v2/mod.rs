use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    app::{
        domain::{
            asset::{AssetRef, ResolvedAsset},
            portfolio::SyncPortfoliosV2Request,
        },
        services::asset::{AssetService, AssetServiceError, ResolvedAssets},
    },
    ports::outbound::{
        adapter::AssetProviderError,
        repository::{
            asset::AssetDirectoryError,
            portfolio::{PortfolioRepository, PortfolioRepositoryError},
        },
    },
};

pub mod validation;

pub use validation::{FieldPath, OwnershipLocation, ValidationError, ValidationIssue};

/// Validated and normalized v2 synchronization command.
///
/// Besides the canonical request it carries the shared metadata of every
/// referenced asset, so the service can onboard them before any write.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncPortfoliosV2Command {
    user_id: Uuid,
    request: SyncPortfoliosV2Request,
    assets: ResolvedAssets,
}

impl SyncPortfoliosV2Command {
    /// Validates semantic rules, resolves ownership and asset references, and
    /// canonicalizes input. Performs reads only.
    pub async fn try_new(
        user_id: Uuid,
        request: SyncPortfoliosV2Request,
        portfolios: &dyn PortfolioRepository,
        assets: &AssetService,
    ) -> Result<Self, SyncPortfoliosV2CommandError> {
        let request = validation::validate_and_normalize(request)
            .map_err(SyncPortfoliosV2CommandError::Validation)?;

        let portfolio_ids = request
            .portfolios
            .iter()
            .map(|portfolio| portfolio.id)
            .chain(request.deleted_portfolios.iter().copied())
            .collect::<Vec<_>>();
        let owners = portfolios
            .find_owners(&portfolio_ids)
            .await
            .map_err(SyncPortfoliosV2CommandError::Repository)?;

        for (index, portfolio) in request.portfolios.iter().enumerate() {
            if owners
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
                .get(portfolio_id)
                .is_some_and(|owner| *owner != user_id)
            {
                return Err(SyncPortfoliosV2CommandError::Ownership(
                    OwnershipLocation::DeletedPortfolio { index },
                ));
            }
        }

        let references = request
            .portfolios
            .iter()
            .flat_map(|portfolio| portfolio.assets.iter().map(|asset| asset.asset.clone()))
            .collect::<Vec<_>>();
        let resolved = if references.is_empty() {
            ResolvedAssets {
                all: Vec::new(),
                newly_resolved: Vec::new(),
            }
        } else {
            assets
                .resolve(&references)
                .await
                .map_err(|error| match error {
                    AssetServiceError::Unresolved(unresolved) => {
                        SyncPortfoliosV2CommandError::Validation(unresolved_issues(
                            &request,
                            &unresolved,
                        ))
                    }
                    AssetServiceError::TooManyNewAssets { .. } => {
                        SyncPortfoliosV2CommandError::Validation(too_many_new_assets())
                    }
                    AssetServiceError::ProviderUnavailable(error) => {
                        SyncPortfoliosV2CommandError::ProviderUnavailable(error)
                    }
                    AssetServiceError::Directory(error) => {
                        SyncPortfoliosV2CommandError::Directory(error)
                    }
                })?
        };

        Ok(Self {
            user_id,
            request,
            assets: resolved,
        })
    }

    /// Borrows the validated request for an outbound operation.
    pub fn request(&self) -> &SyncPortfoliosV2Request {
        &self.request
    }

    /// Returns the authenticated user resolved during command construction.
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Borrows the shared metadata of every distinct referenced asset.
    pub fn assets(&self) -> &[ResolvedAsset] {
        &self.assets.all
    }

    /// Borrows the assets that were unknown to the shared directory and must
    /// be onboarded before the write.
    pub fn newly_resolved_assets(&self) -> &[ResolvedAsset] {
        &self.assets.newly_resolved
    }

    /// Consumes the command and returns its normalized request.
    pub fn into_request(self) -> SyncPortfoliosV2Request {
        self.request
    }
}

/// Builds the validation error for a sync that introduces too many new assets.
fn too_many_new_assets() -> ValidationError {
    ValidationError::single(
        FieldPath::root().field("portfolios"),
        validation::MESSAGE_TOO_MANY_NEW_ASSETS,
    )
}

/// Builds one validation issue for every submitted occurrence of an unresolved reference.
fn unresolved_issues(
    request: &SyncPortfoliosV2Request,
    unresolved: &[AssetRef],
) -> ValidationError {
    let unresolved = unresolved.iter().collect::<HashSet<_>>();
    let issues = request
        .portfolios
        .iter()
        .enumerate()
        .flat_map(|(portfolio_index, portfolio)| {
            portfolio
                .assets
                .iter()
                .enumerate()
                .filter(|(_, asset)| unresolved.contains(&asset.asset))
                .map(move |(asset_index, _)| (portfolio_index, asset_index))
        })
        .map(|(portfolio_index, asset_index)| ValidationIssue {
            path: FieldPath::root()
                .field("portfolios")
                .index(portfolio_index)
                .field("assets")
                .index(asset_index)
                .field("asset"),
            message: validation::MESSAGE_UNRESOLVED_ASSET,
        })
        .collect();
    ValidationError { issues }
}

/// Failure while constructing a validated v2 synchronization command.
#[derive(Debug, thiserror::Error)]
pub enum SyncPortfoliosV2CommandError {
    /// The submitted aggregate violates semantic rules, including references
    /// to assets that no provider knows.
    #[error("{0}")]
    Validation(#[from] ValidationError),
    /// A submitted resource belongs to another user.
    #[error("the submitted resource is owned by another user")]
    Ownership(OwnershipLocation),
    /// Ownership preflight could not be completed.
    #[error("could not resolve Portfolio ownership")]
    Repository(#[source] PortfolioRepositoryError),
    /// A provider could not decide whether a referenced asset exists.
    #[error("asset provider is unavailable")]
    ProviderUnavailable(#[source] AssetProviderError),
    /// The shared asset directory failed while resolving references.
    #[error("asset directory failed")]
    Directory(#[source] AssetDirectoryError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use rust_decimal::dec;

    use super::*;
    use crate::{
        app::domain::portfolio::{
            AllocationMode, AssetClass, PortfolioAssetV2, PortfolioV2, Provider,
        },
        ports::outbound::{
            adapter::MockAssetProvider,
            repository::{asset::MockAssetDirectory, portfolio::MockPortfolioRepository},
        },
    };

    fn id(value: u8) -> Uuid {
        Uuid::parse_str(&format!("00000000-0000-7000-8000-0000000000{value:02}")).unwrap()
    }

    fn resolved(symbol: &str) -> ResolvedAsset {
        ResolvedAsset {
            provider: Provider::YF,
            symbol: symbol.to_string(),
            name: format!("{symbol} name"),
            currency: "EUR".to_string(),
            asset_class: AssetClass::Equity,
        }
    }

    fn assets_service(directory: MockAssetDirectory, provider: MockAssetProvider) -> AssetService {
        AssetService::new(Arc::new(directory), Arc::new(provider))
    }

    /// A directory that stores nothing, so every reference goes to the provider.
    fn empty_directory() -> MockAssetDirectory {
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(Vec::new()));
        directory
    }

    fn repository_with_owners(owners: Vec<(Uuid, Uuid)>) -> MockPortfolioRepository {
        let mut repository = MockPortfolioRepository::new();
        repository.expect_find_owners().returning(move |_| {
            let owners = owners.iter().copied().collect();
            Box::pin(async move { Ok(owners) })
        });
        repository
    }

    #[tokio::test]
    async fn rejects_a_portfolio_owned_by_another_user_during_preflight() {
        // GIVEN an existing Portfolio owned by another user,
        // WHEN the command is built,
        // THEN the source Portfolio index is rejected before any asset is resolved.
        let repository = repository_with_owners(vec![(id(10), Uuid::new_v4())]);
        // No expectations: resolving would panic the mocks.
        let assets = assets_service(MockAssetDirectory::new(), MockAssetProvider::new());

        let error = SyncPortfoliosV2Command::try_new(
            Uuid::nil(),
            request(vec![asset("VWCE")]),
            &repository,
            &assets,
        )
        .await
        .expect_err("foreign Portfolio should be rejected");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::Portfolio { index: 0 })
        ));
    }

    #[tokio::test]
    async fn rejects_a_deleted_portfolio_owned_by_another_user() {
        // GIVEN a tombstone for a Portfolio owned by another user,
        // WHEN the command is built,
        // THEN the tombstone index is rejected.
        let repository = repository_with_owners(vec![(id(20), Uuid::new_v4())]);
        let assets = assets_service(MockAssetDirectory::new(), MockAssetProvider::new());
        let mut request = request(Vec::new());
        request.deleted_portfolios.push(id(20));

        let error = SyncPortfoliosV2Command::try_new(Uuid::nil(), request, &repository, &assets)
            .await
            .expect_err("foreign tombstone should be rejected");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::Ownership(OwnershipLocation::DeletedPortfolio {
                index: 0
            })
        ));
    }

    #[tokio::test]
    async fn keeps_the_resolved_assets_and_performs_no_writes() {
        // GIVEN a valid aggregate whose asset the provider knows,
        // WHEN the command is built,
        // THEN it carries the resolved metadata and nothing is stored
        // (the directory mock has no insert expectation).
        let repository = repository_with_owners(Vec::new());
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|reference| Ok(Some(resolved(&reference.symbol))));
        let assets = assets_service(empty_directory(), provider);

        let command = SyncPortfoliosV2Command::try_new(
            Uuid::nil(),
            request(vec![asset("vwce")]),
            &repository,
            &assets,
        )
        .await
        .expect("valid command");

        assert_eq!(command.assets(), [resolved("VWCE")]);
    }

    #[tokio::test]
    async fn reports_every_unresolved_reference_at_its_real_path() {
        // GIVEN two Portfolios where "NOPE" appears at assets[1] of the first and
        // assets[0] of the second while "VWCE" resolves,
        // WHEN the command is built,
        // THEN validation fails with one issue per offending occurrence.
        let repository = repository_with_owners(Vec::new());
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|reference| Ok((reference.symbol == "VWCE").then(|| resolved("VWCE"))));
        let assets = assets_service(empty_directory(), provider);
        let mut request = request(vec![asset("VWCE"), asset("NOPE")]);
        let mut second = request.portfolios[0].clone();
        second.id = id(11);
        second.assets = vec![asset("NOPE")];
        request.portfolios.push(second);

        let error = SyncPortfoliosV2Command::try_new(Uuid::nil(), request, &repository, &assets)
            .await
            .expect_err("unknown asset should be rejected");

        let SyncPortfoliosV2CommandError::Validation(error) = error else {
            panic!("expected a validation error");
        };
        let fields = error
            .issues
            .iter()
            .map(|issue| issue.path.render())
            .collect::<Vec<_>>();
        assert_eq!(
            fields,
            [
                "portfolios[0].assets[1].asset",
                "portfolios[1].assets[0].asset"
            ]
        );
    }

    #[tokio::test]
    async fn rejects_a_sync_introducing_too_many_new_assets() {
        // GIVEN an empty directory and a Portfolio referencing more unknown assets than the cap,
        // WHEN the command is built,
        // THEN validation fails at `portfolios` and no provider is contacted.
        let repository = repository_with_owners(Vec::new());
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().never();
        let assets = assets_service(empty_directory(), provider);
        let many = (0..=crate::app::services::asset::MAX_NEW_ASSETS_PER_RESOLUTION)
            .map(|i| asset(&format!("S{i}")))
            .collect();

        let error =
            SyncPortfoliosV2Command::try_new(Uuid::nil(), request(many), &repository, &assets)
                .await
                .expect_err("too many new assets should be rejected");

        let SyncPortfoliosV2CommandError::Validation(error) = error else {
            panic!("expected a validation error");
        };
        assert_eq!(error.issues.len(), 1);
        assert_eq!(error.issues[0].path.render(), "portfolios");
    }

    #[tokio::test]
    async fn surfaces_a_provider_outage_as_a_command_error() {
        // GIVEN a provider that cannot answer,
        // WHEN the command is built,
        // THEN the outage is reported instead of a validation error.
        let repository = repository_with_owners(Vec::new());
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|_| Err(AssetProviderError::Malformed("down".to_string())));
        let assets = assets_service(empty_directory(), provider);

        let error = SyncPortfoliosV2Command::try_new(
            Uuid::nil(),
            request(vec![asset("VWCE")]),
            &repository,
            &assets,
        )
        .await
        .expect_err("outage should fail");

        assert!(matches!(
            error,
            SyncPortfoliosV2CommandError::ProviderUnavailable(_)
        ));
    }

    fn asset(symbol: &str) -> PortfolioAssetV2 {
        PortfolioAssetV2 {
            asset: AssetRef {
                provider: Provider::YF,
                symbol: symbol.to_string(),
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
