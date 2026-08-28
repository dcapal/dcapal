use jsonschema::Validator;
use serde_json::Value;

use crate::{
    app::domain::entity::{Asset, AssetId},
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
    ports::outbound::repository::postgres::types::{AssetClass, Provider},
};

pub struct ConversionRateQuery {
    pub base: Asset,
    pub quote: Asset,
}

impl ConversionRateQuery {
    pub async fn try_new(base: &str, quote: &str, repo: &MarketDataRepository) -> Result<Self> {
        let base = AssetId::from(base);
        let base_asset = repo.find_asset(&base).await?;
        if base_asset.is_none() {
            return Err(DcaError::BadRequest(format!("Unknown base asset: {base}")));
        }

        let quote = AssetId::from(quote);
        let quote_asset = repo.find_asset(&quote).await?;
        if quote_asset.is_none() {
            return Err(DcaError::BadRequest(format!(
                "Unknown quote asset: {quote}"
            )));
        }

        Ok(ConversionRateQuery {
            base: base_asset.unwrap(),
            quote: quote_asset.unwrap(),
        })
    }
}

pub struct ImportPortfolioCmd {
    pub pfolio: serde_json::Value,
}

impl ImportPortfolioCmd {
    pub fn try_new(payload: Value, validator: &Validator) -> Result<Self> {
        let payload = normalize_import_payload(payload);
        if !validator.is_valid(&payload) {
            return Err(DcaError::BadRequest(
                "Input portfolio does not match portfolio schema requirements".to_string(),
            ));
        }

        Ok(Self { pfolio: payload })
    }
}

fn normalize_import_payload(mut payload: Value) -> Value {
    if let Some(assets) = payload
        .as_object_mut()
        .and_then(|portfolio| portfolio.get_mut("assets"))
        .and_then(Value::as_array_mut)
    {
        assets.retain_mut(|asset| {
            let Some(asset) = asset.as_object_mut() else {
                return true;
            };

            let provider = asset
                .get("provider")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(provider) = provider {
                let Some(provider) = Provider::from_legacy(&provider) else {
                    return false;
                };
                asset.insert(
                    "provider".to_string(),
                    Value::String(provider.as_legacy_name().to_string()),
                );
            }

            if let Some(asset_class) = asset.get("aclass").and_then(Value::as_str) {
                asset.insert(
                    "aclass".to_string(),
                    Value::String(
                        AssetClass::from_legacy(asset_class)
                            .as_legacy_name()
                            .to_string(),
                    ),
                );
            }

            true
        });
    }

    payload
}

#[cfg(test)]
mod tests {
    use jsonschema::draft7;
    use serde_json::json;

    use super::*;

    fn validator() -> Validator {
        draft7::new(&json!({
            "type": "object",
            "required": ["quoteCcy", "assets"],
            "properties": {
                "quoteCcy": {"type": "string"},
                "assets": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["symbol", "name", "aclass", "provider"],
                        "properties": {
                            "symbol": {"type": "string"},
                            "name": {"type": "string"},
                            "aclass": {"enum": ["EQUITY", "CURRENCY", "OTHER"]},
                            "provider": {"enum": ["DCAPal", "YF"]}
                        }
                    }
                }
            }
        }))
        .unwrap()
    }

    fn asset(provider: &str, asset_class: &str) -> Value {
        json!({
            "symbol": "ASSET",
            "name": "Asset",
            "aclass": asset_class,
            "provider": provider
        })
    }

    #[test]
    fn import_normalization_maps_aliases_and_filters_unsupported_assets() {
        // GIVEN a structurally valid import with mixed-case aliases and an unsupported provider,
        // WHEN the import command is constructed, THEN aliases are canonicalized and the asset is dropped.
        let payload = json!({
            "quoteCcy": "eur",
            "assets": [asset("kRaKeN", "cash"), asset("IBKR", "unknown")]
        });

        let command = ImportPortfolioCmd::try_new(payload, &validator()).unwrap();

        assert_eq!(command.pfolio["assets"].as_array().unwrap().len(), 1);
        assert_eq!(command.pfolio["assets"][0]["provider"], "DCAPal");
        assert_eq!(command.pfolio["assets"][0]["aclass"], "CURRENCY");
    }

    #[test]
    fn import_normalization_keeps_empty_portfolios() {
        // GIVEN an import containing only a structurally valid unsupported asset, WHEN it is normalized,
        // THEN the portfolio remains valid with an empty asset list.
        let payload = json!({
            "quoteCcy": "eur",
            "assets": [asset("IBKR", "equity")]
        });

        let command = ImportPortfolioCmd::try_new(payload, &validator()).unwrap();

        assert_eq!(command.pfolio["assets"], json!([]));
    }

    #[test]
    fn import_normalization_filters_malformed_unsupported_assets() {
        // GIVEN an unsupported-provider asset with malformed non-provider fields, WHEN the import is normalized,
        // THEN the unsupported asset is omitted before schema validation.
        let payload = json!({
            "quoteCcy": "eur",
            "assets": [{
                "symbol": 42,
                "name": null,
                "aclass": "equity",
                "provider": "IBKR"
            }]
        });

        let command = ImportPortfolioCmd::try_new(payload, &validator()).unwrap();

        assert_eq!(command.pfolio["assets"], json!([]));
    }

    #[test]
    fn import_retained_structural_invalid_data_is_rejected_by_schema() {
        // GIVEN a retained supported-provider asset with a malformed field, WHEN the import is validated,
        // THEN schema validation rejects the malformed asset.
        let payload = json!({
            "quoteCcy": "eur",
            "assets": [{
                "symbol": 42,
                "name": "Asset",
                "aclass": "equity",
                "provider": "YF"
            }]
        });

        assert!(ImportPortfolioCmd::try_new(payload, &validator()).is_err());
    }

    #[test]
    fn import_malformed_root_is_rejected_by_schema() {
        // GIVEN an import whose assets field has the wrong structure, WHEN the import is validated,
        // THEN schema validation rejects the root payload.
        let payload = json!({
            "quoteCcy": "eur",
            "assets": "not-an-array"
        });

        assert!(ImportPortfolioCmd::try_new(payload, &validator()).is_err());
    }
}
