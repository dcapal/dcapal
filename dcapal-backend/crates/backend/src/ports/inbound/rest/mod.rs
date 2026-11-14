//! The [`rest`](self) module implements the REST API of the system.
//!
//! Handlers live under [`handlers`], one module per endpoint nested like the URL path.
//! Their wire types are mirrored under [`types`]. Modules shared by all endpoints
//! (errors, extractors, OpenAPI export) stay directly in this module.

use axum::Router;
use utoipa::openapi::{Info, OpenApi, Paths};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::AppContext;

pub mod error;
pub mod extract;
pub mod handlers;
pub mod openapi;
pub mod types;

/// Builds the REST router and the OpenAPI document from the same route set.
pub fn build_openapi_router() -> (Router<AppContext>, OpenApi) {
    OpenApiRouter::with_openapi(base_openapi())
        .routes(routes!(handlers::root::root))
        .routes(routes!(handlers::assets::fiat::get_assets_fiat))
        .routes(routes!(handlers::assets::crypto::get_assets_crypto))
        .routes(routes!(handlers::assets::search::get_assets_data))
        .routes(routes!(handlers::assets::chart::get_assets_chart))
        .routes(routes!(handlers::price::get_price))
        .routes(routes!(handlers::import::portfolio::import_portfolio))
        .routes(routes!(
            handlers::import::portfolio_by_id::get_imported_portfolio
        ))
        .nest("/v1", build_v1_openapi_router())
        .nest("/v2", build_v2_openapi_router())
        .nest("/markets/v2", build_markets_v2_openapi_router())
        .split_for_parts()
}

fn build_v1_openapi_router() -> OpenApiRouter<AppContext> {
    OpenApiRouter::new().routes(routes!(handlers::sync_portfolios::sync_portfolios))
}

fn build_v2_openapi_router() -> OpenApiRouter<AppContext> {
    OpenApiRouter::new().routes(routes!(handlers::sync_portfolios_v2::sync_portfolios))
}

fn build_markets_v2_openapi_router() -> OpenApiRouter<AppContext> {
    OpenApiRouter::new().routes(routes!(handlers::lookup_assets_v2::lookup_assets))
}

fn base_openapi() -> OpenApi {
    OpenApi::new(
        Info::new("DCA-Pal APIs", env!("CARGO_PKG_VERSION")),
        Paths::new(),
    )
}
