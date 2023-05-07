use std::sync::Once;

use log::Level;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::AMOUNT_DECIMALS;
use crate::WEIGHT_DECIMALS;

static INIT: Once = Once::new();

pub fn require_init() {
    set_panic_hook();
    INIT.call_once(|| {
        wasm_logger::init(
            wasm_logger::Config::new(max_log_level()).module_prefix("dcapal_optimizer_wasm"),
        );
    });
}

fn max_log_level() -> Level {
    #[cfg(debug_assertions)]
    return Level::Debug;
    #[cfg(not(debug_assertions))]
    return Level::Info;
}

fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub fn parse_amount(amount: f64) -> Decimal {
    Decimal::from_f64(amount).unwrap().round_dp(AMOUNT_DECIMALS)
}

pub fn parse_weight(weight: f64) -> Decimal {
    Decimal::from_f64(weight).unwrap().round_dp(WEIGHT_DECIMALS)
}
