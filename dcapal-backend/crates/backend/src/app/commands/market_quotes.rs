//! Commands of the v2 market-data endpoints: asset search and FX rates.

use crate::app::domain::{fiat::is_fiat, portfolio::Provider};

/// Longest accepted asset search text, in characters.
pub const MAX_SEARCH_QUERY_CHARS: usize = 100;

/// Validation failures of the v2 market-data commands.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarketCommandError {
    /// The provider of a search is not one of the supported providers.
    #[error("provider must be one of {supported}")]
    UnsupportedProvider { supported: &'static str },
    /// A required query parameter is missing or blank.
    #[error("must not be empty")]
    Required { field: &'static str },
    /// The search text is longer than [`MAX_SEARCH_QUERY_CHARS`].
    #[error("must be at most {MAX_SEARCH_QUERY_CHARS} characters")]
    QueryTooLong,
    /// A currency is not one of the catalog fiat currencies.
    #[error("must be a supported fiat currency")]
    UnknownCurrency { field: &'static str },
}

impl MarketCommandError {
    /// The request field the failure is about.
    pub fn field(&self) -> &'static str {
        match self {
            Self::UnsupportedProvider { .. } => "provider",
            Self::QueryTooLong => "query",
            Self::Required { field } | Self::UnknownCurrency { field } => field,
        }
    }
}

/// Validated request to search the assets of one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAssetsCommand {
    /// The provider whose listings are searched.
    pub provider: Provider,
    /// Trimmed, non-empty search text.
    pub query: String,
}

impl SearchAssetsCommand {
    /// Validates the provider identifier and the search text.
    pub fn try_new(provider: &str, query: Option<&str>) -> Result<Self, MarketCommandError> {
        let provider =
            Provider::parse(provider).ok_or(MarketCommandError::UnsupportedProvider {
                supported: Provider::names(),
            })?;
        let query = query.map(str::trim).unwrap_or_default();
        if query.is_empty() {
            return Err(MarketCommandError::Required { field: "query" });
        }
        if query.chars().count() > MAX_SEARCH_QUERY_CHARS {
            return Err(MarketCommandError::QueryTooLong);
        }
        Ok(Self {
            provider,
            query: query.to_string(),
        })
    }
}

/// Validated request for the FX rate between two fiat currencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxRateCommand {
    /// Upper-case base currency.
    pub base: String,
    /// Upper-case quote currency.
    pub quote: String,
}

impl FxRateCommand {
    /// Checks that both currencies are present catalog fiat currencies.
    pub fn try_new(base: Option<&str>, quote: Option<&str>) -> Result<Self, MarketCommandError> {
        let currency = |value: Option<&str>, field: &'static str| {
            let value = value.map(str::trim).unwrap_or_default();
            if value.is_empty() {
                Err(MarketCommandError::Required { field })
            } else if is_fiat(value) {
                Ok(value.to_ascii_uppercase())
            } else {
                Err(MarketCommandError::UnknownCurrency { field })
            }
        };
        Ok(Self {
            base: currency(base, "base")?,
            quote: currency(quote, "quote")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GIVEN provider identifiers and search texts of every shape,
    /// WHEN the search command is built,
    /// THEN supported providers are accepted case-insensitively with a trimmed query, and an
    /// unsupported provider, a blank query or a very long query is rejected with its field.
    #[test]
    fn search_command_validates_provider_and_query() {
        let command = SearchAssetsCommand::try_new("YF", Some("  vwce ")).unwrap();
        assert_eq!(
            (command.provider, command.query.as_str()),
            (Provider::YF, "vwce")
        );
        assert!(SearchAssetsCommand::try_new("kraken", Some("btc")).is_ok());

        let unsupported = SearchAssetsCommand::try_new("ibkr", Some("x")).unwrap_err();
        assert_eq!(unsupported.field(), "provider");
        for blank in [None, Some("   ")] {
            let error = SearchAssetsCommand::try_new("yf", blank).unwrap_err();
            assert_eq!(error, MarketCommandError::Required { field: "query" });
        }
        let long = "x".repeat(MAX_SEARCH_QUERY_CHARS + 1);
        assert_eq!(
            SearchAssetsCommand::try_new("yf", Some(&long)).unwrap_err(),
            MarketCommandError::QueryTooLong
        );
    }

    /// GIVEN currency codes that are catalog fiat, crypto, missing or unknown,
    /// WHEN the FX command is built,
    /// THEN catalog fiat codes are upper-cased and anything else is rejected with its field.
    #[test]
    fn fx_command_accepts_only_catalog_fiat() {
        let command = FxRateCommand::try_new(Some("eur"), Some(" USD ")).unwrap();
        assert_eq!(
            (command.base.as_str(), command.quote.as_str()),
            ("EUR", "USD")
        );

        assert_eq!(
            FxRateCommand::try_new(Some("btc"), Some("usd")).unwrap_err(),
            MarketCommandError::UnknownCurrency { field: "base" }
        );
        assert_eq!(
            FxRateCommand::try_new(Some("eur"), None).unwrap_err(),
            MarketCommandError::Required { field: "quote" }
        );
    }
}
