//! Custom axum extractors of the REST boundary.

use axum::{extract::FromRequestParts, http::request::Parts};

use super::{
    error::V2HttpError,
    types::sync_portfolio_v2::{V2FieldError, V2ValidationError},
};

/// Query parameter that carries one asset reference per occurrence.
const ASSET_PARAM: &str = "asset";

/// The raw values of the repeated `asset` query parameter, in request order.
///
/// Other query keys are ignored. Values are only percent-decoded here; the
/// `<provider>:<symbol>` syntax, the maximum count and de-duplication belong
/// to `LookupAssetsCommand::try_new`. A value that is not valid UTF-8 once
/// decoded is rejected with the v2 `validation_error` body rather than axum's
/// plain-text rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupAssetRefs(pub Vec<String>);

impl<S> FromRequestParts<S> for LookupAssetRefs
where
    S: Send + Sync,
{
    type Rejection = V2HttpError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let mut assets = Vec::new();
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key != ASSET_PARAM {
                continue;
            }
            // Lossy decoding substitutes U+FFFD for invalid UTF-8 sequences.
            if value.contains(char::REPLACEMENT_CHARACTER) {
                return Err(V2HttpError::Validation(V2ValidationError {
                    errors: vec![V2FieldError {
                        field: format!("{ASSET_PARAM}[{}]", assets.len()),
                        message: "must be valid percent-encoded UTF-8".to_string(),
                    }],
                }));
            }
            assets.push(value.into_owned());
        }
        Ok(Self(assets))
    }
}

#[cfg(test)]
mod tests {
    use axum::{http::Request, response::IntoResponse};

    use super::*;

    /// Runs the extractor on a request for `uri`.
    async fn extract(uri: &str) -> Result<LookupAssetRefs, V2HttpError> {
        let (mut parts, ()) = Request::builder().uri(uri).body(()).unwrap().into_parts();
        LookupAssetRefs::from_request_parts(&mut parts, &()).await
    }

    fn refs(values: &[&str]) -> LookupAssetRefs {
        LookupAssetRefs(values.iter().map(|v| v.to_string()).collect())
    }

    #[tokio::test]
    async fn collects_repeated_asset_keys_in_order() {
        // GIVEN a query repeating `asset` and carrying an unrelated key,
        // WHEN the extractor runs,
        // THEN only the `asset` values are returned, in request order.
        let got = extract("/assets/lookup?asset=yf:AAPL&x=1&asset=kraken:XBT&asset=yf:AAPL")
            .await
            .unwrap();

        assert_eq!(got, refs(&["yf:AAPL", "kraken:XBT", "yf:AAPL"]));
    }

    #[tokio::test]
    async fn missing_or_empty_query_yields_no_assets() {
        // GIVEN requests without a query string or without any `asset` key,
        // WHEN the extractor runs,
        // THEN it succeeds with no values so the command reports "required".
        for uri in [
            "/assets/lookup",
            "/assets/lookup?",
            "/assets/lookup?other=1",
        ] {
            assert_eq!(extract(uri).await.unwrap(), refs(&[]), "{uri}");
        }
    }

    #[tokio::test]
    async fn keeps_empty_values_and_decodes_percent_and_plus() {
        // GIVEN an empty `asset=` and percent-encoded or plus-encoded values,
        // WHEN the extractor runs,
        // THEN empty values are kept and the others are decoded.
        let got = extract("/assets/lookup?asset=&asset=yf%3AA%2FB&asset=yf:A+B")
            .await
            .unwrap();

        assert_eq!(got, refs(&["", "yf:A/B", "yf:A B"]));
    }

    #[tokio::test]
    async fn invalid_utf8_is_a_v2_validation_error() {
        // GIVEN an `asset` value whose percent-encoding is not valid UTF-8,
        // WHEN the extractor runs,
        // THEN it is rejected with a 400 in the v2 `validation_error` body.
        let rejection = extract("/assets/lookup?asset=yf:OK&asset=yf:%FF")
            .await
            .unwrap_err();

        let response = rejection.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], "validation_error");
        assert_eq!(body["errors"][0]["field"], "asset[1]");
    }
}
