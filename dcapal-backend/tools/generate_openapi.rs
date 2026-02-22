use std::{env, path::PathBuf};

use anyhow::Result;
use dcapal_backend::ports::inbound::rest::{build_openapi_router, openapi::write_openapi_to_file};

fn main() -> Result<()> {
    let output = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_output_path);

    let (_, openapi) = build_openapi_router();
    write_openapi_to_file(&openapi, &output)?;

    println!("OpenAPI spec exported to {}", output.display());
    Ok(())
}

fn default_output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/openapi.json")
}
