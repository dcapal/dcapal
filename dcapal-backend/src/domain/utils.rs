use std::time::{Duration, Instant};

use futures::Future;
use tokio::sync::{OnceCell, RwLock};

pub struct ExpiringOnceCell<T>
where
    T: Clone,
{
    cell: RwLock<OnceCell<T>>,
}

impl<T> ExpiringOnceCell<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            cell: RwLock::new(OnceCell::new()),
        }
    }

    pub async fn get_or_try_init<E, I, Fut, F>(
        &self,
        init: I,
        is_expired: F,
    ) -> std::result::Result<T, E>
    where
        I: Fn() -> Fut,
        Fut: Future<Output = std::result::Result<T, E>>,
        F: FnOnce(&T) -> bool,
    {
        {
            let lock = self.cell.read().await;
            let v = lock.get_or_try_init(|| async { init().await }).await?;

            if !is_expired(v) {
                return Ok(v.clone());
            }
        }

        // If value is outdated, clear OnceCell if none is updating it
        if self.cell.try_read().is_ok() {
            if let Ok(mut lock) = self.cell.try_write() {
                lock.take();
            }
        }

        // Compute new value
        let lock = self.cell.read().await;
        let v = lock.get_or_try_init(init).await?;

        Ok(v.clone())
    }
}

impl<T> Default for ExpiringOnceCell<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

pub trait Expiring {
    fn is_outdated(&self) -> bool;
    fn time_to_live(&self) -> Duration;
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
