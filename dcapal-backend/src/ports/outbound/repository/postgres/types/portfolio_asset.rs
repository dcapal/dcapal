use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

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
