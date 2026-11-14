//! `GET /assets/chart/{symbol}`: historical chart proxy.

use axum::{
    extract::{Path, Query, State},
    response::Response,
};

use crate::{
    AppContext,
    error::Result,
    ports::inbound::rest::types::assets::chart::{
        GetAssetChartPath, GetAssetChartQuery, YahooChartResponse,
    },
};

#[utoipa::path(
    get,
    path = "/assets/chart/{symbol}",
    params(GetAssetChartPath, GetAssetChartQuery),
    responses(
        (
            status = 200,
            description = "Provider payload passthrough",
            body = YahooChartResponse,
            example = json!(
                {
                    "chart": {
                        "error": null,
                        "result": [
                            {
                                "meta": {
                                    "currency": "USD"
                                },
                                "indicators": {
                                    "quote": [
                                        {
                                            "close": [65231.12, 65310.44, 65195.87]
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                }
            )
        )
    )
)]
/// Proxies a historical chart request to the configured market-data provider.
pub async fn get_assets_chart(
    Path(path): Path<GetAssetChartPath>,
    Query(params): Query<GetAssetChartQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    Ok(ctx
        .providers
        .yahoo
        .chart(path.symbol, params.start_period, params.end_period)
        .await)
}
