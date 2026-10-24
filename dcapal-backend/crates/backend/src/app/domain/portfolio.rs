use derive_more::Display;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{DateTime, app::domain::asset::AssetRef};

/// Provider supported by the Portfolio aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum Provider {
    /// Yahoo Finance.
    YF,
    /// Kraken.
    Kraken,
}

impl Provider {
    /// Returns the canonical v2 provider identifier.
    pub fn as_str(&self) -> &str {
        match self {
            Self::YF => "yf",
            Self::Kraken => "kraken",
        }
    }

    /// Tells whether the provider's own lookup yields metadata better than what
    /// a client may have cached, so a refresh may replace the cached name.
    ///
    /// The Kraken catalog only knows the symbol, which would overwrite a
    /// possibly more descriptive cached name; Yahoo Finance reports real names.
    pub const fn has_authoritative_metadata(&self) -> bool {
        match self {
            Self::YF => true,
            Self::Kraken => false,
        }
    }

    /// Parses a canonical v2 provider identifier.
    pub fn parse(value: &str) -> Option<Self> {
        if value.trim().eq_ignore_ascii_case("yf") {
            Some(Self::YF)
        } else if value.trim().eq_ignore_ascii_case("kraken") {
            Some(Self::Kraken)
        } else {
            None
        }
    }
}

/// Asset class used by Portfolio allocation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
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
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Other => "other",
            Self::Equity => "equity",
            Self::Bond => "bond",
            Self::Cash => "cash",
            Self::Crypto => "crypto",
            Self::Commodity => "commodity",
        }
    }

    /// Parses a canonical v2 asset-class identifier.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        [
            ("other", Self::Other),
            ("equity", Self::Equity),
            ("bond", Self::Bond),
            ("cash", Self::Cash),
            ("crypto", Self::Crypto),
            ("commodity", Self::Commodity),
        ]
        .into_iter()
        .find_map(|(name, class)| value.eq_ignore_ascii_case(name).then_some(class))
    }
}

/// Portfolio-wide allocation semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
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
#[derive(Debug, Clone, PartialEq, Display)]
pub enum FeeStructure {
    /// No transaction fee applies.
    #[display("ZeroFee")]
    ZeroFee,
    /// A fixed amount applies to each transaction.
    #[display("Fixed")]
    Fixed { fee_amount: Decimal },
    /// A percentage rate applies, optionally bounded by minimum and maximum amounts.
    #[display("Variable")]
    Variable {
        fee_rate: Decimal,
        min_fee: Option<Decimal>,
        max_fee: Option<Decimal>,
    },
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
#[derive(Debug, Clone, PartialEq)]
pub struct SyncPortfoliosV2Response {
    pub updated_portfolios: Vec<PortfolioV2>,
    pub deleted_portfolios: Vec<Uuid>,
}
