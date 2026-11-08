//! Provider-bound asset references and their resolved shared metadata.

use crate::app::domain::portfolio::{AssetClass, Provider};

/// Reference to an asset by the provider that lists it and its provider symbol.
///
/// The symbol is stored trimmed and upper-cased, matching the canonical form
/// persisted for shared asset metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetRef {
    provider: Provider,
    symbol: String,
}

impl AssetRef {
    /// Creates a reference, normalizing the symbol to its canonical form.
    pub fn new(provider: Provider, symbol: &str) -> Self {
        Self {
            provider,
            symbol: Self::canonical_symbol(symbol),
        }
    }

    /// Returns the provider listing the asset.
    pub const fn provider(&self) -> Provider {
        self.provider
    }

    /// Returns the canonical provider symbol.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the canonical form of a provider symbol: trimmed and upper-cased.
    fn canonical_symbol(symbol: &str) -> String {
        symbol.trim().to_ascii_uppercase()
    }
}

/// Shared, provider-sourced metadata of an asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    pub provider: Provider,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    pub asset_class: AssetClass,
}

impl From<&ResolvedAsset> for AssetRef {
    /// Returns the reference identifying the asset.
    fn from(asset: &ResolvedAsset) -> Self {
        Self::new(asset.provider, &asset.symbol)
    }
}

/// Origin of the metadata stored for a shared asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    /// Supplied by a client through v1 synchronization, or stored before
    /// provenance was tracked. The provider has not confirmed it.
    ClientSupplied,
    /// Confirmed by the asset's provider.
    ProviderConfirmed,
}

/// Shared asset metadata as stored in the directory, with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAsset {
    /// The stored metadata.
    pub asset: ResolvedAsset,
    /// Whether the provider has confirmed that metadata.
    pub source: MetadataSource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_ref_normalizes_symbol() {
        // GIVEN a symbol with surrounding whitespace and lower-case letters,
        // WHEN an AssetRef is created,
        // THEN the symbol is trimmed and upper-cased.
        let reference = AssetRef::new(Provider::YF, "  vwce.de ");

        assert_eq!(reference.symbol(), "VWCE.DE");
    }
}
