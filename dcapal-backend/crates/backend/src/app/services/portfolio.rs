use std::{collections::HashMap, sync::Arc, time::Duration};

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

const SYNC_MAX_ATTEMPTS: usize = 3;

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

        for attempt in 0..SYNC_MAX_ATTEMPTS {
            match self.sync_once(user_id, &req).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if error.is_retryable_concurrency() && attempt + 1 < SYNC_MAX_ATTEMPTS =>
                {
                    let delay = Duration::from_millis(25 * (1 << attempt));
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = SYNC_MAX_ATTEMPTS,
                        delay_ms = delay.as_millis(),
                        "retrying Portfolio synchronization after a concurrency conflict"
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("the synchronization retry loop always returns")
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
}

impl PortfolioServiceError {
    fn is_retryable_concurrency(&self) -> bool {
        matches!(self, Self::Repository(error) if error.is_retryable_concurrency())
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
    use std::{
        borrow::Cow,
        error::Error as StdError,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;

    use super::*;
    use crate::ports::inbound::rest::request::PortfolioAssetRequest;
    use crate::ports::outbound::repository::{
        portfolio::PortfolioRepository,
        postgres::types::{PortfolioAssetRow, PortfolioRow},
    };

    #[derive(Debug)]
    struct TestDatabaseError {
        code: &'static str,
    }

    impl std::fmt::Display for TestDatabaseError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "test database error {}", self.code)
        }
    }

    impl StdError for TestDatabaseError {}

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
    }

    impl RetryRepository {
        fn new(failures_before_success: usize, error_code: &'static str) -> Self {
            Self {
                failures_before_success,
                error_code,
                upsert_attempts: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl PortfolioRepository for RetryRepository {
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

    #[tokio::test]
    async fn serialization_failure_retries_twice_then_succeeds() {
        // GIVEN a sync that encounters two serialization failures, WHEN the service retries it,
        // THEN the third total attempt succeeds.
        let repository = Arc::new(RetryRepository::new(2, "40001"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone());

        service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect("third attempt should succeed");

        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn deadlock_failure_is_retried() {
        // GIVEN a deadlock detected during synchronization, WHEN the service retries it,
        // THEN it performs another complete attempt and succeeds.
        let repository = Arc::new(RetryRepository::new(1, "40P01"));
        let attempts = &repository.upsert_attempts;
        let service = PortfolioService::new(repository.clone());

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
        let service = PortfolioService::new(repository.clone());

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
        let service = PortfolioService::new(repository.clone());

        let error = service
            .sync_portfolios(Uuid::new_v4(), valid_request())
            .await
            .expect_err("non-concurrency failure should fail immediately");

        assert_eq!(attempts.load(Ordering::Relaxed), 1);
        assert!(!error.is_retryable_concurrency());
    }
}
