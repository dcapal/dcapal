use chrono::{DateTime, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use uuid::Uuid;

use super::FeeFields;

/// Portfolio-wide allocation semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, sqlx::Type)]
#[repr(i16)]
pub enum AllocationMode {
    /// Every Portfolio Asset allocation is an absolute Portfolio target.
    Simple = 1,
    /// Asset Class targets are absolute and Portfolio Asset allocations are relative within a class.
    Strategic = 2,
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
    /// The stored fee policy columns.
    #[sqlx(flatten)]
    pub fees: FeeFields,
    /// When the database row was created.
    pub created_at: DateTime<Utc>,
    /// When the database row was last changed.
    pub updated_at: DateTime<Utc>,
}
