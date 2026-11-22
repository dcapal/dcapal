//! The [`adapter`](self) module contains adapters to third-party services

mod asset_provider;
mod cw;
mod fiat_asset;
mod ipapi;
mod kraken;
mod kraken_asset;
mod yahoo;

use std::sync::Arc;

pub use asset_provider::*;
pub use cw::*;
use failsafe::{
    StateMachine,
    backoff::EqualJittered,
    failure_policy::{ConsecutiveFailures, OrElse, SuccessRateOverTimeWindow},
};
pub use fiat_asset::CatalogFiatAssetProvider;
pub use ipapi::*;
pub use kraken::*;
pub use kraken_asset::KrakenAssetProvider;
pub use yahoo::*;

use crate::app::domain::market_quote::{FailureKind, PriceFailure};

/// A failed price call as the adapters see it: the visible failure and whether a
/// timeout caused it (the provider metrics label timeouts separately).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PriceCallError {
    pub failure: PriceFailure,
    pub timed_out: bool,
}

impl From<PriceFailure> for PriceCallError {
    fn from(failure: PriceFailure) -> Self {
        Self {
            failure,
            timed_out: false,
        }
    }
}

impl PriceCallError {
    /// A provider error caused by an elapsed request deadline.
    pub(crate) fn timeout(message: impl Into<String>) -> Self {
        Self {
            failure: PriceFailure::provider_error(message),
            timed_out: true,
        }
    }

    /// Classifies a price call for the provider metrics: a missing price is `not_found`.
    pub(crate) fn outcome<T>(
        result: &std::result::Result<T, Self>,
    ) -> crate::app::infra::stats::ProviderOutcome {
        use crate::app::infra::stats::ProviderOutcome;
        match result {
            Ok(_) => ProviderOutcome::Ok,
            Err(e) if e.timed_out => ProviderOutcome::Timeout,
            Err(e) if e.failure.kind == FailureKind::Unavailable => ProviderOutcome::NotFound,
            Err(_) => ProviderOutcome::Error,
        }
    }
}

type DefaultCircuitBreaker = StateMachine<
    OrElse<SuccessRateOverTimeWindow<EqualJittered>, ConsecutiveFailures<EqualJittered>>,
    (),
>;

#[derive(Clone)]
pub struct PriceProviders {
    pub cw: Arc<CryptoWatchProvider>,
    pub kraken: Arc<KrakenProvider>,
    pub yahoo: Arc<YahooProvider>,
    pub ipapi: Arc<IpApi>,
}
