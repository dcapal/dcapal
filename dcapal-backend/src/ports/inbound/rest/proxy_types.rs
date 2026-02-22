use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartResponse {
    pub chart: Option<YahooChartPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartPayload {
    pub error: Option<serde_json::Value>,
    pub result: Option<Vec<YahooChartResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartResult {
    pub meta: Option<YahooChartMeta>,
    pub indicators: Option<YahooChartIndicators>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartMeta {
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartIndicators {
    pub quote: Option<Vec<YahooChartQuote>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YahooChartQuote {
    pub close: Option<Vec<Option<f64>>>,
}
