use crate::{
    error::{DcaError, Result},
    repository::MarketDataRepository,
};

use super::entity::{Asset, AssetId};

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
