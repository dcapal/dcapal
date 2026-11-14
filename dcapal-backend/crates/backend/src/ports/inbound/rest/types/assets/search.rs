//! Wire types of `GET /assets/search`.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
/// Query parameters for provider-backed asset search.
pub struct GetAssetsQuery {
    /// Asset name query.
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooSearchResponse {
    pub quotes: Option<Vec<YahooSearchQuote>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooSearchQuote {
    #[serde(rename = "quoteType")]
    pub quote_type: Option<String>,
    pub longname: Option<String>,
    pub shortname: Option<String>,
    pub symbol: Option<String>,
    pub exchange: Option<String>,
}
