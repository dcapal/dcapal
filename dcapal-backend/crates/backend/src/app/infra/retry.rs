//! Shared retry policies for application operations.

use std::{future::Future, time::Duration};

use backoff::{ExponentialBackoffBuilder, future::retry};

/// Maximum number of times a synchronisation operation may be attempted.
pub const SYNC_MAX_ATTEMPTS: usize = 3;

const SYNC_INITIAL_INTERVAL: Duration = Duration::from_millis(25);
const SYNC_MAX_INTERVAL: Duration = Duration::from_millis(50);

/// Retries transient operation failures with the synchronisation backoff policy.
///
/// The operation receives three total attempts. The first retry waits 25 ms and
/// the second waits 50 ms. The retry predicate decides which failures are
/// transient; all other failures return immediately.
pub async fn retry_sync<T, E, Operation, OperationFuture, ShouldRetry>(
    mut operation: Operation,
    should_retry: ShouldRetry,
) -> Result<T, E>
where
    Operation: FnMut() -> OperationFuture,
    OperationFuture: Future<Output = Result<T, E>>,
    ShouldRetry: Fn(&E) -> bool + Clone,
    E: Send + 'static,
{
    let backoff = ExponentialBackoffBuilder::new()
        .with_initial_interval(SYNC_INITIAL_INTERVAL)
        .with_multiplier(2.0)
        .with_max_interval(SYNC_MAX_INTERVAL)
        .with_randomization_factor(0.0)
        .with_max_elapsed_time(None)
        .build();
    let mut attempts = 0;

    retry(backoff, || {
        attempts += 1;
        let attempt = attempts;
        let operation_result = operation();
        let should_retry = should_retry.clone();
        async move {
            match operation_result.await {
                Ok(value) => Ok(value),
                Err(error) if attempt < SYNC_MAX_ATTEMPTS && should_retry(&error) => {
                    Err(backoff::Error::transient(error))
                }
                Err(error) => Err(backoff::Error::permanent(error)),
            }
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[tokio::test]
    async fn retries_transient_failures_then_returns_success() {
        // GIVEN an operation that fails transiently twice, WHEN it is retried,
        // THEN the third attempt succeeds.
        let attempts = Arc::new(AtomicUsize::new(0));
        let operation_attempts = Arc::clone(&attempts);

        let result = retry_sync(
            move || {
                let attempt = operation_attempts.fetch_add(1, Ordering::SeqCst) + 1;
                async move {
                    if attempt < SYNC_MAX_ATTEMPTS {
                        Err("transient")
                    } else {
                        Ok("success")
                    }
                }
            },
            |error| *error == "transient",
        )
        .await;

        assert_eq!(result, Ok("success"));
        assert_eq!(attempts.load(Ordering::SeqCst), SYNC_MAX_ATTEMPTS);
    }

    #[tokio::test]
    async fn returns_permanent_failure_without_retrying() {
        // GIVEN an operation that fails permanently, WHEN it is executed,
        // THEN the failure is returned after one attempt.
        let attempts = Arc::new(AtomicUsize::new(0));
        let operation_attempts = Arc::clone(&attempts);

        let result: Result<(), &str> = retry_sync(
            move || {
                operation_attempts.fetch_add(1, Ordering::SeqCst);
                async { Err("permanent") }
            },
            |_| false,
        )
        .await;

        assert_eq!(result, Err("permanent"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn returns_transient_failure_after_three_total_attempts() {
        // GIVEN an operation that always fails transiently, WHEN the retry
        // budget is exhausted, THEN the final failure is returned.
        let attempts = Arc::new(AtomicUsize::new(0));
        let operation_attempts = Arc::clone(&attempts);

        let result: Result<(), &str> = retry_sync(
            move || {
                operation_attempts.fetch_add(1, Ordering::SeqCst);
                async { Err("serialization conflict") }
            },
            |_| true,
        )
        .await;

        assert_eq!(result, Err("serialization conflict"));
        assert_eq!(attempts.load(Ordering::SeqCst), SYNC_MAX_ATTEMPTS);
    }
}
