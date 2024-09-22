use jsonschema::Validator;

use crate::{
    app::domain::entity::{Asset, AssetId},
    error::{DcaError, Result},
    ports::outbound::repository::market_data::MarketDataRepository,
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
            return Err(DcaError::BadRequest(format!(
                "Unknown base asset: {}",
                base
            )));
        }

        let quote = AssetId::from(quote);
        let quote_asset = repo.find_asset(&quote).await?;
        if quote_asset.is_none() {
            return Err(DcaError::BadRequest(format!(
                "Unknown quote asset: {}",
                quote
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
    pub fn try_new(payload: serde_json::Value, validator: &Validator) -> Result<Self> {
        if !validator.is_valid(&payload) {
            return Err(DcaError::BadRequest(
                "Input portfolio does not match portfolio schema requirements".to_string(),
            ));
        }

        Ok(Self { pfolio: payload })
    }
}
