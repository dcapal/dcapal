//! Wire types of `GET /assets/chart/{symbol}`.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
/// Time range parameters for a historical asset chart.
pub struct GetAssetChartQuery {
    /// Chart start timestamp (unix seconds).
    pub start_period: i64,
    /// Chart end timestamp (unix seconds).
    pub end_period: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
/// Path parameters identifying a charted asset.
pub struct GetAssetChartPath {
    /// Asset symbol.
    pub symbol: String,
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
