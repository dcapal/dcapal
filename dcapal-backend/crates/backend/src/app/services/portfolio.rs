use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::{
    app::{
        commands::sync_portfolios_v2::{
            OwnershipLocation, SyncPortfoliosV2Command, SyncPortfoliosV2CommandError,
            ValidationError,
        },
        domain::portfolio::SyncPortfoliosV2Response,
        infra::retry::retry_sync,
        services::asset::{AssetService, AssetServiceError},
    },
    error::DcaError,
    ports::{
        inbound::rest::{
            request::{PortfolioRequest, SyncPortfoliosRequest},
            response::{PortfolioResponse, SyncPortfoliosResponse},
        },
        outbound::repository::{
            portfolio::{PortfolioRepository, PortfolioRepositoryError},
            postgres::types::{AssetClass, Provider},
        },
    },
};

/// Errors raised while synchronizing portfolios.
#[derive(Debug, thiserror::Error)]
pub enum PortfolioServiceError {
    #[error("portfolio cannot be updated")]
    CannotUpdate(#[source] PortfolioRepositoryError),
    #[error("portfolio repository failed")]
    Repository(#[source] PortfolioRepositoryError),
    #[error("portfolio response conversion failed")]
    ResponseConversion(#[source] crate::error::DcaError),
    #[error("v2 Portfolio validation failed")]
    Validation(#[source] ValidationError),
    #[error("v2 Portfolio ownership violation at {location:?}")]
    Ownership { location: OwnershipLocation },
    #[error("portfolio synchronization conflict")]
    Conflict(#[source] PortfolioRepositoryError),
    #[error("referenced assets could not be onboarded")]
    Asset(#[source] AssetServiceError),
}

impl From<PortfolioRepositoryError> for PortfolioServiceError {
    fn from(error: PortfolioRepositoryError) -> Self {
        match error {
            error @ PortfolioRepositoryError::CannotUpdate => Self::CannotUpdate(error),
            error @ PortfolioRepositoryError::OwnershipConflict(_) => Self::CannotUpdate(error),
            error => Self::Repository(error),
        }
    }
}

impl From<SyncPortfoliosV2CommandError> for PortfolioServiceError {
    fn from(error: SyncPortfoliosV2CommandError) -> Self {
        match error {
            SyncPortfoliosV2CommandError::Validation(error) => Self::Validation(error),
            SyncPortfoliosV2CommandError::Ownership(location) => Self::Ownership { location },
            SyncPortfoliosV2CommandError::Repository(error) => Self::Repository(error),
            SyncPortfoliosV2CommandError::ProviderUnavailable(error) => {
                Self::Asset(AssetServiceError::ProviderUnavailable(error))
            }
            SyncPortfoliosV2CommandError::Directory(error) => {
                Self::Asset(AssetServiceError::Directory(error))
            }
        }
    }
}

impl From<PortfolioServiceError> for DcaError {
    fn from(error: PortfolioServiceError) -> Self {
        match error {
            PortfolioServiceError::CannotUpdate(error) => Self::ValidationFailure {
                message: "Portfolio cannot be updated".to_string(),
                source: Box::new(PortfolioServiceError::CannotUpdate(error)),
            },
            PortfolioServiceError::Validation(error) => Self::ValidationFailure {
                message: error.to_string(),
                source: Box::new(error),
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
    assets: Arc<AssetService>,
}

impl PortfolioService {
    /// Creates a portfolio service using the supplied persistence port and the
    /// asset service that onboards assets referenced by v2 synchronization.
    pub fn new(
        portfolio_repository: Arc<dyn PortfolioRepository>,
        assets: Arc<AssetService>,
    ) -> Self {
        Self {
            portfolio_repository,
            assets,
        }
    }

    /// Synchronizes a user's local portfolios with the server state.
    pub async fn sync_portfolios(
        &self,
        user_id: Uuid,
        req: SyncPortfoliosRequest,
    ) -> std::result::Result<SyncPortfoliosResponse, PortfolioServiceError> {
        let req = normalize_sync_request(req);

        retry_sync(
            || self.sync_once(user_id, &req),
            PortfolioServiceError::is_retryable_concurrency,
        )
        .await
    }

    async fn sync_once(
        &self,
        user_id: Uuid,
        req: &SyncPortfoliosRequest,
    ) -> std::result::Result<SyncPortfoliosResponse, PortfolioServiceError> {
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
        for client_pf in &req.portfolios {
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
        for deleted_pf in &req.deleted_portfolios {
            self.portfolio_repository
                .soft_delete(user_id, *deleted_pf)
                .await
                .map_err(PortfolioServiceError::from)?;
        }

        Ok(SyncPortfoliosResponse {
            updated_portfolios,
            deleted_portfolios,
        })
    }

    /// Atomically synchronizes a validated complete v2 Portfolio aggregate.
    pub async fn sync_portfolios_v2(
        &self,
        command: SyncPortfoliosV2Command,
    ) -> std::result::Result<SyncPortfoliosV2Response, PortfolioServiceError> {
        let user_id = command.user_id();
        // Shared asset rows are stored before the transaction opens so that no
        // provider or directory round trip happens while rows are locked.
        // Only assets the directory lacked need storing; known ones are skipped.
        if !command.newly_resolved_assets().is_empty() {
            self.assets
                .onboard(command.newly_resolved_assets())
                .await
                .map_err(PortfolioServiceError::Asset)?;
        }
        let request = command.into_request();
        let response = retry_sync(
            || self.portfolio_repository.sync_v2(user_id, request.clone()),
            PortfolioRepositoryError::is_retryable_concurrency,
        )
        .await;

        match response {
            Ok(response) => Ok(response),
            Err(error) if error.is_retryable_concurrency() => {
                Err(PortfolioServiceError::Conflict(error))
            }
            Err(error) => Err(PortfolioServiceError::from(error)),
        }
    }
}

impl PortfolioServiceError {
    fn is_retryable_concurrency(&self) -> bool {
        matches!(self, Self::Repository(error) if error.is_retryable_concurrency())
    }
}

fn normalize_sync_request(mut req: SyncPortfoliosRequest) -> SyncPortfoliosRequest {
    for portfolio in &mut req.portfolios {
        portfolio.assets.retain_mut(|asset| {
            let Some(provider) = Provider::from_legacy(&asset.provider) else {
                return false;
            };

            asset.provider = provider.as_legacy_name().to_string();
            asset.aclass = AssetClass::from_legacy(&asset.aclass)
                .as_legacy_name()
                .to_string();
            true
        });
    }

    req
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Cow,
        error::Error as StdError,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;

    use super::*;
    use crate::app::commands::sync_portfolios_v2::SyncPortfoliosV2Command;
    use crate::app::domain::{
        asset::{AssetRef, ResolvedAsset},
        portfolio::{
            AllocationMode, AssetClass, PortfolioAssetV2, PortfolioV2, Provider,
            SyncPortfoliosV2Request, SyncPortfoliosV2Response,
        },
    };
    use crate::ports::inbound::rest::request::PortfolioAssetRequest;
    use crate::ports::outbound::repository::{
        portfolio::PortfolioRepository,
        postgres::types::{PortfolioAssetRow, PortfolioRow},
    };
    use crate::ports::outbound::{
        adapter::MockAssetProvider, repository::asset::MockAssetDirectory,
    };

    #[test]
    fn sync_normalization_maps_aliases_and_drops_unsupported_assets() {
        // GIVEN a sync request with mixed-case aliases and an unsupported provider,
        // WHEN the request is normalized, THEN supported assets use canonical v1 names
        // and unsupported assets are removed.
        let mut request = valid_request();
        request.portfolios[0].assets.push(PortfolioAssetRequest {
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            aclass: "cash".to_string(),
            base_ccy: "EUR".to_string(),
            provider: "kRaKeN".to_string(),
            qty: Decimal::ONE,
            target_weight: Decimal::ONE,
            price: Decimal::ONE,
            average_buy_price: Decimal::ONE,
            fees: None,
        });
        request.portfolios[0].assets.push(PortfolioAssetRequest {
            symbol: "SPY".to_string(),
            name: "S&P 500".to_string(),
            aclass: "EQUITY".to_string(),
            base_ccy: "USD".to_string(),
            provider: "IBKR".to_string(),
            qty: Decimal::ONE,
            target_weight: Decimal::ONE,
            price: Decimal::ONE,
            average_buy_price: Decimal::ONE,
            fees: None,
        });

        let normalized = normalize_sync_request(request);
        let assets = &normalized.portfolios[0].assets;

        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].provider, "YF");
        assert_eq!(assets[0].aclass, "EQUITY");
        assert_eq!(assets[1].provider, "DCAPal");
        assert_eq!(assets[1].aclass, "CURRENCY");
    }

    #[test]
    fn sync_normalization_retains_portfolios_when_all_assets_are_unsupported() {
        // GIVEN a portfolio containing only unsupported providers, WHEN the sync request
        // is normalized, THEN the portfolio remains with an empty asset set.
        let mut request = valid_request();
        request.portfolios[0].assets[0].provider = "IBKR".to_string();

        let normalized = normalize_sync_request(request);

        assert_eq!(normalized.portfolios.len(), 1);
        assert!(normalized.portfolios[0].assets.is_empty());
    }

    /// Asset service for tests that never touch assets; any call panics the mocks.
    fn unused_assets() -> Arc<AssetService> {
        Arc::new(AssetService::new(
            Arc::new(MockAssetDirectory::new()),
            Arc::new(MockAssetProvider::new()),
        ))
    }

    fn resolved_vwce() -> ResolvedAsset {
        ResolvedAsset {
            provider: Provider::YF,
            symbol: "VWCE".to_string(),
            name: "Asset".to_string(),
            currency: "EUR".to_string(),
            asset_class: AssetClass::Equity,
        }
    }

    /// Provider that knows VWCE, used when the directory lacks it.
    fn vwce_provider() -> MockAssetProvider {
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|_| Ok(Some(resolved_vwce())));
        provider
    }

    /// Asset service whose directory lacks VWCE, whose provider knows it, and
    /// which records every onboarding call as `"onboard"` in `log`.
    fn logging_assets(log: &Arc<Mutex<Vec<&'static str>>>) -> Arc<AssetService> {
        let mut directory = MockAssetDirectory::new();
        directory.expect_find_by_refs().returning(|_| Ok(vec![]));
        let onboard_log = log.clone();
        directory
            .expect_insert_missing()
            .times(1)
            .returning(move |assets| {
                assert_eq!(assets, [resolved_vwce()]);
                onboard_log.lock().unwrap().push("onboard");
                Ok(())
            });
        Arc::new(AssetService::new(
            Arc::new(directory),
            Arc::new(vwce_provider()),
        ))
    }

    #[tokio::test]
    async fn v2_known_assets_are_not_onboarded() {
        // GIVEN a v2 sync whose only asset is already in the shared directory,
        // WHEN the service synchronizes it,
        // THEN the directory is never asked to insert anything (no insert expectation).
        let repository = Arc::new(RetryRepository::new(0, "23505"));
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(vec![resolved_vwce()]));
        directory.expect_insert_missing().never();
        let assets = Arc::new(AssetService::new(
            Arc::new(directory),
            Arc::new(MockAssetProvider::new()),
        ));
        let service = PortfolioService::new(repository.clone(), assets.clone());

        service
            .sync_portfolios_v2(valid_v2_command(repository.as_ref(), &assets).await)
            .await
            .expect("known-only sync should succeed");
    }

    #[tokio::test]
    async fn v2_onboards_assets_once_before_the_retried_transaction() {
        // GIVEN a v2 sync whose repository fails twice with serialization
        // conflicts before succeeding,
        // WHEN the service synchronizes it,
        // THEN assets are onboarded exactly once, before the first attempt, and the
        // transaction is retried without onboarding again.
        let log = Arc::new(Mutex::new(Vec::new()));
        let repository = Arc::new(RetryRepository::new(2, "40001").with_log(log.clone()));
        let attempts = &repository.upsert_attempts;
        let assets = logging_assets(&log);
        let service = PortfolioService::new(repository.clone(), assets.clone());

        service
            .sync_portfolios_v2(valid_v2_command(repository.as_ref(), &assets).await)
            .await
            .expect("third v2 attempt should succeed");

        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert_eq!(
            *log.lock().unwrap(),
            ["onboard", "sync_v2", "sync_v2", "sync_v2"]
        );
    }

    #[tokio::test]
    async fn v2_retry_exhaustion_returns_conflict() {
        // GIVEN a v2 sync that keeps failing with serialization failures, WHEN
        // all attempts are exhausted, THEN the service returns Conflict.
        let log = Arc::new(Mutex::new(Vec::new()));
        let repository = Arc::new(RetryRepository::new(3, "40001"));
        let attempts = &repository.upsert_attempts;
        let assets = logging_assets(&log);
        let service = PortfolioService::new(repository.clone(), assets.clone());

        let error = service
            .sync_portfolios_v2(valid_v2_command(repository.as_ref(), &assets).await)
            .await
            .expect_err("retry exhaustion should return a conflict");

        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert!(matches!(error, PortfolioServiceError::Conflict(_)));
    }

    #[tokio::test]
    async fn v2_permanent_failure_is_not_retried() {
        // GIVEN a v2 sync that fails with a non-concurrency database error,
        // WHEN the service executes it, THEN it returns the repository error
        // after one total attempt.
        let log = Arc::new(Mutex::new(Vec::new()));
        let repository = Arc::new(RetryRepository::new(3, "23505"));
        let attempts = &repository.upsert_attempts;
        let assets = logging_assets(&log);
        let service = PortfolioService::new(repository.clone(), assets.clone());

        let error = service
            .sync_portfolios_v2(valid_v2_command(repository.as_ref(), &assets).await)
            .await
            .expect_err("permanent v2 failure should fail immediately");

        assert_eq!(attempts.load(Ordering::Relaxed), 1);
        assert!(matches!(error, PortfolioServiceError::Repository(_)));
    }

    #[tokio::test]
    async fn v2_onboarding_failure_skips_the_transaction() {
        // GIVEN a directory that fails while onboarding assets,
        // WHEN the service synchronizes a v2 aggregate,
        // THEN it fails with an asset error and never opens the repository transaction.
        let repository = Arc::new(RetryRepository::new(0, "23505"));
        let attempts = &repository.upsert_attempts;
        let mut directory = MockAssetDirectory::new();
        directory.expect_find_by_refs().returning(|_| Ok(vec![]));
        directory.expect_insert_missing().returning(|_| {
            Err(
                crate::ports::outbound::repository::asset::AssetDirectoryError::CorruptData(
                    "boom".to_string(),
                ),
            )
        });
        let assets = Arc::new(AssetService::new(
            Arc::new(directory),
            Arc::new(vwce_provider()),
        ));
        let service = PortfolioService::new(repository.clone(), assets.clone());

        let error = service
            .sync_portfolios_v2(valid_v2_command(repository.as_ref(), &assets).await)
            .await
            .expect_err("onboarding failure should abort");

        assert_eq!(attempts.load(Ordering::Relaxed), 0);
        assert!(matches!(error, PortfolioServiceError::Asset(_)));
    }

    #[tokio::test]
    async fn serialization_failure_retries_twice_then_succeeds() {
        // GIVEN a sync that encounters two serialization failures, WHEN the service retries it,
        // THEN the third total attempt succeeds.
        let repository = Arc::new(RetryRepository::new(2, "40001"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone(), unused_assets());

        service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect("third attempt should succeed");

        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn sync_sends_only_supported_normalized_assets_to_repository() {
        // GIVEN a sync request with valid and unsupported assets, WHEN the service synchronizes it,
        // THEN the repository receives the portfolio with only canonical supported assets.
        let repository = Arc::new(RetryRepository::new(0, "23505"));
        let mut request = valid_request();
        request.portfolios[0].assets.push(PortfolioAssetRequest {
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            aclass: "CURRENCY".to_string(),
            base_ccy: "EUR".to_string(),
            provider: "IBKR".to_string(),
            qty: Decimal::ONE,
            target_weight: Decimal::ZERO,
            price: Decimal::ONE,
            average_buy_price: Decimal::ONE,
            fees: None,
        });
        let service = PortfolioService::new(repository.clone(), unused_assets());

        service
            .sync_portfolios(Uuid::new_v4(), request)
            .await
            .expect("supported assets should synchronize");

        let synced = repository
            .last_upserted_portfolio
            .lock()
            .unwrap()
            .clone()
            .expect("repository should receive a portfolio");
        assert_eq!(synced.assets.len(), 1);
        assert_eq!(synced.assets[0].provider, "YF");
        assert_eq!(synced.assets[0].aclass, "EQUITY");
    }

    #[tokio::test]
    async fn sync_sends_an_empty_portfolio_when_all_assets_are_unsupported() {
        // GIVEN a portfolio whose assets all use unsupported providers, WHEN it synchronizes,
        // THEN the repository receives the parent portfolio with an empty asset list.
        let repository = Arc::new(RetryRepository::new(0, "23505"));
        let mut request = valid_request();
        request.portfolios[0].assets[0].provider = "IBKR".to_string();
        let service = PortfolioService::new(repository.clone(), unused_assets());

        service
            .sync_portfolios(Uuid::new_v4(), request)
            .await
            .expect("empty portfolio should synchronize");

        let synced = repository
            .last_upserted_portfolio
            .lock()
            .unwrap()
            .clone()
            .expect("repository should receive a portfolio");
        assert!(synced.assets.is_empty());
    }

    #[tokio::test]
    async fn deadlock_failure_is_retried() {
        // GIVEN a deadlock detected during synchronization, WHEN the service retries it,
        // THEN it performs another complete attempt and succeeds.
        let repository = Arc::new(RetryRepository::new(1, "40P01"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone(), unused_assets());

        service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect("second attempt should succeed");

        assert_eq!(attempts.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn retryable_failure_is_returned_after_three_attempts() {
        // GIVEN a sync that keeps failing with serialization failures, WHEN all retry attempts are used,
        // THEN the service returns the final repository error after three total attempts.
        let repository = Arc::new(RetryRepository::new(3, "40001"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone(), unused_assets());

        let error = service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect_err("retry exhaustion should fail the sync");

        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert!(matches!(
            error,
            PortfolioServiceError::Repository(
                PortfolioRepositoryError::Database(sqlx::Error::Database(database_error))
            ) if database_error.code().as_deref() == Some("40001")
        ));
    }

    #[tokio::test]
    async fn non_concurrency_failure_is_not_retried() {
        // GIVEN a non-concurrency database failure, WHEN synchronization starts,
        // THEN it fails immediately without another attempt.
        let repository = Arc::new(RetryRepository::new(3, "23505"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone(), unused_assets());

        let error = service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect_err("non-concurrency failure should fail immediately");

        assert_eq!(attempts.load(Ordering::Relaxed), 1);
        assert!(!error.is_retryable_concurrency());
    }

    async fn valid_v2_command(
        repository: &dyn PortfolioRepository,
        assets: &AssetService,
    ) -> SyncPortfoliosV2Command {
        SyncPortfoliosV2Command::try_new(
            Uuid::nil(),
            SyncPortfoliosV2Request {
                schema_version: 2,
                portfolios: vec![PortfolioV2 {
                    id: Uuid::new_v4(),
                    name: "Portfolio".to_string(),
                    quote_currency: "EUR".to_string(),
                    allocation_mode: AllocationMode::Simple,
                    drift_band: None,
                    fees: None,
                    assets: vec![PortfolioAssetV2 {
                        asset: AssetRef::new(Provider::YF, "VWCE"),
                        asset_class_override: None,
                        quantity: Decimal::ONE,
                        target_weight: Some(Decimal::from(100)),
                        manual_price: None,
                        average_buy_price: None,
                        fees: None,
                    }],
                    strategic_allocation: None,
                    last_updated_at: Utc::now(),
                }],
                deleted_portfolios: Vec::new(),
            },
            repository,
            assets,
        )
        .await
        .expect("v2 fixture should be valid")
    }

    #[derive(Debug, thiserror::Error)]
    #[error("test database error {code}")]
    struct TestDatabaseError {
        code: &'static str,
    }

    impl sqlx::error::DatabaseError for TestDatabaseError {
        fn message(&self) -> &str {
            "test database error"
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            Some(Cow::Borrowed(self.code))
        }

        fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    struct RetryRepository {
        failures_before_success: usize,
        error_code: &'static str,
        upsert_attempts: AtomicUsize,
        last_upserted_portfolio: Mutex<Option<PortfolioRequest>>,
        /// Optional shared event log; each v2 attempt appends `"sync_v2"`.
        log: Option<Arc<Mutex<Vec<&'static str>>>>,
    }

    impl RetryRepository {
        fn new(failures_before_success: usize, error_code: &'static str) -> Self {
            Self {
                failures_before_success,
                error_code,
                upsert_attempts: AtomicUsize::new(0),
                last_upserted_portfolio: Mutex::new(None),
                log: None,
            }
        }

        fn with_log(mut self, log: Arc<Mutex<Vec<&'static str>>>) -> Self {
            self.log = Some(log);
            self
        }
    }

    #[async_trait]
    impl PortfolioRepository for RetryRepository {
        async fn find_owners(
            &self,
            _portfolio_ids: &[Uuid],
        ) -> crate::ports::outbound::repository::portfolio::Result<
            std::collections::HashMap<Uuid, Uuid>,
        > {
            Ok(std::collections::HashMap::new())
        }

        async fn get_user_portfolios_with_assets(
            &self,
            _user_id: Uuid,
        ) -> crate::ports::outbound::repository::portfolio::Result<
            Vec<(PortfolioRow, Vec<PortfolioAssetRow>)>,
        > {
            Ok(Vec::new())
        }

        async fn soft_delete(
            &self,
            _user_id: Uuid,
            _portfolio_id: Uuid,
        ) -> crate::ports::outbound::repository::portfolio::Result<()> {
            Ok(())
        }

        async fn upsert(
            &self,
            user_id: Uuid,
            portfolio: PortfolioRequest,
        ) -> crate::ports::outbound::repository::portfolio::Result<(
            PortfolioRow,
            Vec<PortfolioAssetRow>,
        )> {
            *self.last_upserted_portfolio.lock().unwrap() = Some(portfolio.clone());
            let attempt = self.upsert_attempts.fetch_add(1, Ordering::Relaxed) + 1;
            if attempt <= self.failures_before_success {
                return Err(PortfolioRepositoryError::Database(sqlx::Error::Database(
                    Box::new(TestDatabaseError {
                        code: self.error_code,
                    }),
                )));
            }

            let now = Utc::now();
            Ok((
                PortfolioRow {
                    id: portfolio.id,
                    user_id,
                    name: portfolio.name,
                    currency: portfolio.quote_ccy,
                    deleted: false,
                    last_updated_at: portfolio.last_updated_at,
                    allocation_mode:
                        crate::ports::outbound::repository::postgres::types::AllocationMode::Simple,
                    drift_band: None,
                    strategic_allocation_present: false,
                    max_fee_impact: None,
                    fee_type: None,
                    fee_amount: None,
                    fee_rate: None,
                    min_fee: None,
                    max_fee: None,
                    created_at: now,
                    updated_at: now,
                },
                Vec::new(),
            ))
        }

        async fn sync_v2(
            &self,
            _user_id: Uuid,
            request: SyncPortfoliosV2Request,
        ) -> crate::ports::outbound::repository::portfolio::Result<SyncPortfoliosV2Response>
        {
            if let Some(log) = &self.log {
                log.lock().unwrap().push("sync_v2");
            }
            let attempt = self.upsert_attempts.fetch_add(1, Ordering::Relaxed) + 1;
            if attempt <= self.failures_before_success {
                return Err(PortfolioRepositoryError::Database(sqlx::Error::Database(
                    Box::new(TestDatabaseError {
                        code: self.error_code,
                    }),
                )));
            }

            Ok(SyncPortfoliosV2Response {
                updated_portfolios: request.portfolios,
                deleted_portfolios: request.deleted_portfolios,
            })
        }
    }

    fn valid_request() -> SyncPortfoliosRequest {
        SyncPortfoliosRequest {
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
                    provider: "YF".to_string(),
                    qty: Decimal::ONE,
                    target_weight: Decimal::ONE,
                    price: Decimal::ONE,
                    average_buy_price: Decimal::ONE,
                    fees: None,
                }],
                last_updated_at: Utc::now(),
            }],
            deleted_portfolios: Vec::new(),
        }
    }
}
