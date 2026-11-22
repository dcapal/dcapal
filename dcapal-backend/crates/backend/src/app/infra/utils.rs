use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::Future;
use tokio::{
    sync::{OnceCell, RwLock, watch},
    task::{JoinError, JoinHandle},
};
use tracing::{error, info};

use crate::error::Result;

pub struct ExpiringOnceCellValue<T> {
    pub value: T,
    pub is_expired: bool,
}

pub struct ExpiringOnceCell<T>
where
    T: Clone,
{
    cell: RwLock<OnceCell<T>>,
    is_expired: Arc<dyn Fn(&T) -> bool + Send + Sync>,
}

impl<T> ExpiringOnceCell<T>
where
    T: Clone,
{
    pub fn new<F>(is_expired: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        Self {
            cell: RwLock::new(OnceCell::new()),
            is_expired: Arc::new(is_expired),
        }
    }

    pub async fn get_or_try_init<E, I, Fut>(&self, init: I) -> std::result::Result<T, E>
    where
        I: Fn() -> Fut,
        Fut: Future<Output = std::result::Result<T, E>>,
    {
        {
            let lock = self.cell.read().await;
            let v = lock.get_or_try_init(|| async { init().await }).await?;

            if !(self.is_expired)(v) {
                return Ok(v.clone());
            }
        }

        // If value is outdated, clear OnceCell if none is updating it
        if self.cell.try_read().is_ok()
            && let Ok(mut lock) = self.cell.try_write()
        {
            lock.take();
        }

        // Compute new value
        let lock = self.cell.read().await;
        let v = lock.get_or_try_init(init).await?;

        Ok(v.clone())
    }

    pub async fn get(&self) -> Option<ExpiringOnceCellValue<T>> {
        let lock = self.cell.read().await;
        let v = lock.get()?;

        if (self.is_expired)(v) {
            Some(ExpiringOnceCellValue {
                value: v.clone(),
                is_expired: true,
            })
        } else {
            Some(ExpiringOnceCellValue {
                value: v.clone(),
                is_expired: false,
            })
        }
    }
}

impl<T> Default for ExpiringOnceCell<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new(|_| true)
    }
}

pub trait Expiring {
    fn is_outdated(&self) -> bool;
    fn time_to_live(&self) -> Duration;

    /// Returns the remaining lifetime as a Jiff span.
    fn time_to_live_span(&self) -> jiff::Span {
        jiff::Span::try_from(self.time_to_live()).unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum ExpiringOption<T: Expiring> {
    Some(T),
    None(Instant, Duration),
}

impl<T: Expiring> From<ExpiringOption<T>> for Option<T> {
    fn from(v: ExpiringOption<T>) -> Self {
        match v {
            ExpiringOption::Some(v) => Some(v),
            ExpiringOption::None(_, _) => None,
        }
    }
}

impl<T: Expiring> Expiring for ExpiringOption<T> {
    fn is_outdated(&self) -> bool {
        match self {
            ExpiringOption::Some(v) => v.is_outdated(),
            ExpiringOption::None(start, ttl) => start.elapsed() >= *ttl,
        }
    }

    fn time_to_live(&self) -> Duration {
        match self {
            ExpiringOption::Some(v) => v.time_to_live(),
            ExpiringOption::None(start, ttl) => (*start + *ttl) - Instant::now(),
        }
    }
}

pub type StopToken = watch::Receiver<bool>;

pub async fn should_stop(stop_rx: &mut StopToken) {
    while stop_rx.changed().await.is_ok() {
        if *stop_rx.borrow_and_update() {
            return;
        }
    }
}

/// Spawns a long-running background task and reports how it ended.
///
/// The task runs on its own Tokio task, and a thin supervisor awaits it. A task
/// that returns an error is logged at ERROR with its full cause chain (the
/// `Debug` form of [`crate::error::DcaError`]); a panic or cancellation is logged at ERROR with
/// the task `name` and the panic message, so a worker that dies is never silent.
/// A normal return is logged at INFO. The returned handle completes once the
/// task has ended and has been reported, and never propagates the panic.
pub fn spawn_supervised<F>(name: &'static str, task: F) -> JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    let inner = tokio::spawn(task);
    tokio::spawn(async move {
        match inner.await {
            Ok(Ok(())) => info!(task = name, "Background task finished"),
            Ok(Err(e)) => error!(task = name, "Background task failed: {e:?}"),
            Err(e) => {
                let panicked = e.is_panic();
                error!(
                    task = name,
                    panicked,
                    "Background task died: {}",
                    describe_join_error(e)
                );
            }
        }
    })
}

/// Describes why a task ended abnormally, including the panic message if any.
fn describe_join_error(error: JoinError) -> String {
    if !error.is_panic() {
        return error.to_string();
    }

    let payload = error.into_panic();
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panic with a non-string payload".to_string())
}

#[cfg(test)]
mod supervised_tests {
    use super::spawn_supervised;
    use crate::error::DcaError;

    #[tokio::test]
    async fn supervisor_survives_a_panicking_task() {
        // GIVEN a background task that panics,
        // WHEN it is spawned under the supervisor,
        let handle = spawn_supervised("panicking-test-task", async {
            panic!("boom");
        });

        // THEN the supervisor reports it and finishes normally instead of
        // propagating the panic to whoever awaits the handle.
        assert!(handle.await.is_ok());
    }

    #[tokio::test]
    async fn supervisor_finishes_after_a_task_error() {
        // GIVEN a background task that returns an error,
        // WHEN it is spawned under the supervisor,
        let handle = spawn_supervised("failing-test-task", async {
            Err(DcaError::Generic("boom".to_string()))
        });

        // THEN the supervisor reports it and finishes normally.
        assert!(handle.await.is_ok());
    }

    #[tokio::test]
    async fn supervisor_finishes_after_a_normal_return() {
        // GIVEN a background task that returns normally,
        // WHEN it is spawned under the supervisor,
        let handle = spawn_supervised("returning-test-task", async { Ok(()) });

        // THEN the supervisor finishes normally too.
        assert!(handle.await.is_ok());
    }
}
