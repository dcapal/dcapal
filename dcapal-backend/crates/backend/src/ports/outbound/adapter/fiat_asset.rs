use std::sync::Arc;

use async_trait::async_trait;

use super::asset_provider::{AssetProvider, AssetProviderError, ProviderAsset};
use crate::app::domain::{
    asset::{AssetRef, ResolvedAsset},
    fiat::fiat_name,
    portfolio::{AssetClass, Provider},
};

/// Resolves the fiat currencies of the catalog without calling Yahoo, and
/// delegates every other symbol to the wrapped provider.
///
/// Fiat currencies are Yahoo cash assets named in English and quoted in their
/// own currency; a bare currency code is not a Yahoo ticker.
pub struct CatalogFiatAssetProvider {
    inner: Arc<dyn AssetProvider>,
}

impl CatalogFiatAssetProvider {
    /// Wraps the provider that resolves every non-catalog symbol.
    pub fn new(inner: Arc<dyn AssetProvider>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl AssetProvider for CatalogFiatAssetProvider {
    async fn lookup(&self, asset: &AssetRef) -> Result<Option<ProviderAsset>, AssetProviderError> {
        match fiat_name(asset.symbol()) {
            Some(name) => Ok(Some(ProviderAsset {
                asset: ResolvedAsset {
                    provider: Provider::YF,
                    symbol: asset.symbol().to_string(),
                    name: name.to_string(),
                    currency: asset.symbol().to_string(),
                    asset_class: AssetClass::Cash,
                },
                name_authoritative: true,
            })),
            None => self.inner.lookup(asset).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::outbound::adapter::asset_provider::MockAssetProvider;

    #[tokio::test]
    async fn catalog_fiat_resolves_without_calling_the_provider() {
        // GIVEN a wrapped provider that must not be called,
        // WHEN a catalog fiat symbol is looked up,
        // THEN it resolves to English-named cash in its own currency.
        let provider = CatalogFiatAssetProvider::new(Arc::new(MockAssetProvider::new()));

        let found = provider
            .lookup(&AssetRef::new(Provider::YF, "eur"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.asset.name, "Euro");
        assert_eq!(found.asset.currency, "EUR");
        assert_eq!(found.asset.asset_class, AssetClass::Cash);
        assert!(found.name_authoritative);
    }

    #[tokio::test]
    async fn other_symbols_are_delegated() {
        // GIVEN a wrapped provider that knows nothing,
        // WHEN a non-catalog symbol is looked up,
        // THEN the wrapped provider answers.
        let mut inner = MockAssetProvider::new();
        inner.expect_lookup().times(1).returning(|_| Ok(None));
        let provider = CatalogFiatAssetProvider::new(Arc::new(inner));

        let found = provider
            .lookup(&AssetRef::new(Provider::YF, "AAPL"))
            .await
            .unwrap();

        assert_eq!(found, None);
    }
}
