//! REST handlers: one module per endpoint, nested like the URL path.
//!
//! Wire types of each endpoint live in the mirrored module under `rest/types/`.

pub mod assets;
mod cache;
pub mod import;
pub mod lookup_assets_v2;
pub mod price;
pub mod root;
pub mod sync_portfolios;
pub mod sync_portfolios_v2;
