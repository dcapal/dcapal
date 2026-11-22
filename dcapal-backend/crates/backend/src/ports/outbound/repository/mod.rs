//! The [`repository`](self) module contains interfaces to persistent storage
//! services, like PostgreSQL.

use std::collections::HashMap;

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::{PgPool, query, query_as, query_scalar};
use uuid::Uuid;

use crate::{DateTime, error::Result};
pub mod asset;
pub mod market_data;
pub mod portfolio;
/// PostgreSQL-backed repository implementations.
pub mod postgres;
pub mod user;

/// Stores when periodic workers last completed.
#[derive(Clone)]
pub struct MiscRepository {
    pool: PgPool,
}

impl MiscRepository {
    const CW_LAST_FETCHED: &'static str = "cw-last-fetched";

    /// Creates a repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns when market discovery last completed, if it ever did.
    pub async fn get_cw_last_fetched(&self) -> Result<Option<DateTime>> {
        let completed_at: Option<jiff_sqlx::Timestamp> =
            query_scalar("SELECT completed_at FROM worker_marker WHERE name = $1")
                .bind(Self::CW_LAST_FETCHED)
                .fetch_optional(&self.pool)
                .await?;
        Ok(completed_at.map(jiff_sqlx::Timestamp::to_jiff))
    }

    /// Records when market discovery last completed.
    pub async fn set_cw_last_fetched(&self, ts: DateTime) -> Result<()> {
        query(
            "INSERT INTO worker_marker (name, completed_at) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET completed_at = EXCLUDED.completed_at",
        )
        .bind(Self::CW_LAST_FETCHED)
        .bind(ts.to_sqlx())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Stores visitor and import counters exposed as metrics.
#[derive(Clone)]
pub struct StatsRepository {
    pool: PgPool,
}

impl StatsRepository {
    const IMPORTED_PORTFOLIO_KEY: &'static str = "imported-portfolio-total";

    /// Creates a repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Counts one more visit of an IP address and returns its visit total.
    pub async fn bump_visit(&self, ip: &str) -> Result<i64> {
        Ok(query_scalar(
            "INSERT INTO visitor (ip, visits) VALUES ($1, 1)
             ON CONFLICT (ip) DO UPDATE SET visits = visitor.visits + 1
             RETURNING visits",
        )
        .bind(ip)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Returns the visit total of every known IP address.
    pub async fn fetch_all_visitors(&self) -> Result<HashMap<String, i64>> {
        let rows: Vec<(String, i64)> = query_as("SELECT ip, visits FROM visitor")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().collect())
    }

    /// Counts one more imported portfolio and returns the new total.
    pub async fn increase_imported_portfolio_count(&self) -> Result<i64> {
        Ok(query_scalar(
            "INSERT INTO app_counter (name, value) VALUES ($1, 1)
             ON CONFLICT (name) DO UPDATE SET value = app_counter.value + 1
             RETURNING value",
        )
        .bind(Self::IMPORTED_PORTFOLIO_KEY)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Returns how many portfolios were imported so far.
    pub async fn get_imported_portfolio_count(&self) -> Result<i64> {
        let count: Option<i64> = query_scalar("SELECT value FROM app_counter WHERE name = $1")
            .bind(Self::IMPORTED_PORTFOLIO_KEY)
            .fetch_optional(&self.pool)
            .await?;

        Ok(count.unwrap_or_default())
    }
}

/// Stores short-lived portfolio transfers created by the import endpoint.
#[derive(Clone)]
pub struct ImportedRepository {
    pool: PgPool,
}

/// A stored portfolio transfer and the instant it stops being readable.
pub struct ImportedPortfolio {
    pub id: Uuid,
    pub expires_at: DateTime,
}

impl ImportedRepository {
    /// How long an imported portfolio can be fetched.
    const TTL: std::time::Duration = std::time::Duration::from_secs(60);

    /// Creates a repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores a portfolio for [`Self::TTL`] and purges the transfers that already expired.
    pub async fn store_portfolio(&self, pfolio: &serde_json::Value) -> Result<ImportedPortfolio> {
        // Whole seconds, like the expiry the clients were given before.
        let expires_at = Timestamp::from_second((Timestamp::now() + Self::TTL).as_second())
            .unwrap_or_else(|_| Timestamp::now());
        let id = Uuid::new_v4();

        // One statement purges and inserts atomically, saving the transaction round-trips.
        query(
            "WITH purged AS (DELETE FROM imported_portfolio WHERE expires_at <= NOW())
             INSERT INTO imported_portfolio (id, payload, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(pfolio)
        .bind(expires_at.to_sqlx())
        .execute(&self.pool)
        .await?;

        Ok(ImportedPortfolio { id, expires_at })
    }

    /// Returns an imported portfolio unless it expired or never existed.
    pub async fn find_portfolio(&self, id: &str) -> Result<Option<serde_json::Value>> {
        // An identifier that is not a UUID cannot name a stored portfolio.
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(None);
        };

        Ok(query_scalar(
            "SELECT payload FROM imported_portfolio WHERE id = $1 AND expires_at > NOW()",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }
}
