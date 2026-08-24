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

/// A row from the `portfolio_asset` table.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PortfolioAssetRow {
    /// The asset row identifier.
    pub id: Uuid,
    /// The provider symbol for the asset.
    pub symbol: String,
    /// The portfolio containing the asset.
    pub portfolio_id: Uuid,
    /// The asset display name.
    pub name: String,
    /// The asset class reported by the provider.
    pub asset_class: String,
    /// The asset's trading currency.
    pub currency: String,
    /// The provider supplying the asset data.
    pub provider: String,
    /// The current quantity held.
    pub quantity: Decimal,
    /// The target portfolio weight.
    pub target_weight: Decimal,
    /// The latest known asset price.
    pub price: Decimal,
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
}
