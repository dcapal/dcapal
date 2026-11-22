//! `GET /markets/v2/assets/{provider}/search`: typed asset search of one provider.

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    AppContext,
    app::{commands::market_quotes::SearchAssetsCommand, infra::claim::Claims},
    ports::inbound::rest::{
        error::V2HttpError,
        types::{
            markets_v2::assets::search::{SearchAssetsV2Response, SearchPath, SearchQuery},
            v2_common::V2ErrorResponse,
        },
    },
};

/// Searches the assets listed by one provider, reporting failures in the outcome.
#[utoipa::path(
    get,
    path = "/assets/{provider}/search",
    operation_id = "search_assets_v2",
    params(
        ("Authorization" = String, Header, description = "Bearer JWT token"),
        SearchPath,
        SearchQuery
    ),
    responses(
        (status = 200, description = "Search answer; `outcome` tells usable, empty, providerError or malformed", body = SearchAssetsV2Response),
        (status = 400, description = "Unsupported provider or invalid query", body = V2ErrorResponse),
        (status = 429, description = "The provider rate limit is exhausted", body = V2ErrorResponse),
        (status = 500, description = "Catalog failure", body = V2ErrorResponse)
    )
)]
pub async fn search_assets(
    State(ctx): State<AppContext>,
    _claims: Claims,
    Path(path): Path<SearchPath>,
    Query(query): Query<SearchQuery>,
) -> Result<Response, V2HttpError> {
    let command = SearchAssetsCommand::try_new(&path.provider, query.query.as_deref())?;
    let search = ctx.services.asset_search.search(command).await?;

    Ok(Json(SearchAssetsV2Response::from(search)).into_response())
}
