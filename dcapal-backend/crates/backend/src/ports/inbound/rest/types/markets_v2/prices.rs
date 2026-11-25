//! Wire types of `GET /markets/v2/prices`.

use serde::Serialize;
use utoipa::ToSchema;

use super::common::{
    FailureV2, QuoteSourceV2, QuoteStatusV2, format_decimal, format_time, provider_name,
};
use crate::{
    app::domain::market_quote::PriceQuote,
    ports::inbound::rest::types::v2_common::{AssetRefV2, v2_provider_schema},
};

/// The current price of one requested asset with its provenance.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceV2 {
    /// The requested asset, normalized.
    pub asset: AssetRefV2,
    /// Provider that supplied the price.
    #[schema(schema_with = v2_provider_schema)]
    pub provider: String,
    /// Currency the price is expressed in, as the provider spells it.
    pub currency: Option<String>,
    /// Positive decimal string; null unless `status` is `usable`.
    pub price: Option<String>,
    /// RFC 3339 time the provider observed the price.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub observed_at: Option<String>,
    /// RFC 3339 time the backend obtained the price.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub fetched_at: Option<String>,
    /// `provider` for a fresh value, `lastKnown` when the latest refresh failed.
    pub source: Option<QuoteSourceV2>,
    pub status: QuoteStatusV2,
    /// Why the price is missing, or why a last-known value is stale.
    pub error: Option<FailureV2>,
}

/// Prices of the requested assets, in request order.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PricesV2Response {
    pub prices: Vec<PriceV2>,
}

impl From<PriceQuote> for PriceV2 {
    fn from(quote: PriceQuote) -> Self {
        Self {
            asset: AssetRefV2 {
                provider: provider_name(quote.asset.provider()),
                symbol: quote.asset.symbol().to_string(),
            },
            provider: provider_name(quote.provider),
            currency: quote.currency,
            price: quote.price.map(format_decimal),
            observed_at: quote.observed_at.map(format_time),
            fetched_at: quote.fetched_at.map(format_time),
            source: quote.source.map(Into::into),
            status: quote.status.into(),
            error: quote.failure.map(|f| FailureV2::new(f, quote.failed_at)),
        }
    }
}
