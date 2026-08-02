use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value};
use utoipa::openapi::OpenApi;

use crate::AppContext;

/// Converts the generated OpenAPI document to JSON and applies repository schema fixes.
pub fn openapi_value(openapi: &OpenApi) -> Result<Value> {
    let mut value = serde_json::to_value(openapi)?;
    inline_import_portfolio_request_schema(&mut value);
    Ok(value)
}

/// Serves the checked-in OpenAPI representation through the REST application.
pub async fn get_openapi_json(State(ctx): State<AppContext>) -> Response {
    Json(ctx.openapi_value.clone()).into_response()
}

/// Serializes an OpenAPI document with a trailing newline for stable diffs.
pub fn openapi_json(openapi: &OpenApi) -> Result<String> {
    let mut json = serde_json::to_string_pretty(&openapi_value(openapi)?)?;
    json.push('\n');
    Ok(json)
}

/// Writes the OpenAPI document through a temporary file to avoid partial output.
pub fn write_openapi_to_file(openapi: &OpenApi, output: &Path) -> Result<()> {
    let json = openapi_json(openapi)?;
    write_file_atomically(output, &json)
}

fn write_file_atomically(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("failed creating output directory {}", parent.display()))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("openapi.json");
    let tmp_path: PathBuf = parent.join(format!("{file_name}.tmp"));

    fs::write(&tmp_path, content)
        .with_context(|| format!("failed writing temporary file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed moving temporary file {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

fn inline_import_portfolio_request_schema(openapi: &mut Value) {
    // Some OpenAPI consumers do not resolve local `$defs` references in an
    // inline request body, so publish the portfolio schema as a self-contained object.
    let schema_path =
        "/paths/~1import~1portfolio/post/requestBody/content/application~1json/schema";
    let request_schema = openapi
        .pointer_mut(schema_path)
        .unwrap_or_else(|| panic!("missing OpenAPI schema path: {schema_path}"));

    let mut schema = super::PORTFOLIO_JSON_SCHEMA.clone();
    let definitions = schema
        .get("$defs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if let Some(schema_object) = schema.as_object_mut() {
        schema_object.remove("$defs");
    }

    inline_local_schema_refs(&mut schema, &definitions, &mut HashSet::new());
    *request_schema = schema;
}

fn inline_local_schema_refs(
    value: &mut Value,
    definitions: &Map<String, Value>,
    active_definitions: &mut HashSet<String>,
) {
    // Track the current reference chain so a recursive schema cannot recurse forever.
    let local_reference = value
        .as_object()
        .and_then(|object| object.get("$ref"))
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .map(str::to_owned);

    if let Some(name) = local_reference {
        if let Some(definition) = definitions.get(&name)
            && active_definitions.insert(name.clone())
        {
            let mut replacement = definition.clone();
            inline_local_schema_refs(&mut replacement, definitions, active_definitions);
            active_definitions.remove(&name);
            *value = replacement;
        }
        return;
    }

    match value {
        Value::Array(items) => {
            for item in items {
                inline_local_schema_refs(item, definitions, active_definitions);
            }
        }
        Value::Object(object) => {
            for child in object.values_mut() {
                inline_local_schema_refs(child, definitions, active_definitions);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use utoipa::openapi::RefOr;

    use super::*;

    #[test]
    fn includes_expected_paths() {
        let (_, openapi) = super::super::build_openapi_router();
        let paths = &openapi.paths.paths;

        for expected in [
            "/",
            "/assets/fiat",
            "/assets/crypto",
            "/assets/search",
            "/assets/chart/{symbol}",
            "/price/{asset}",
            "/import/portfolio",
            "/import/portfolio/{id}",
            "/v1/sync/portfolios",
        ] {
            assert!(paths.contains_key(expected), "missing path {expected}");
        }
    }

    #[test]
    fn chart_query_params_are_camel_case() {
        let (_, openapi) = super::super::build_openapi_router();
        let op = openapi
            .paths
            .paths
            .get("/assets/chart/{symbol}")
            .and_then(|path_item| path_item.get.as_ref())
            .expect("missing GET chart operation");

        let param_names: Vec<String> = op
            .parameters
            .as_ref()
            .expect("missing chart params")
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect();

        assert!(param_names.contains(&"startPeriod".to_string()));
        assert!(param_names.contains(&"endPeriod".to_string()));
        assert!(!param_names.contains(&"start_period".to_string()));
        assert!(!param_names.contains(&"end_period".to_string()));
    }

    #[test]
    fn serialization_is_deterministic_for_same_document() {
        let (_, openapi) = super::super::build_openapi_router();

        let first = openapi_json(&openapi).expect("serialize openapi");
        let second = openapi_json(&openapi).expect("serialize openapi");

        assert_eq!(first, second);
    }

    #[test]
    fn search_response_schema_is_typed() {
        let (_, openapi) = super::super::build_openapi_router();
        let operation = openapi
            .paths
            .paths
            .get("/assets/search")
            .and_then(|path_item| path_item.get.as_ref())
            .expect("missing GET /assets/search operation");

        let schema_ref = response_schema_ref(
            operation
                .responses
                .responses
                .get("200")
                .expect("missing 200 response"),
        )
        .expect("missing search response schema ref");

        assert_eq!(schema_ref, "#/components/schemas/YahooSearchResponse");
    }

    #[test]
    fn chart_response_schema_is_typed() {
        let (_, openapi) = super::super::build_openapi_router();
        let operation = openapi
            .paths
            .paths
            .get("/assets/chart/{symbol}")
            .and_then(|path_item| path_item.get.as_ref())
            .expect("missing GET /assets/chart/{symbol} operation");

        let schema_ref = response_schema_ref(
            operation
                .responses
                .responses
                .get("200")
                .expect("missing 200 response"),
        )
        .expect("missing chart response schema ref");

        assert_eq!(schema_ref, "#/components/schemas/YahooChartResponse");
    }

    #[test]
    fn import_portfolio_request_schema_is_inlined_from_json_schema() {
        let (_, openapi) = super::super::build_openapi_router();
        let openapi = openapi_value(&openapi).expect("serialize openapi");

        let schema = openapi
            .pointer("/paths/~1import~1portfolio/post/requestBody/content/application~1json/schema")
            .expect("missing POST /import/portfolio request schema");

        assert!(schema.get("$ref").is_none(), "schema should be inlined");
        assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
        assert!(schema.get("properties").is_some());

        assert!(schema.get("$defs").is_none());
        assert_no_local_schema_refs(schema);
    }

    fn assert_no_local_schema_refs(value: &Value) {
        match value {
            Value::Array(items) => {
                for item in items {
                    assert_no_local_schema_refs(item);
                }
            }
            Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                    assert!(
                        !reference.starts_with("#/$defs/"),
                        "unexpected local schema reference: {reference}"
                    );
                }
                for child in object.values() {
                    assert_no_local_schema_refs(child);
                }
            }
            _ => {}
        }
    }

    fn response_schema_ref(response: &RefOr<utoipa::openapi::Response>) -> Option<String> {
        let response = match response {
            RefOr::Ref(_) => return None,
            RefOr::T(r) => r,
        };

        let json_content = response.content.get("application/json")?;
        let schema = json_content.schema.as_ref()?;

        match schema {
            RefOr::Ref(r) => Some(r.ref_location.clone()),
            RefOr::T(_) => None,
        }
    }
}
