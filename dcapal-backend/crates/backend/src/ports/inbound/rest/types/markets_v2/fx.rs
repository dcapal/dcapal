//! Wire types of `GET /markets/v2/fx`.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::common::{
    FailureV2, QuoteSourceV2, QuoteStatusV2, format_decimal, format_time, provider_name,
};
use crate::{
    app::domain::market_quote::{FxLeg, FxRate, Synthetic},
    ports::inbound::rest::types::v2_common::v2_provider_schema,
};

/// Query parameters of the FX rate endpoint.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FxQuery {
    /// Base currency: a catalog fiat code such as `EUR`. Required.
    pub base: Option<String>,
    /// Quote currency: a catalog fiat code such as `USD`. Required.
    pub quote: Option<String>,
}

/// How an FX rate was derived.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyntheticV2 {
    /// Read directly from a stored market, or the identity rate of one currency.
    None,
    /// The reciprocal of a stored market.
    Inverse,
    /// Computed from two stored markets through USD.
    Triangulated,
}

/// A stored market an FX rate was derived from.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FxLegV2 {
    pub base: String,
    pub quote: String,
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    /// RFC 3339 time the provider observed the leg.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub observed_at: Option<String>,
}

/// The value of one unit of `base` in `quote`, with its provenance.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FxRateV2Response {
    pub base: String,
    pub quote: String,
    /// Positive decimal string; null unless `status` is `usable`.
    pub rate: Option<String>,
    /// Provider of the legs, when they all share one.
    pub provider: Option<String>,
    /// Oldest provider observation time among the legs, RFC 3339.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub observed_at: Option<String>,
    /// Oldest backend storage time among the legs, RFC 3339.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub fetched_at: Option<String>,
    pub synthetic: SyntheticV2,
    pub legs: Vec<FxLegV2>,
    /// `provider` for a fresh value, `lastKnown` when a leg's latest refresh failed.
    pub source: Option<QuoteSourceV2>,
    pub status: QuoteStatusV2,
    pub error: Option<FailureV2>,
}

impl From<Synthetic> for SyntheticV2 {
    fn from(synthetic: Synthetic) -> Self {
        match synthetic {
            Synthetic::None => Self::None,
            Synthetic::Inverse => Self::Inverse,
            Synthetic::Triangulated => Self::Triangulated,
        }
    }
}

impl From<FxLeg> for FxLegV2 {
    fn from(leg: FxLeg) -> Self {
        Self {
            base: leg.base.to_uppercase(),
            quote: leg.quote.to_uppercase(),
            provider: provider_name(leg.provider),
            observed_at: leg.observed_at.map(format_time),
        }
    }
}

impl From<FxRate> for FxRateV2Response {
    fn from(rate: FxRate) -> Self {
        Self {
            base: rate.base,
            quote: rate.quote,
            rate: rate.rate.map(format_decimal),
            provider: rate.provider.map(provider_name),
            observed_at: rate.observed_at.map(format_time),
            fetched_at: rate.fetched_at.map(format_time),
            synthetic: rate.synthetic.into(),
            legs: rate.legs.into_iter().map(Into::into).collect(),
            source: rate.source.map(Into::into),
            status: rate.status.into(),
            error: rate.failure.map(|f| FailureV2::new(f, None)),
        }
    }
}
