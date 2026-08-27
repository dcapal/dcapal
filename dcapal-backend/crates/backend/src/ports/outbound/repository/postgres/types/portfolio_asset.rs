use chrono::{DateTime, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rust_decimal::Decimal;
use uuid::Uuid;

/// The provider codes used by canonical Portfolio Asset storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(i16)]
pub enum Provider {
    /// Yahoo Finance.
    YF = 2,
    /// Kraken.
    Kraken = 1,
}

impl Provider {
    /// Converts a legacy synchronization provider into canonical storage.
    pub fn from_legacy(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("dcapal") || value.eq_ignore_ascii_case("kraken") {
            Some(Self::Kraken)
        } else if value.eq_ignore_ascii_case("yf") || value.eq_ignore_ascii_case("yahoo") {
            Some(Self::YF)
        } else {
            None
        }
    }

    /// Returns the v1 synchronization provider name for this canonical provider.
    pub const fn as_legacy_name(self) -> &'static str {
        match self {
            Self::Kraken => "DCAPal",
            Self::YF => "YF",
        }
    }
}

/// The Asset Class codes used by canonical Portfolio Asset storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(i16)]
pub enum AssetClass {
    /// An unclassified or unsupported asset.
    Other = 0,
    /// An equity asset.
    Equity = 1,
    /// A bond asset.
    Bond = 2,
    /// Cash or cash-equivalent assets.
    Cash = 3,
    /// A cryptocurrency asset.
    Crypto = 4,
    /// A commodity asset.
    Commodity = 5,
}

impl AssetClass {
    /// Converts a legacy synchronization Asset Class into canonical storage.
    pub fn from_legacy(value: &str) -> Self {
        Self::normalize_legacy(value).unwrap_or(Self::Other)
    }

    /// Converts a recognized legacy Asset Class name into its canonical class.
    pub fn normalize_legacy(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("equity") {
            Some(Self::Equity)
        } else if value.eq_ignore_ascii_case("bond") {
            Some(Self::Bond)
        } else if value.eq_ignore_ascii_case("currency") || value.eq_ignore_ascii_case("cash") {
            Some(Self::Cash)
        } else if value.eq_ignore_ascii_case("crypto") {
            Some(Self::Crypto)
        } else if value.eq_ignore_ascii_case("commodity") {
            Some(Self::Commodity)
        } else if value.eq_ignore_ascii_case("other") {
            Some(Self::Other)
        } else {
            None
        }
    }

    /// Returns the v1 synchronization Asset Class name for this canonical class.
    pub const fn as_legacy_name(self) -> &'static str {
        match self {
            Self::Other => "OTHER",
            Self::Equity => "EQUITY",
            Self::Bond => "BOND",
            Self::Cash => "CURRENCY",
            Self::Crypto => "CRYPTO",
            Self::Commodity => "COMMODITY",
        }
    }
}

/// A joined shared-asset and Portfolio relationship row after enum decoding.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PortfolioAssetRow {
    /// The Portfolio Asset relationship identifier.
    pub id: Uuid,
    /// The portfolio containing the asset.
    pub portfolio_id: Uuid,
    /// The shared asset metadata identifier.
    pub assets_data_id: Uuid,
    /// The provider symbol for the shared asset.
    pub symbol: String,
    /// The immutable shared asset display name.
    pub name: String,
    /// The shared default Asset Class.
    #[sqlx(try_from = "i16")]
    pub asset_class: AssetClass,
    /// The Portfolio-specific Asset Class override.
    pub asset_class_override: Option<i16>,
    /// The shared asset's trading currency.
    pub currency: String,
    /// The shared asset provider.
    #[sqlx(try_from = "i16")]
    pub provider: Provider,
    /// The current quantity held.
    pub quantity: Decimal,
    /// The target portfolio weight.
    pub target_weight: Decimal,
    /// The Portfolio-specific manual price override.
    pub manual_price: Option<Decimal>,
    /// The maximum fee impact configured for transactions.
    pub max_fee_impact: Option<Decimal>,
    /// The persisted fee structure name.
    pub fee_type: Option<String>,
    /// The fixed fee amount, when the fee structure is fixed.
    pub fee_amount: Option<Decimal>,
    /// The variable fee rate, when the fee structure is variable.
    pub fee_rate: Option<Decimal>,
    /// The minimum variable fee, when configured.
    pub min_fee: Option<Decimal>,
    /// The maximum variable fee, when configured.
    pub max_fee: Option<Decimal>,
    /// The average price paid for the current holding.
    pub average_buy_price: Option<Decimal>,
    /// When the database row was created.
    pub created_at: DateTime<Utc>,
    /// When the database row was last changed.
    pub updated_at: DateTime<Utc>,
}

impl PortfolioAssetRow {
    /// Returns the Portfolio-specific class when present, otherwise the shared default.
    pub fn effective_asset_class(&self) -> AssetClass {
        self.asset_class_override
            .and_then(|value| AssetClass::try_from(value).ok())
            .unwrap_or(self.asset_class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_codes_round_trip() {
        // GIVEN each supported Provider code, WHEN it converts to and from storage,
        // THEN it preserves the assigned value and rejects unsupported codes.
        assert_eq!(i16::from(Provider::Kraken), 1);
        assert_eq!(i16::from(Provider::YF), 2);
        assert_eq!(Provider::try_from(1), Ok(Provider::Kraken));
        assert_eq!(Provider::try_from(2), Ok(Provider::YF));
        assert!(Provider::try_from(0).is_err());
    }

    #[test]
    fn asset_class_codes_round_trip() {
        // GIVEN each supported Asset Class code, WHEN it converts to and from storage,
        // THEN it preserves the assigned value and rejects unsupported codes.
        let values = [
            (AssetClass::Other, 0),
            (AssetClass::Equity, 1),
            (AssetClass::Bond, 2),
            (AssetClass::Cash, 3),
            (AssetClass::Crypto, 4),
            (AssetClass::Commodity, 5),
        ];

        for (asset_class, value) in values {
            assert_eq!(i16::from(asset_class), value);
            assert_eq!(AssetClass::try_from(value), Ok(asset_class));
        }

        assert!(AssetClass::try_from(6).is_err());
    }

    #[test]
    fn legacy_provider_aliases_are_case_insensitive() {
        // GIVEN every supported legacy provider alias in mixed casing, WHEN it is normalized,
        // THEN it maps to the canonical provider and v1 response name.
        let aliases = [
            ("dCaPaL", Provider::Kraken, "DCAPal"),
            ("kRaKeN", Provider::Kraken, "DCAPal"),
            ("yF", Provider::YF, "YF"),
            ("YaHoO", Provider::YF, "YF"),
        ];

        for (value, expected, wire_name) in aliases {
            assert_eq!(Provider::from_legacy(value), Some(expected));
            assert_eq!(expected.as_legacy_name(), wire_name);
        }

        assert_eq!(Provider::from_legacy("IBKR"), None);
    }

    #[test]
    fn legacy_values_map_to_canonical_codes() {
        // GIVEN a legacy Asset Class name, WHEN storage values are derived,
        // THEN known aliases use canonical codes and unsupported names use the stated policy.
        assert_eq!(Provider::Kraken.as_legacy_name(), "DCAPal");
        assert_eq!(AssetClass::from_legacy("CURRENCY"), AssetClass::Cash);
        assert_eq!(AssetClass::from_legacy("unclassified"), AssetClass::Other);
    }

    #[test]
    fn legacy_asset_class_aliases_are_case_insensitive() {
        // GIVEN supported legacy Asset Class aliases in different casing, WHEN they are normalized,
        // THEN each alias maps to the canonical class used by v1.
        let aliases = [
            ("equity", AssetClass::Equity, "EQUITY"),
            ("Bond", AssetClass::Bond, "BOND"),
            ("cash", AssetClass::Cash, "CURRENCY"),
            ("Currency", AssetClass::Cash, "CURRENCY"),
            ("crypto", AssetClass::Crypto, "CRYPTO"),
            ("COMMODITY", AssetClass::Commodity, "COMMODITY"),
            ("other", AssetClass::Other, "OTHER"),
        ];

        for (value, expected, wire_name) in aliases {
            assert_eq!(AssetClass::normalize_legacy(value), Some(expected));
            assert_eq!(expected.as_legacy_name(), wire_name);
        }
    }
}
