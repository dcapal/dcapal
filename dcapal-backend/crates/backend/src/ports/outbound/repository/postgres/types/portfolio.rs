use chrono::{DateTime, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Portfolio-wide allocation semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(i16)]
pub enum AllocationMode {
    /// Every Portfolio Asset allocation is an absolute Portfolio target.
    Simple = 1,
    /// Asset Class targets are absolute and Portfolio Asset allocations are relative within a class.
    Strategic = 2,
}

impl AllocationMode {
    /// Returns the stable v2 wire name for this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Strategic => "strategic",
        }
    }
}

/// A row from the `portfolios` table.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PortfolioRow {
    /// The portfolio identifier.
    pub id: Uuid,
    /// The user who owns the portfolio.
    pub user_id: Uuid,
    /// The portfolio display name.
    pub name: String,
    /// The currency used for portfolio values.
    pub currency: String,
    /// Whether the portfolio has been soft-deleted.
    pub deleted: bool,
    /// The timestamp supplied by the client for synchronization.
    pub last_updated_at: DateTime<Utc>,
    /// The portfolio-wide allocation semantics.
    #[sqlx(try_from = "i16")]
    pub allocation_mode: AllocationMode,
    /// The optional portfolio drift band used by rebalancing.
    pub drift_band: Option<Decimal>,
    /// Whether the client explicitly supplied a Strategic Allocation object.
    pub strategic_allocation_present: bool,
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
    /// When the database row was created.
    pub created_at: DateTime<Utc>,
    /// When the database row was last changed.
    pub updated_at: DateTime<Utc>,
}
