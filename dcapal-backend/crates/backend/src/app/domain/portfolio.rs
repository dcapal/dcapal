use std::sync::LazyLock;

use rust_decimal::Decimal;
use strum::VariantNames;
use uuid::Uuid;

use crate::{DateTime, app::domain::asset::AssetRef};

/// Provider supported by the Portfolio aggregate.
///
/// The lowercase variant names are the canonical v2 provider identifiers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::IntoStaticStr,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Provider {
    /// Yahoo Finance.
    YF,
    /// Kraken.
    Kraken,
}

impl Provider {
    /// Returns the canonical v2 provider identifier.
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    /// Returns every canonical provider identifier as a comma-separated list,
    /// for validation messages.
    pub fn names() -> &'static str {
        static NAMES: LazyLock<String> =
            LazyLock::new(|| <Provider as VariantNames>::VARIANTS.join(", "));
        NAMES.as_str()
    }

    /// Parses a canonical v2 provider identifier, ignoring case and
    /// surrounding whitespace.
    pub fn parse(value: &str) -> Option<Self> {
        value.trim().parse().ok()
    }
}

/// Tells whether `value` is a percentage between 0 and 100 inclusive.
pub fn is_percentage(value: Decimal) -> bool {
    (Decimal::ZERO..=Decimal::from(100)).contains(&value)
}

/// Asset class used by Portfolio allocation rules.
///
/// The declaration order is the canonical ordering of class targets in
/// synchronization responses (it matches the stored class codes). The lowercase
/// variant names are the canonical v2 asset-class identifiers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::Display,
    strum::IntoStaticStr,
    strum::EnumString,
    strum::EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum AssetClass {
    /// Assets with no more specific classification.
    Other,
    /// Equity securities.
    Equity,
    /// Fixed-income securities.
    Bond,
    /// Cash and cash equivalents.
    Cash,
    /// Cryptocurrencies.
    Crypto,
    /// Commodities.
    Commodity,
}

impl AssetClass {
    /// Returns the canonical v2 asset-class identifier.
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    /// Parses a canonical v2 asset-class identifier, ignoring case and
    /// surrounding whitespace.
    pub fn parse(value: &str) -> Option<Self> {
        value.trim().parse().ok()
    }
}

/// Portfolio-wide allocation semantics.
///
/// The lowercase variant names are the stable v2 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum AllocationMode {
    /// Each Portfolio Asset target is an absolute Portfolio weight.
    Simple,
    /// Asset Class targets are absolute and Portfolio Asset targets are relative.
    Strategic,
}

/// Portfolio-specific holding, fee, and allocation values for one referenced asset.
///
/// The relationship is identified by `(Portfolio, asset)`; shared asset
/// metadata is never carried by the aggregate.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioAssetV2 {
    pub asset: AssetRef,
    pub asset_class_override: Option<AssetClass>,
    pub quantity: Decimal,
    pub target_weight: Option<Decimal>,
    pub manual_price: Option<Decimal>,
    pub average_buy_price: Option<Decimal>,
    pub fees: Option<TransactionFees>,
}

/// Strategic class and relative Portfolio Asset targets.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategicAllocationV2 {
    pub asset_class_targets: Vec<StrategicAssetClassTargetV2>,
    pub asset_targets: Vec<StrategicAssetTargetV2>,
}

/// An absolute target for one Asset Class.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategicAssetClassTargetV2 {
    pub asset_class: AssetClass,
    pub target_weight: Decimal,
}

/// A relative target for one Portfolio Asset.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategicAssetTargetV2 {
    pub asset: AssetRef,
    pub relative_target_weight: Decimal,
}

/// Portfolio- or asset-level transaction fee policy.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionFees {
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructure,
}

/// Fee policy variants persisted by the Portfolio aggregate.
///
/// [`FeeType`] is the fieldless counterpart; its names are the values stored
/// in the `fee_type` columns.
#[derive(Debug, Clone, PartialEq, strum::EnumDiscriminants)]
#[strum_discriminants(name(FeeType))]
#[strum_discriminants(derive(strum::Display, strum::EnumString))]
pub enum FeeStructure {
    /// No transaction fee applies.
    ZeroFee,
    /// A fixed amount applies to each transaction.
    Fixed { fee_amount: Decimal },
    /// A percentage rate applies, optionally bounded by minimum and maximum amounts.
    Variable {
        fee_rate: Decimal,
        min_fee: Option<Decimal>,
        max_fee: Option<Decimal>,
    },
}

impl FeeStructure {
    /// Returns the fieldless kind of this fee policy.
    pub fn fee_type(&self) -> FeeType {
        self.into()
    }
}

/// Complete v2 synchronization input after HTTP decoding.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncPortfoliosV2Request {
    /// The only accepted request schema version.
    pub schema_version: u16,
    /// Complete portfolio aggregates supplied by the client.
    pub portfolios: Vec<PortfolioV2>,
    /// Portfolio identifiers that should become tombstones.
    pub deleted_portfolios: Vec<Uuid>,
}

/// Complete saved Portfolio aggregate used by the application layer.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioV2 {
    pub id: Uuid,
    pub name: String,
    pub quote_currency: String,
    pub allocation_mode: AllocationMode,
    pub drift_band: Option<Decimal>,
    pub fees: Option<TransactionFees>,
    pub assets: Vec<PortfolioAssetV2>,
    pub strategic_allocation: Option<StrategicAllocationV2>,
    pub last_updated_at: DateTime,
}

/// Canonical synchronization result returned by the application service.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncPortfoliosV2Response {
    pub updated_portfolios: Vec<PortfolioV2>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn provider_names_are_lowercase_and_parse_case_insensitively() {
        // GIVEN every supported provider,
        // WHEN its identifier is read and parsed back, also with odd casing and padding,
        // THEN identifiers are the lowercase names in declaration order and parsing
        // accepts them, while unknown identifiers are rejected.
        let names: Vec<_> = Provider::iter().map(|provider| provider.as_str()).collect();

        assert_eq!(names, ["yf", "kraken"]);
        assert_eq!(Provider::names(), "yf, kraken");
        assert_eq!(Provider::parse(" Kraken "), Some(Provider::Kraken));
        assert_eq!(Provider::parse("YF"), Some(Provider::YF));
        assert_eq!(Provider::parse("ibkr"), None);
    }

    #[test]
    fn asset_class_names_round_trip() {
        // GIVEN every asset class,
        // WHEN its identifier is read and parsed back,
        // THEN the lowercase name parses to the same class, ignoring case and padding.
        for class in AssetClass::iter() {
            assert_eq!(AssetClass::parse(class.as_str()), Some(class));
            assert_eq!(
                AssetClass::parse(&format!(" {} ", class.as_str().to_uppercase())),
                Some(class)
            );
        }

        assert_eq!(AssetClass::Commodity.as_str(), "commodity");
        assert_eq!(AssetClass::parse("currency"), None);
    }

    #[test]
    fn fee_type_uses_its_stored_names() {
        // GIVEN the fee policies,
        // WHEN they are rendered as names,
        // THEN fee types keep the stored variant names.
        assert_eq!(FeeStructure::ZeroFee.fee_type().to_string(), "ZeroFee");
        assert_eq!(FeeType::Fixed.to_string(), "Fixed");
        assert_eq!("Variable".parse(), Ok(FeeType::Variable));
    }
}
