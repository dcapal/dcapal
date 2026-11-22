//! Provenance of market data: why a price is missing and where a price came from.

use strum::IntoEnumIterator;

/// Why a current price could not be produced.
///
/// The camel-case names are the wire identifiers of the v2 market-data contract and
/// the prefix of the failure text stored next to the last known price.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    strum::Display,
    strum::IntoStaticStr,
    strum::EnumString,
    strum::EnumIter,
)]
#[strum(serialize_all = "camelCase")]
pub enum FailureKind {
    /// The provider answered, but has no usable price (unknown symbol, no trades,
    /// missing, zero, negative or non-finite value).
    Unavailable,
    /// The provider could not be reached or answered with an error.
    ProviderError,
    /// The provider answered with a payload that cannot be understood.
    Malformed,
}

/// A visible failure to obtain a price: what went wrong and a readable reason.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct PriceFailure {
    /// Failure category.
    pub kind: FailureKind,
    /// Human-readable reason, free of provider secrets.
    pub message: String,
}

impl PriceFailure {
    /// Creates a failure of the given kind.
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates an [`FailureKind::Unavailable`] failure.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(FailureKind::Unavailable, message)
    }

    /// Creates a [`FailureKind::ProviderError`] failure.
    pub fn provider_error(message: impl Into<String>) -> Self {
        Self::new(FailureKind::ProviderError, message)
    }

    /// Creates a [`FailureKind::Malformed`] failure.
    pub fn malformed(message: impl Into<String>) -> Self {
        Self::new(FailureKind::Malformed, message)
    }

    /// Encodes the failure as the text stored in `market_price.last_error`.
    pub fn to_stored(&self) -> String {
        self.to_string()
    }

    /// Decodes the text stored in `market_price.last_error`.
    ///
    /// Text without a known kind prefix is kept whole as a provider error, so rows
    /// written by older code still surface as failures.
    pub fn from_stored(text: &str) -> Self {
        FailureKind::iter()
            .find_map(|kind| {
                text.strip_prefix(&format!("{kind}: "))
                    .map(|message| Self::new(kind, message))
            })
            .unwrap_or_else(|| Self::provider_error(text))
    }
}

/// Checks that a provider price is a finite, strictly positive number.
///
/// Zero, negative and non-finite values are never prices: they are reported as
/// unavailable instead of being stored or shown as zero.
pub fn validate_price(price: f64) -> Result<f64, PriceFailure> {
    if price.is_finite() && price > 0.0 {
        Ok(price)
    } else {
        Err(PriceFailure::unavailable(format!(
            "provider returned a non-positive or non-finite price ({price})"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GIVEN failures of every kind,
    /// WHEN they are encoded for storage and decoded again,
    /// THEN kind and message survive; text without a kind prefix is a provider error.
    #[test]
    fn stored_failures_round_trip() {
        for kind in FailureKind::iter() {
            let failure = PriceFailure::new(kind, "reason: with colon");
            assert_eq!(PriceFailure::from_stored(&failure.to_stored()), failure);
        }
        assert_eq!(
            PriceFailure::from_stored("legacy text"),
            PriceFailure::provider_error("legacy text")
        );
    }

    /// GIVEN zero, negative, NaN and infinite provider values,
    /// WHEN each is validated,
    /// THEN each is unavailable; a positive value is accepted unchanged.
    #[test]
    fn non_positive_and_non_finite_prices_are_unavailable() {
        for bad in [0.0, -1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                validate_price(bad).unwrap_err().kind,
                FailureKind::Unavailable
            );
        }
        assert_eq!(validate_price(0.0001).unwrap(), 0.0001);
    }
}
