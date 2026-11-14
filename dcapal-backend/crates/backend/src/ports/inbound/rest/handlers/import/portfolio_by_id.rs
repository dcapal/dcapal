//! `GET /import/portfolio/{id}`: reads a stored shared portfolio.

use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use hyper::StatusCode;

use crate::{
    AppContext, error::Result,
    ports::inbound::rest::types::import::portfolio_by_id::GetImportedPortfolioPath,
};

#[utoipa::path(
    get,
    path = "/import/portfolio/{id}",
    params(GetImportedPortfolioPath),
    responses(
        (status = 200, description = "Imported portfolio payload", body = serde_json::Value),
        (status = 404, description = "Portfolio not found or expired")
    )
)]
/// Returns a stored shared portfolio or a not-found response when it expired.
pub async fn get_imported_portfolio(
    Path(path): Path<GetImportedPortfolioPath>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;

    match repo.find_portfolio(&path.id).await? {
        Some(portfolio) => Ok(Json(portfolio).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
