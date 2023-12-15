//! The [`adapter`](self) module contains adapters to third-party services

mod cw;
mod ipapi;
mod kraken;
mod yahoo;

use std::sync::Arc;

pub use cw::*;
pub use ipapi::*;
pub use kraken::*;
pub use yahoo::*;

use failsafe::{
    backoff::EqualJittered,
    failure_policy::{ConsecutiveFailures, OrElse, SuccessRateOverTimeWindow},
    StateMachine,
};

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
