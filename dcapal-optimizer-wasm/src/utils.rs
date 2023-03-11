pub fn set_panic_hook() {
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
    fn round_n(&self, decimals: i32) -> Self;
}

impl Round for f64 {
    fn round_n(&self, decimals: i32) -> Self {
        let pow = f64::powi(10.0, decimals);

        (self * pow).round() / pow
    }
}
