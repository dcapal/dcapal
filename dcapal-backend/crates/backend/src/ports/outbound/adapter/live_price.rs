//! Port for providers that quote a listed symbol on demand.

use async_trait::async_trait;

use crate::{DateTime, app::domain::market_quote::PriceFailure};

/// The current price of one listed symbol, in the currency the provider lists it in.
#[derive(Debug, Clone, PartialEq)]
pub struct LivePrice {
    /// Strictly positive price.
    pub price: f64,
    /// Currency of the price exactly as the provider spells it (`GBp` is pence).
    pub currency: String,
    /// When the provider observed the price, when it says so.
    pub observed_at: Option<DateTime>,
}

/// A provider that answers the current price of a symbol at request time.
///
/// Implementations ask only their own provider and report every failure.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LivePriceProvider: Send + Sync {
    /// Fetches the current price of `symbol`.
    async fn current_price(&self, symbol: &str) -> Result<LivePrice, PriceFailure>;
}
