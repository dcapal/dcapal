use std::collections::HashSet;

use crate::app::domain::{asset::AssetRef, portfolio::Provider};

/// Maximum number of asset references accepted by one lookup.
pub const MAX_LOOKUP_ASSETS: usize = 20;

/// Validation failures of a lookup request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LookupAssetsCommandError {
    /// No asset reference was submitted.
    #[error("at least one asset is required")]
    Empty,
    /// More than [`MAX_LOOKUP_ASSETS`] references were submitted.
    #[error("at most {MAX_LOOKUP_ASSETS} assets can be looked up at once")]
    TooMany,
    /// The reference at `index` is not a `<provider>:<symbol>` pair of a supported provider.
    #[error("asset at index {index} is invalid: {reason}")]
    Invalid { index: usize, reason: &'static str },
}

/// Validated, de-duplicated request to read known asset metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupAssetsCommand {
    assets: Vec<AssetRef>,
}

impl LookupAssetsCommand {
    /// Parses `<provider>:<symbol>` values, splitting on the first colon.
    ///
    /// The submitted count (before de-duplication) must be within
    /// `1..=MAX_LOOKUP_ASSETS`. Duplicates collapse onto their first
    /// occurrence, so the command keeps request order.
    pub fn try_new(raw_assets: &[String]) -> Result<Self, LookupAssetsCommandError> {
        if raw_assets.is_empty() {
            return Err(LookupAssetsCommandError::Empty);
        }
        if raw_assets.len() > MAX_LOOKUP_ASSETS {
            return Err(LookupAssetsCommandError::TooMany);
        }

        let mut seen = HashSet::new();
        let mut assets = Vec::with_capacity(raw_assets.len());
        for (index, raw) in raw_assets.iter().enumerate() {
            let reference = parse_asset_ref(raw)
                .map_err(|reason| LookupAssetsCommandError::Invalid { index, reason })?;
            if seen.insert(reference.clone()) {
                assets.push(reference);
            }
        }

        Ok(Self { assets })
    }

    /// Returns the distinct references in request order.
    pub fn assets(&self) -> &[AssetRef] {
        &self.assets
    }
}

fn parse_asset_ref(raw: &str) -> Result<AssetRef, &'static str> {
    let (provider, symbol) = raw
        .split_once(':')
        .ok_or("must have the form <provider>:<symbol>")?;
    let provider = Provider::parse(provider).ok_or("provider must be one of yf, kraken")?;
    let reference = AssetRef::new(provider, symbol);
    if reference.symbol.is_empty() {
        return Err("symbol must not be empty");
    }
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn parses_on_first_colon_and_normalizes() {
        // GIVEN references whose symbol contains a colon and mixed case,
        // WHEN the command is built,
        // THEN the provider is split on the first colon and symbols are upper-cased.
        let command = LookupAssetsCommand::try_new(&raw(&["yf:vwce.de", "kraken: btc "])).unwrap();

        assert_eq!(
            command.assets(),
            [
                AssetRef::new(Provider::YF, "VWCE.DE"),
                AssetRef::new(Provider::Kraken, "BTC")
            ]
        );
        let colon = LookupAssetsCommand::try_new(&raw(&["yf:a:b"])).unwrap();
        assert_eq!(colon.assets()[0].symbol, "A:B");
    }

    #[test]
    fn duplicates_collapse_in_request_order() {
        // GIVEN a request repeating a reference with different casing,
        // WHEN the command is built,
        // THEN the reference appears once at its first position.
        let command =
            LookupAssetsCommand::try_new(&raw(&["yf:aapl", "kraken:btc", "yf:AAPL"])).unwrap();

        assert_eq!(command.assets().len(), 2);
        assert_eq!(command.assets()[0].symbol, "AAPL");
    }

    #[test]
    fn rejects_empty_and_oversized_batches() {
        // GIVEN no references, and 21 references,
        // WHEN the command is built, THEN both are rejected; 20 are accepted.
        assert_eq!(
            LookupAssetsCommand::try_new(&[]),
            Err(LookupAssetsCommandError::Empty)
        );
        let many = (0..21).map(|i| format!("yf:S{i}")).collect::<Vec<_>>();
        assert_eq!(
            LookupAssetsCommand::try_new(&many),
            Err(LookupAssetsCommandError::TooMany)
        );
        assert!(LookupAssetsCommand::try_new(&many[..20]).is_ok());
    }

    #[test]
    fn rejects_malformed_references_with_their_index() {
        // GIVEN references missing a colon, naming an unknown provider, or with a blank symbol,
        // WHEN the command is built, THEN the first offending index is reported.
        for bad in ["aapl", "ibkr:aapl", "yf:", "yf:   ", ":aapl"] {
            assert!(
                matches!(
                    LookupAssetsCommand::try_new(&raw(&["yf:ok", bad])),
                    Err(LookupAssetsCommandError::Invalid { index: 1, .. })
                ),
                "{bad}"
            );
        }
    }
}
