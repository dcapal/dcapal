//! Provenance of market data: why a price is missing and where a price came from.

use std::collections::HashMap;

use strum::IntoEnumIterator;

use crate::{
    DateTime,
    app::domain::{asset::AssetRef, portfolio::Provider},
};

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

/// Where the value of a quote comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum QuoteSource {
    /// The provider answered the latest refresh with this value.
    Provider,
    /// The latest refresh failed; this is the last value the provider supplied.
    LastKnown,
}

/// Outcome of asking for one price or rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum QuoteStatus {
    /// A positive price is available.
    Usable,
    /// The provider has no usable price.
    Unavailable,
    /// The provider could not be reached or answered with an error.
    ProviderError,
    /// The provider answered with an unreadable payload.
    Malformed,
    /// The market is known but has never been priced.
    Pending,
}

impl From<FailureKind> for QuoteStatus {
    fn from(kind: FailureKind) -> Self {
        match kind {
            FailureKind::Unavailable => Self::Unavailable,
            FailureKind::ProviderError => Self::ProviderError,
            FailureKind::Malformed => Self::Malformed,
        }
    }
}

/// A stored market with its latest price and the latest refresh failure, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredQuote {
    /// Lower-case symbol of the base asset.
    pub base: String,
    /// Lower-case symbol of the quote asset.
    pub quote: String,
    /// Provider that supplies the price.
    pub provider: Provider,
    /// Last known price, absent until the first price is stored.
    pub price: Option<f64>,
    /// When the provider observed the price.
    pub observed_at: Option<DateTime>,
    /// When the backend stored the price.
    pub fetched_at: Option<DateTime>,
    /// Latest refresh failure, cleared by the next successful refresh.
    pub failure: Option<PriceFailure>,
    /// When the latest refresh failure happened.
    pub failed_at: Option<DateTime>,
}

impl StoredQuote {
    /// The quote status and source this stored market amounts to.
    ///
    /// A price next to a refresh failure is still usable but is the last known value;
    /// no price with a failure carries the failure's kind; no price and no failure
    /// means the market was never priced.
    pub fn status(&self) -> (QuoteStatus, Option<QuoteSource>) {
        match (&self.price, &self.failure) {
            (Some(_), None) => (QuoteStatus::Usable, Some(QuoteSource::Provider)),
            (Some(_), Some(_)) => (QuoteStatus::Usable, Some(QuoteSource::LastKnown)),
            (None, Some(failure)) => (failure.kind.into(), None),
            (None, None) => (QuoteStatus::Pending, None),
        }
    }
}

/// The current price of one asset with its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceQuote {
    /// The asset the price belongs to.
    pub asset: AssetRef,
    /// Provider that supplied (or would supply) the price.
    pub provider: Provider,
    /// Currency the price is expressed in.
    pub currency: Option<String>,
    /// The positive price, absent unless [`QuoteStatus::Usable`].
    pub price: Option<f64>,
    /// When the provider observed the price.
    pub observed_at: Option<DateTime>,
    /// When the backend obtained the price.
    pub fetched_at: Option<DateTime>,
    /// Where the price comes from, absent when there is none.
    pub source: Option<QuoteSource>,
    /// Outcome of the request.
    pub status: QuoteStatus,
    /// Why the price is missing, or why the shown value is the last known one.
    pub failure: Option<PriceFailure>,
    /// When the failure happened, when it was recorded.
    pub failed_at: Option<DateTime>,
}

impl PriceQuote {
    /// A quote that has no price because of `failure`.
    pub fn failed(asset: AssetRef, failure: PriceFailure) -> Self {
        let provider = asset.provider();
        Self {
            asset,
            provider,
            currency: None,
            price: None,
            observed_at: None,
            fetched_at: None,
            source: None,
            status: failure.kind.into(),
            failure: Some(failure),
            failed_at: None,
        }
    }

    /// A quote built from a stored market priced in `currency`.
    pub fn from_stored(asset: AssetRef, currency: &str, stored: &StoredQuote) -> Self {
        let (status, source) = stored.status();
        Self {
            asset,
            provider: stored.provider,
            currency: Some(currency.to_string()),
            price: stored.price,
            observed_at: stored.observed_at,
            fetched_at: stored.fetched_at,
            source,
            status,
            failure: stored.failure.clone(),
            failed_at: stored.failed_at,
        }
    }
}

/// How an FX rate was derived from stored markets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum Synthetic {
    /// Read directly from a stored market (or the identity rate of one currency).
    None,
    /// The reciprocal of a stored market.
    Inverse,
    /// Computed from two stored markets through USD.
    Triangulated,
}

/// A stored market that contributed to an FX rate.
#[derive(Debug, Clone, PartialEq)]
pub struct FxLeg {
    /// Lower-case base symbol of the stored market.
    pub base: String,
    /// Lower-case quote symbol of the stored market.
    pub quote: String,
    /// Provider that supplied the leg.
    pub provider: Provider,
    /// When the provider observed the leg.
    pub observed_at: Option<DateTime>,
}

/// An FX rate: the value of one unit of `base` in `quote`, with its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct FxRate {
    /// Upper-case base currency.
    pub base: String,
    /// Upper-case quote currency.
    pub quote: String,
    /// The positive rate, absent unless [`QuoteStatus::Usable`].
    pub rate: Option<f64>,
    /// Provider of the legs, when they all share one.
    pub provider: Option<Provider>,
    /// Oldest observation time among the legs.
    pub observed_at: Option<DateTime>,
    /// Oldest storage time among the legs.
    pub fetched_at: Option<DateTime>,
    /// How the rate was derived.
    pub synthetic: Synthetic,
    /// The stored markets the rate was derived from.
    pub legs: Vec<FxLeg>,
    /// Where the value comes from, absent when there is none.
    pub source: Option<QuoteSource>,
    /// Outcome of the request.
    pub status: QuoteStatus,
    /// Why the rate is missing, or why the shown value is the last known one.
    pub failure: Option<PriceFailure>,
}

/// USD, the pivot currency of every stored FX market.
const PIVOT: &str = "usd";

/// Ids of the stored markets an FX rate between two currencies can be derived from.
pub fn fx_candidate_markets(base: &str, quote: &str) -> Vec<String> {
    let (base, quote) = (base.to_lowercase(), quote.to_lowercase());
    let mut ids = vec![
        format!("{base}{quote}"),
        format!("{quote}{base}"),
        format!("{base}{PIVOT}"),
        format!("{PIVOT}{base}"),
        format!("{quote}{PIVOT}"),
        format!("{PIVOT}{quote}"),
    ];
    ids.sort();
    ids.dedup();
    ids
}

/// Derives the rate of `base` in `quote` from stored markets, keyed by market id.
///
/// A direct market wins over its inverse, which wins over a triangulation through
/// USD. A currency against itself is 1. A leg whose latest refresh failed still
/// contributes its last known price, and the rate is then reported as last known with
/// that failure. When no rate can be derived the status says why: a failed leg
/// reports its failure, a never-priced leg is pending, and no stored market at all
/// is unavailable.
pub fn derive_fx_rate(base: &str, quote: &str, stored: &HashMap<String, StoredQuote>) -> FxRate {
    let (b, q) = (base.to_lowercase(), quote.to_lowercase());
    let empty = |synthetic, status, failure| FxRate {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        rate: None,
        provider: None,
        observed_at: None,
        fetched_at: None,
        synthetic,
        legs: Vec::new(),
        source: None,
        status,
        failure,
    };

    if b == q {
        let mut identity = empty(Synthetic::None, QuoteStatus::Usable, None);
        identity.rate = Some(1.0);
        identity.source = Some(QuoteSource::Provider);
        return identity;
    }

    // A priced stored market as (value of one base unit in its quote unit, market).
    let priced = |id: String| stored.get(&id).and_then(|m| m.price.map(|p| (p, m)));
    let direct = priced(format!("{b}{q}")).map(|(p, m)| (p, Synthetic::None, vec![m]));
    let inverse = || priced(format!("{q}{b}")).map(|(p, m)| (1.0 / p, Synthetic::Inverse, vec![m]));
    let triangulated = || {
        // The value of one unit of `currency` in USD, with the market it comes from.
        let in_pivot = |currency: &str| -> Option<(f64, Option<&StoredQuote>)> {
            if currency == PIVOT {
                return Some((1.0, None));
            }
            priced(format!("{currency}{PIVOT}"))
                .map(|(p, m)| (p, Some(m)))
                .or_else(|| priced(format!("{PIVOT}{currency}")).map(|(p, m)| (1.0 / p, Some(m))))
        };
        let (base_usd, base_leg) = in_pivot(&b)?;
        let (quote_usd, quote_leg) = in_pivot(&q)?;
        Some((
            base_usd / quote_usd,
            Synthetic::Triangulated,
            base_leg.into_iter().chain(quote_leg).collect(),
        ))
    };

    let Some((rate, synthetic, legs)) = direct.or_else(inverse).or_else(triangulated) else {
        let candidates = fx_candidate_markets(&b, &q)
            .into_iter()
            .filter_map(|id| stored.get(&id))
            .collect::<Vec<_>>();
        return match candidates.iter().find_map(|m| m.failure.clone()) {
            Some(failure) => empty(Synthetic::None, failure.kind.into(), Some(failure)),
            None if candidates.is_empty() => empty(
                Synthetic::None,
                QuoteStatus::Unavailable,
                Some(PriceFailure::unavailable(
                    "no FX market links the currencies",
                )),
            ),
            None => empty(Synthetic::None, QuoteStatus::Pending, None),
        };
    };

    // The rate is only as fresh as its stalest leg.
    let failure = legs.iter().find_map(|m| m.failure.clone());
    FxRate {
        base: base.to_uppercase(),
        quote: quote.to_uppercase(),
        rate: Some(rate),
        provider: legs
            .first()
            .map(|m| m.provider)
            .filter(|p| legs.iter().all(|m| m.provider == *p)),
        observed_at: legs.iter().filter_map(|m| m.observed_at).min(),
        fetched_at: legs.iter().filter_map(|m| m.fetched_at).min(),
        synthetic,
        source: Some(if failure.is_some() {
            QuoteSource::LastKnown
        } else {
            QuoteSource::Provider
        }),
        status: QuoteStatus::Usable,
        failure,
        legs: legs
            .into_iter()
            .map(|m| FxLeg {
                base: m.base.clone(),
                quote: m.quote.clone(),
                provider: m.provider,
                observed_at: m.observed_at,
            })
            .collect(),
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

    fn stored(base: &str, quote: &str, price: Option<f64>) -> (String, StoredQuote) {
        (
            format!("{base}{quote}"),
            StoredQuote {
                base: base.into(),
                quote: quote.into(),
                provider: Provider::YF,
                price,
                observed_at: price.map(|_| DateTime::from_second(1_700_000_000).unwrap()),
                fetched_at: price.map(|_| DateTime::from_second(1_700_000_100).unwrap()),
                failure: None,
                failed_at: None,
            },
        )
    }

    fn markets(entries: Vec<(String, StoredQuote)>) -> HashMap<String, StoredQuote> {
        entries.into_iter().collect()
    }

    /// GIVEN stored EUR/USD and GBP/USD markets,
    /// WHEN EUR/USD, USD/EUR, EUR/GBP and EUR/EUR are derived,
    /// THEN they are read directly, inverted, triangulated through USD, and 1, each
    /// naming its legs.
    #[test]
    fn fx_rates_are_direct_inverse_triangulated_or_identity() {
        let m = markets(vec![
            stored("eur", "usd", Some(1.25)),
            stored("gbp", "usd", Some(1.5)),
        ]);

        let direct = derive_fx_rate("EUR", "USD", &m);
        assert_eq!(
            (direct.rate, direct.synthetic),
            (Some(1.25), Synthetic::None)
        );
        assert_eq!(direct.legs.len(), 1);
        assert_eq!(direct.provider, Some(Provider::YF));

        let inverse = derive_fx_rate("USD", "EUR", &m);
        assert_eq!(
            (inverse.rate, inverse.synthetic),
            (Some(0.8), Synthetic::Inverse)
        );

        let cross = derive_fx_rate("EUR", "GBP", &m);
        assert_eq!(cross.synthetic, Synthetic::Triangulated);
        assert!((cross.rate.unwrap() - 1.25 / 1.5).abs() < 1e-12);
        assert_eq!(cross.legs.len(), 2);

        let same = derive_fx_rate("EUR", "EUR", &m);
        assert_eq!((same.rate, same.legs.len()), (Some(1.0), 0));
    }

    /// GIVEN FX markets that are missing, never priced, or whose refresh failed,
    /// WHEN a rate is derived,
    /// THEN the status is unavailable, pending, or the failure's kind; and a priced
    /// leg with a failed refresh still yields its last known rate, flagged as such.
    #[test]
    fn fx_rate_status_explains_why_a_rate_is_missing() {
        let none = HashMap::new();
        assert_eq!(
            derive_fx_rate("EUR", "GBP", &none).status,
            QuoteStatus::Unavailable
        );

        let unpriced = markets(vec![
            stored("eur", "usd", None),
            stored("gbp", "usd", Some(1.5)),
        ]);
        assert_eq!(
            derive_fx_rate("EUR", "GBP", &unpriced).status,
            QuoteStatus::Pending
        );

        let mut failing = stored("eur", "usd", None);
        failing.1.failure = Some(PriceFailure::malformed("bad body"));
        let broken = markets(vec![failing, stored("gbp", "usd", Some(1.5))]);
        let rate = derive_fx_rate("EUR", "GBP", &broken);
        assert_eq!(rate.status, QuoteStatus::Malformed);
        assert_eq!(rate.failure.unwrap().message, "bad body");

        let mut stale = stored("eur", "usd", Some(1.25));
        stale.1.failure = Some(PriceFailure::provider_error("HTTP 503"));
        let last_known = derive_fx_rate("EUR", "USD", &markets(vec![stale]));
        assert_eq!(last_known.status, QuoteStatus::Usable);
        assert_eq!(last_known.source, Some(QuoteSource::LastKnown));
        assert_eq!(last_known.failure.unwrap().kind, FailureKind::ProviderError);
    }
}
