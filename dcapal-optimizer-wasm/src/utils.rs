use std::sync::Once;

use log::Level;

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

pub trait Round {
    fn round_n(&self, decimals: u32) -> Self;
}

impl Round for f64 {
    fn round_n(&self, decimals: u32) -> Self {
        let pow = f64::powi(10.0, decimals as i32);

        (self * pow).round() / pow
    }
}
