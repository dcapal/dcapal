use std::sync::Arc;

use async_trait::async_trait;

use super::asset_provider::{AssetProvider, AssetProviderError, ProviderAsset};
use crate::{
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::Asset,
        portfolio::{AssetClass, Provider},
    },
    ports::outbound::repository::market_data::MarketDataRepository,
};

/// Resolves Kraken assets from the crypto assets kept by market discovery.
///
/// The provider reads the provider-confirmed Kraken crypto rows of `assets_data`
/// (client-supplied rows are ignored, so a client cannot vouch for itself).
/// Kraken has no display name for cryptocurrencies, so the symbol as submitted
/// serves as name and as trading currency (a placeholder name). Without any
/// confirmed Kraken crypto row (cold start) an unknown symbol is reported as a
/// provider outage rather than as unknown. Fiat currencies are not Kraken
/// assets: they belong to Yahoo. This path needs PostgreSQL and is covered by
/// the repository integration tests.
pub struct KrakenAssetProvider {
    catalog: Arc<MarketDataRepository>,
}

impl KrakenAssetProvider {
    /// Creates a provider reading the supplied asset catalog.
    pub fn new(catalog: Arc<MarketDataRepository>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl AssetProvider for KrakenAssetProvider {
    async fn lookup(&self, asset: &AssetRef) -> Result<Option<ProviderAsset>, AssetProviderError> {
        let id = asset.symbol().to_ascii_lowercase();
        let found = self
            .catalog
            .find_asset(&id)
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;

        if let Some(Asset::Crypto(_)) = found {
            // The catalog has no display name better than the symbol, so no
            // entry may rename an asset a client already named.
            return Ok(Some(ProviderAsset {
                asset: resolved_crypto(asset),
                name_authoritative: false,
            }));
        }
        if found.is_some() {
            // A fiat currency is a Yahoo asset, not a Kraken one.
            return Ok(None);
        }

        // Without confirmed Kraken crypto rows (cold start) an unknown symbol
        // cannot be told from a catalog that was never loaded.
        let has_catalog = self
            .catalog
            .has_crypto_assets()
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;
        if !has_catalog {
            return Err(AssetProviderError::Unavailable(
                "asset catalog is empty".into(),
            ));
        }
        Ok(None)
    }
}

/// Builds the shared asset metadata of a Kraken crypto asset: the symbol is both
/// name and trading currency.
fn resolved_crypto(reference: &AssetRef) -> ResolvedAsset {
    ResolvedAsset {
        provider: Provider::Kraken,
        symbol: reference.symbol().to_string(),
        name: reference.symbol().to_string(),
        currency: reference.symbol().to_string(),
        asset_class: AssetClass::Crypto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_crypto_catalog_entry() {
        // GIVEN a Kraken crypto reference, WHEN it is mapped to shared metadata,
        // THEN it resolves to a crypto asset named and quoted by its symbol.
        let reference = AssetRef::new(Provider::Kraken, "btc");
        let asset = resolved_crypto(&reference);

        assert_eq!(asset.name, "BTC");
        assert_eq!(asset.currency, "BTC");
        assert_eq!(asset.asset_class, AssetClass::Crypto);
    }
}
