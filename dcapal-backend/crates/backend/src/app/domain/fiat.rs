//! The fixed catalog of fiat currencies known to the market-data catalog.
//!
//! Fiat currencies are shared asset metadata with the Yahoo provider and the
//! Cash asset class, named in English. Their prices against each other come from
//! Yahoo (see the FX ticker table in the Yahoo adapter), not from Kraken.

/// Upper-case symbol and English name of every catalog fiat currency.
pub const FIAT_CATALOG: [(&str, &str); 8] = [
    ("AED", "United Arab Emirates Dirham"),
    ("AUD", "Australian Dollar"),
    ("CAD", "Canadian Dollar"),
    ("CHF", "Swiss franc"),
    ("EUR", "Euro"),
    ("GBP", "British Pound"),
    ("JPY", "Japanese Yen"),
    ("USD", "United States Dollar"),
];

/// Returns the upper-case symbols of the catalog fiat currencies.
pub fn fiat_symbols() -> Vec<&'static str> {
    FIAT_CATALOG.iter().map(|(symbol, _)| *symbol).collect()
}

/// Returns the English name of a catalog fiat currency, matching the symbol
/// or lower-case id case-insensitively.
pub fn fiat_name(id: &str) -> Option<&'static str> {
    FIAT_CATALOG
        .iter()
        .find(|(symbol, _)| symbol.eq_ignore_ascii_case(id))
        .map(|(_, name)| *name)
}

/// Tells whether the symbol or lower-case id is a catalog fiat currency.
pub fn is_fiat(id: &str) -> bool {
    fiat_name(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lookups_ignore_case() {
        // GIVEN the fiat catalog, WHEN an id is looked up in lower or upper case,
        // THEN the English name is found, and unknown ids are not fiat.
        assert_eq!(fiat_name("eur"), Some("Euro"));
        assert_eq!(fiat_name("EUR"), Some("Euro"));
        assert!(is_fiat("usd"));
        assert!(!is_fiat("btc"));
        assert_eq!(fiat_symbols().len(), 8);
    }
}
