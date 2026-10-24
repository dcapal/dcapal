//! Outbound port resolving asset metadata from the provider that lists it.

use std::sync::Arc;

use async_trait::async_trait;

use crate::app::domain::{
    asset::{AssetRef, ResolvedAsset},
    portfolio::Provider,
};

/// Errors raised while resolving an asset from its provider.
#[derive(Debug, thiserror::Error)]
pub enum AssetProviderError {
    /// The provider could not be reached or answered with a failure status.
    #[error("asset provider is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The provider answered with data that cannot be turned into an asset.
    #[error("asset provider returned malformed data: {0}")]
    Malformed(String),
    /// The reference names a provider this adapter does not serve.
    #[error("asset provider does not serve '{0}'")]
    UnsupportedProvider(String),
}

/// Looks up the metadata of a single asset at its provider.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetProvider: Send + Sync {
    /// Resolves the referenced asset.
    ///
    /// `Ok(None)` means the provider does not know the asset; `Err` means the
    /// provider is unavailable or answered with unusable data.
    async fn lookup(&self, asset: &AssetRef) -> Result<Option<ResolvedAsset>, AssetProviderError>;
}

/// Dispatches each lookup to the adapter serving the reference's provider.
pub struct ProviderRoutedAssetProvider {
    yahoo: Arc<dyn AssetProvider>,
    kraken: Arc<dyn AssetProvider>,
}

impl ProviderRoutedAssetProvider {
    /// Creates a router over the Yahoo and Kraken adapters.
    pub fn new(yahoo: Arc<dyn AssetProvider>, kraken: Arc<dyn AssetProvider>) -> Self {
        Self { yahoo, kraken }
    }
}

#[async_trait]
impl AssetProvider for ProviderRoutedAssetProvider {
    async fn lookup(&self, asset: &AssetRef) -> Result<Option<ResolvedAsset>, AssetProviderError> {
        match &asset.provider {
            Provider::YF => self.yahoo.lookup(asset).await,
            Provider::Kraken => self.kraken.lookup(asset).await,
            Provider::Unsupported(value) => {
                Err(AssetProviderError::UnsupportedProvider(value.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::domain::portfolio::AssetClass;

    fn resolved(provider: Provider, symbol: &str) -> ResolvedAsset {
        ResolvedAsset {
            provider,
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            currency: "USD".to_string(),
            asset_class: AssetClass::Other,
        }
    }

    #[tokio::test]
    async fn routes_lookups_by_provider() {
        // GIVEN a router over a Yahoo and a Kraken adapter,
        // WHEN a Kraken reference and a Yahoo reference are looked up,
        // THEN each reaches only its own adapter.
        let mut yahoo = MockAssetProvider::new();
        yahoo
            .expect_lookup()
            .times(1)
            .returning(|a| Ok(Some(resolved(a.provider.clone(), &a.symbol))));
        let mut kraken = MockAssetProvider::new();
        kraken.expect_lookup().times(1).returning(|_| Ok(None));
        let router = ProviderRoutedAssetProvider::new(Arc::new(yahoo), Arc::new(kraken));

        let yf = router.lookup(&AssetRef::new(Provider::YF, "aapl")).await;
        let kr = router.lookup(&AssetRef::new(Provider::Kraken, "btc")).await;

        assert_eq!(yf.unwrap(), Some(resolved(Provider::YF, "AAPL")));
        assert_eq!(kr.unwrap(), None);
    }

    #[tokio::test]
    async fn rejects_unsupported_providers() {
        // GIVEN a router, WHEN a reference with an unsupported provider is looked up,
        // THEN the lookup fails without reaching any adapter.
        let router = ProviderRoutedAssetProvider::new(
            Arc::new(MockAssetProvider::new()),
            Arc::new(MockAssetProvider::new()),
        );

        let result = router
            .lookup(&AssetRef::new(Provider::Unsupported("ibkr".into()), "X"))
            .await;

        assert!(matches!(
            result,
            Err(AssetProviderError::UnsupportedProvider(_))
        ));
    }
}
