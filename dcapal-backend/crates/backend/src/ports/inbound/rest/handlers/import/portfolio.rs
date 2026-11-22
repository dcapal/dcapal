//! `POST /import/portfolio`: validates and stores a shared portfolio payload.

use std::sync::LazyLock;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;
use metrics::counter;

use crate::{
    AppContext, app::services::command::ImportPortfolioCmd, error::Result, infra::stats,
    ports::inbound::rest::types::import::portfolio::ImportPortfolioResponse,
};

static PORTFOLIO_SCHEMA_STR: &str =
    include_str!("../../../../../../../../docs/schema/portfolio/v1/schema.json");

/// The v1 portfolio import JSON schema as a document.
pub(crate) static PORTFOLIO_JSON_SCHEMA: LazyLock<serde_json::Value> =
    LazyLock::new(|| serde_json::from_str(PORTFOLIO_SCHEMA_STR).unwrap());

static PORTFOLIO_SCHEMA_VALIDATOR: LazyLock<jsonschema::Validator> =
    LazyLock::new(|| jsonschema::draft7::new(&PORTFOLIO_JSON_SCHEMA).unwrap());

#[utoipa::path(
    post,
    path = "/import/portfolio",
    request_body(
        content = serde_json::Value,
        content_type = "application/json",
        description = "Portfolio JSON object validated against dcapal-backend/docs/schema/portfolio/v1/schema.json"
    ),
    responses(
        (status = 201, description = "Imported portfolio metadata", body = ImportPortfolioResponse),
        (status = 400, description = "Input portfolio does not match schema requirements")
    )
)]
/// Validates and stores a shared portfolio payload.
pub async fn import_portfolio(
    State(ctx): State<AppContext>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Response> {
    let repo = &ctx.repos.imported;
    let stats_repo = &ctx.repos.stats;

    let cmd = ImportPortfolioCmd::try_new(payload, &PORTFOLIO_SCHEMA_VALIDATOR)?;
    let imported = repo.store_portfolio(&cmd.pfolio).await?;

    counter!(stats::IMPORTED_PORTFOLIOS_TOTAL).increment(1);
    let _ = stats_repo.increase_imported_portfolio_count().await;

    let response = (
        StatusCode::CREATED,
        Json(ImportPortfolioResponse::from(imported)),
    );

    Ok(response.into_response())
}
