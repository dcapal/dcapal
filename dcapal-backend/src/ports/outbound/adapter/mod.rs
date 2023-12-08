//! The [`adapter`](self) module contains adapters to third-party services

mod cw;
mod ipapi;
mod kraken;
mod yahoo;

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
