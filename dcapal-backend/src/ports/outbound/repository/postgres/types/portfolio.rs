use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PortfolioRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub currency: String,
    pub deleted: bool,
    pub last_updated_at: DateTime<Utc>,
    pub max_fee_impact: Option<Decimal>,
    pub fee_type: Option<String>,
    pub fee_amount: Option<Decimal>,
    pub fee_rate: Option<Decimal>,
    pub min_fee: Option<Decimal>,
    pub max_fee: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
