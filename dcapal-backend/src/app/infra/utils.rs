use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::Future;
use tokio::sync::{watch, OnceCell, RwLock};

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

    pub async fn get(&self) -> Option<ExpiringOnceCellValue<T>> {
        let lock = self.cell.read().await;
        let Some(v) = lock.get() else {
            return None;
        };

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

    fn time_to_live_chrono(&self) -> chrono::Duration {
        chrono::Duration::from_std(self.time_to_live()).unwrap()
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
