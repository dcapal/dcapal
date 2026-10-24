use std::sync::Arc;

use async_trait::async_trait;

use super::asset_provider::{AssetProvider, AssetProviderError};
use crate::{
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::Asset,
        portfolio::{AssetClass, Provider},
    },
    ports::outbound::repository::market_data::MarketDataRepository,
};

/// Resolves Kraken assets from the Redis asset catalog kept by market discovery.
///
/// The catalog stores no display name for cryptocurrencies, so the symbol as
/// submitted serves as name and as trading currency (a placeholder name). An
/// empty catalog (cold start) is reported as a provider outage rather than as
/// an unknown symbol; this path needs Redis and is not unit tested. Fiat catalog entries carry
/// their English name and resolve to cash in their own currency.
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
    async fn lookup(&self, asset: &AssetRef) -> Result<Option<ResolvedAsset>, AssetProviderError> {
        let id = asset.symbol.to_ascii_lowercase();
        let found = self
            .catalog
            .find_asset(&id)
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;

        if let Some(catalog_asset) = found {
            return Ok(Some(resolved_from_catalog(asset, &catalog_asset)));
        }

        // A catalog that was never loaded (cold start) cannot tell an unknown
        // symbol from a missing catalog, so it is reported as unavailable.
        if self.catalog_is_empty().await? {
            return Err(AssetProviderError::Unavailable(
                "asset catalog is empty".into(),
            ));
        }
        Ok(None)
    }
}

impl KrakenAssetProvider {
    /// Tells whether the catalog holds neither crypto nor fiat assets.
    async fn catalog_is_empty(&self) -> Result<bool, AssetProviderError> {
        let has_assets = self
            .catalog
            .has_assets()
            .await
            .map_err(|e| AssetProviderError::Unavailable(Box::new(e)))?;
        Ok(!has_assets)
    }
}

/// Builds the shared asset metadata for a catalog entry of the referenced symbol.
fn resolved_from_catalog(reference: &AssetRef, catalog_asset: &Asset) -> ResolvedAsset {
    let (name, asset_class) = match catalog_asset {
        Asset::Crypto(_) => (reference.symbol.clone(), AssetClass::Crypto),
        // The catalog stores the fiat display name in the `symbol` field.
        Asset::Fiat(fiat) => (fiat.symbol.clone(), AssetClass::Cash),
    };

    ResolvedAsset {
        provider: Provider::Kraken,
        symbol: reference.symbol.clone(),
        name,
        currency: reference.symbol.clone(),
        asset_class,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::domain::entity::{Crypto, Fiat};

    #[test]
    fn maps_crypto_catalog_entry() {
        // GIVEN a crypto catalog entry, WHEN it is mapped for its reference,
        // THEN it resolves to a crypto asset named and quoted by its symbol.
        let reference = AssetRef::new(Provider::Kraken, "btc");
        let asset = resolved_from_catalog(
            &reference,
            &Asset::Crypto(Crypto::new_with_id("btc".into())),
        );

        assert_eq!(asset.name, "BTC");
        assert_eq!(asset.currency, "BTC");
        assert_eq!(asset.asset_class, AssetClass::Crypto);
    }

    #[test]
    fn maps_fiat_catalog_entry() {
        // GIVEN a fiat catalog entry, WHEN it is mapped for its reference,
        // THEN it resolves to cash carrying the fiat display name.
        let reference = AssetRef::new(Provider::Kraken, "eur");
        let asset = resolved_from_catalog(
            &reference,
            &Asset::Fiat(Fiat::new("eur".into(), "Euro".into())),
        );

        assert_eq!(asset.name, "Euro");
        assert_eq!(asset.currency, "EUR");
        assert_eq!(asset.asset_class, AssetClass::Cash);
    }
}
