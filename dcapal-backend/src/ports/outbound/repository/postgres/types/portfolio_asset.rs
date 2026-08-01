use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PortfolioAssetRow {
    pub id: Uuid,
    pub symbol: String,
    pub portfolio_id: Uuid,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub provider: String,
    pub quantity: Decimal,
    pub target_weight: Decimal,
    pub price: Decimal,
    pub max_fee_impact: Option<Decimal>,
    pub fee_type: Option<String>,
    pub fee_amount: Option<Decimal>,
    pub fee_rate: Option<Decimal>,
    pub min_fee: Option<Decimal>,
    pub max_fee: Option<Decimal>,
    pub average_buy_price: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
