//! Provider-bound asset references and their resolved shared metadata.

use crate::app::domain::portfolio::{AssetClass, Provider};

/// Reference to an asset by the provider that lists it and its provider symbol.
///
/// The symbol is stored trimmed and upper-cased, matching the canonical form
/// persisted for shared asset metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetRef {
    pub provider: Provider,
    pub symbol: String,
}

impl AssetRef {
    /// Creates a reference, normalizing the symbol to its canonical form.
    pub fn new(provider: Provider, symbol: &str) -> Self {
        Self {
            provider,
            symbol: symbol.trim().to_ascii_uppercase(),
        }
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

impl ResolvedAsset {
    /// Returns the reference identifying this asset.
    pub fn as_ref(&self) -> AssetRef {
        AssetRef {
            provider: self.provider.clone(),
            symbol: self.symbol.clone(),
        }
    }
}

/// Origin of the metadata stored for a shared asset.
///
/// The discriminants are the values persisted in `assets_data.metadata_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum MetadataSource {
    /// Supplied by a client through v1 synchronization, or stored before
    /// provenance was tracked. The provider has not confirmed it.
    ClientSupplied = 0,
    /// Confirmed by the asset's provider.
    ProviderConfirmed = 1,
}

impl MetadataSource {
    /// Returns the persisted discriminant.
    pub const fn code(self) -> i16 {
        self as i16
    }
}

impl TryFrom<i16> for MetadataSource {
    type Error = i16;

    /// Decodes a persisted discriminant, returning it back when unknown.
    fn try_from(code: i16) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::ClientSupplied),
            1 => Ok(Self::ProviderConfirmed),
            other => Err(other),
        }
    }
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

        assert_eq!(reference.symbol, "VWCE.DE");
    }
}
