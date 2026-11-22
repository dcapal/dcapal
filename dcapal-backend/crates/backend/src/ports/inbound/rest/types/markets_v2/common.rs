//! Wire types shared by the v2 market-data endpoints.

use jiff::Timestamp;
use serde::Serialize;
use utoipa::ToSchema;

use crate::app::domain::{
    market_quote::{FailureKind, PriceFailure, QuoteSource, QuoteStatus},
    portfolio::Provider,
};

/// Why a price, rate or search has no usable value.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FailureKindV2 {
    /// The provider answered but has no usable price.
    Unavailable,
    /// The provider could not be reached or answered with an error.
    ProviderError,
    /// The provider answered with an unreadable payload.
    Malformed,
}

/// Outcome of one price or rate.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuoteStatusV2 {
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

/// Where a price comes from.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuoteSourceV2 {
    /// The provider answered the latest refresh with this value.
    Provider,
    /// The latest refresh failed; this is the last value the provider supplied.
    LastKnown,
}

/// A visible failure: what went wrong and, when recorded, when.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FailureV2 {
    pub kind: FailureKindV2,
    pub message: String,
    /// RFC 3339 time the failure was recorded, when known.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub occurred_at: Option<String>,
}

impl From<FailureKind> for FailureKindV2 {
    fn from(kind: FailureKind) -> Self {
        match kind {
            FailureKind::Unavailable => Self::Unavailable,
            FailureKind::ProviderError => Self::ProviderError,
            FailureKind::Malformed => Self::Malformed,
        }
    }
}

impl From<QuoteStatus> for QuoteStatusV2 {
    fn from(status: QuoteStatus) -> Self {
        match status {
            QuoteStatus::Usable => Self::Usable,
            QuoteStatus::Unavailable => Self::Unavailable,
            QuoteStatus::ProviderError => Self::ProviderError,
            QuoteStatus::Malformed => Self::Malformed,
            QuoteStatus::Pending => Self::Pending,
        }
    }
}

impl From<QuoteSource> for QuoteSourceV2 {
    fn from(source: QuoteSource) -> Self {
        match source {
            QuoteSource::Provider => Self::Provider,
            QuoteSource::LastKnown => Self::LastKnown,
        }
    }
}

impl FailureV2 {
    /// Describes a failure that happened at `at`, when that is known.
    ///
    /// The message is a generic text of the failure kind: what the provider or the HTTP
    /// client reported is for logs only and never reaches a client.
    pub fn new(failure: PriceFailure, at: Option<Timestamp>) -> Self {
        let message = match failure.kind {
            FailureKind::Unavailable => "The provider has no usable price.",
            FailureKind::ProviderError => {
                "The provider could not be reached or answered with an error."
            }
            FailureKind::Malformed => "The provider answered with an unreadable payload.",
        };
        Self {
            kind: failure.kind.into(),
            message: message.to_string(),
            occurred_at: at.map(format_time),
        }
    }
}

/// Formats a time as RFC 3339 in UTC.
pub fn format_time(time: Timestamp) -> String {
    time.to_string()
}

/// Formats a price as a decimal string with the shortest digits that read back exactly.
pub fn format_decimal(value: f64) -> String {
    value.to_string()
}

/// The canonical v2 identifier of a provider.
pub fn provider_name(provider: Provider) -> String {
    provider.as_str().to_string()
}
