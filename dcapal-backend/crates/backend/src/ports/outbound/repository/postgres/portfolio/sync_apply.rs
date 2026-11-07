//! Application of a v2 synchronization plan.
//!
//! The plan already holds every decision (which Portfolios are written, the
//! derived allocation rows, the tombstones). This module locks the rows the
//! plan was derived from, checks that they are unchanged, and executes
//! set-based statements covering all Portfolios of the plan at once, so the
//! number of statements does not depend on the number of Portfolios or assets.

use std::collections::HashMap;

use chrono::Utc;
use const_format::formatcp;
use rust_decimal::Decimal;
use sqlx::{
    Postgres, Row, Transaction,
    postgres::{PgArguments, PgRow},
    query,
    query::Query,
    query_as,
};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio_sync::{
        ExpectedVersion, PortfolioVersion, PortfolioWrite, SyncPlan, TombstoneKind, WriteKind,
    },
};
use crate::ports::outbound::repository::portfolio::{
    OwnershipConflict, PortfolioRepositoryError, Result,
};
use crate::ports::outbound::repository::postgres::convert::unnest_refs;
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, Provider as DbProvider,
};

use super::{FeeFields, SqlxPortfolioRepository};

/// A locked Portfolio row: identifier, owner, deleted flag, client timestamp and
/// server timestamp.
type LockedRow = (
    Uuid,
    Uuid,
    bool,
    chrono::DateTime<Utc>,
    chrono::DateTime<Utc>,
);

/// The name of the unique index on `portfolios.id`.
const PORTFOLIO_PRIMARY_KEY: &str = "portfolios_pkey";

/// The fee columns shared by Portfolios and Portfolio Assets, one array per
/// column. They are always bound first, as `$1` to `$6`.
#[derive(Default)]
struct FeeColumns {
    max_fee_impacts: Vec<Option<Decimal>>,
    fee_types: Vec<Option<String>>,
    fee_amounts: Vec<Option<Decimal>>,
    fee_rates: Vec<Option<Decimal>>,
    min_fees: Vec<Option<Decimal>>,
    max_fees: Vec<Option<Decimal>>,
}

impl FeeColumns {
    fn push(&mut self, fees: FeeFields) {
        self.max_fee_impacts.push(fees.max_fee_impact);
        self.fee_types.push(fees.fee_type);
        self.fee_amounts.push(fees.fee_amount);
        self.fee_rates.push(fees.fee_rate);
        self.min_fees.push(fees.min_fee);
        self.max_fees.push(fees.max_fee);
    }

    /// Binds the arrays as `$1` to `$6`, matching [`FEE_ARRAYS`].
    fn bind<'q>(
        &'q self,
        query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments> {
        query
            .bind(&self.max_fee_impacts)
            .bind(&self.fee_types)
            .bind(&self.fee_amounts)
            .bind(&self.fee_rates)
            .bind(&self.min_fees)
            .bind(&self.max_fees)
    }
}

/// The typed `UNNEST` arguments of [`FeeColumns`], in bind order.
const FEE_ARRAYS: &str =
    "$1::numeric[], $2::text[], $3::numeric[], $4::numeric[], $5::numeric[], $6::numeric[]";

/// The fee columns in the order of [`FEE_ARRAYS`].
const FEE_COLUMNS: &str = "max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee";

/// The `SET` assignments copying the fee columns from the row source `$source`.
macro_rules! fee_set {
    ($source:literal) => {
        concat!(
            "max_fee_impact = ",
            $source,
            ".max_fee_impact, fee_type = ",
            $source,
            ".fee_type, fee_amount = ",
            $source,
            ".fee_amount, fee_rate = ",
            $source,
            ".fee_rate, min_fee = ",
            $source,
            ".min_fee, max_fee = ",
            $source,
            ".max_fee"
        )
    };
}

/// The columns of the Portfolios to insert or update, one array per column.
#[derive(Default)]
struct PortfolioColumns {
    fees: FeeColumns,
    ids: Vec<Uuid>,
    names: Vec<String>,
    currencies: Vec<String>,
    last_updated_at: Vec<chrono::DateTime<Utc>>,
    allocation_modes: Vec<DbAllocationMode>,
    drift_bands: Vec<Option<Decimal>>,
    strategic_present: Vec<bool>,
}

/// The Portfolio row source: the fee arrays, then the arrays of
/// [`PortfolioColumns`] as `$7` to `$13`, aliased `w` with [`PORTFOLIO_COLUMNS`].
const PORTFOLIO_ROWS: &str = formatcp!(
    "UNNEST({FEE_ARRAYS}, $7::uuid[], $8::text[], $9::text[], $10::timestamptz[],
            $11::smallint[], $12::numeric[], $13::bool[])
          AS w({FEE_COLUMNS}, id, name, currency, last_updated_at, allocation_mode,
               drift_band, strategic_allocation_present)"
);

/// The target columns of [`PORTFOLIO_ROWS`], in order.
const PORTFOLIO_COLUMNS: &str = formatcp!(
    "{FEE_COLUMNS}, id, name, currency, last_updated_at, allocation_mode, drift_band,
     strategic_allocation_present"
);

impl PortfolioColumns {
    fn of_kind(plan: &SyncPlan, kind: WriteKind) -> Self {
        let mut columns = Self::default();
        for write in plan.writes.iter().filter(|write| write.kind == kind) {
            let content = &write.content;
            columns
                .fees
                .push(FeeFields::from_fees(content.fees.as_ref()));
            columns.ids.push(write.id);
            columns.names.push(content.name.clone());
            columns.currencies.push(content.quote_currency.clone());
            columns.last_updated_at.push(write.last_updated_at);
            columns
                .allocation_modes
                .push(content.allocation_mode.into());
            columns.drift_bands.push(content.drift_band);
            columns
                .strategic_present
                .push(content.strategic_allocation_present);
        }
        columns
    }

    /// Binds every array as `$1` to `$13`, matching [`PORTFOLIO_ROWS`].
    fn bind<'q>(
        &'q self,
        query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments> {
        self.fees
            .bind(query)
            .bind(&self.ids)
            .bind(&self.names)
            .bind(&self.currencies)
            .bind(&self.last_updated_at)
            .bind(&self.allocation_modes)
            .bind(&self.drift_bands)
            .bind(&self.strategic_present)
    }
}

/// Reports a unique violation of the Portfolio identifier as a stale
/// snapshot: another transaction created the row after the plan assumed it
/// absent. Other failures are reported unchanged.
fn insert_conflict_as_stale(error: sqlx::Error) -> PortfolioRepositoryError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
        && database_error.constraint() == Some(PORTFOLIO_PRIMARY_KEY)
    {
        return PortfolioRepositoryError::StaleSnapshot;
    }
    PortfolioRepositoryError::Database(error)
}

/// Locks the rows named by the plan and verifies that they are exactly as the
/// plan assumed.
///
/// Rows are locked in identifier order so concurrent applications cannot
/// deadlock. A row of another user is an ownership conflict; a missing,
/// unexpectedly present or changed row of this user is a stale snapshot.
async fn lock_and_verify(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    expected: &[ExpectedVersion],
) -> Result<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = expected.iter().map(|portfolio| portfolio.id).collect();
    let rows: Vec<LockedRow> = query_as(
        "SELECT id, user_id, deleted, last_updated_at, updated_at
         FROM portfolios
         WHERE id = ANY($1)
         ORDER BY id
         FOR UPDATE",
    )
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?;
    let locked: HashMap<Uuid, (Uuid, PortfolioVersion)> = rows
        .into_iter()
        .map(|(id, owner, deleted, last_updated_at, updated_at)| {
            (
                id,
                (
                    owner,
                    PortfolioVersion {
                        deleted,
                        last_updated_at,
                        updated_at,
                    },
                ),
            )
        })
        .collect();

    if let Some(foreign) = expected.iter().find(|portfolio| {
        locked
            .get(&portfolio.id)
            .is_some_and(|(owner, _)| *owner != user_id)
    }) {
        return Err(PortfolioRepositoryError::OwnershipConflict(
            OwnershipConflict::Portfolio { id: foreign.id },
        ));
    }
    let unchanged = expected.iter().all(|portfolio| {
        locked.get(&portfolio.id).map(|(_, version)| version) == portfolio.version.as_ref()
    });
    if unchanged {
        Ok(())
    } else {
        Err(PortfolioRepositoryError::StaleSnapshot)
    }
}

/// Looks up the shared asset identifiers of the given asset references.
///
/// The assets were onboarded by the service before the plan was applied, so a
/// missing reference is a caller bug rather than a user error.
async fn resolve_asset_ids<'a>(
    tx: &mut Transaction<'_, Postgres>,
    refs: impl IntoIterator<Item = &'a AssetRef>,
) -> Result<HashMap<&'a AssetRef, Uuid>> {
    let mut seen = std::collections::HashSet::new();
    let distinct: Vec<&AssetRef> = refs
        .into_iter()
        .filter(|reference| seen.insert(*reference))
        .collect();
    if distinct.is_empty() {
        return Ok(HashMap::new());
    }

    let (providers, symbols) = unnest_refs(distinct.iter().copied());
    let rows: Vec<(Uuid, DbProvider, String)> = query_as(
        "SELECT ad.id, ad.provider, ad.symbol
         FROM assets_data AS ad
         JOIN UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
           ON ad.provider = wanted.provider AND ad.symbol = wanted.symbol",
    )
    .bind(&providers)
    .bind(symbols)
    .fetch_all(&mut **tx)
    .await?;
    let stored: HashMap<(DbProvider, String), Uuid> = rows
        .into_iter()
        .map(|(id, provider, symbol)| ((provider, symbol), id))
        .collect();

    distinct
        .into_iter()
        .zip(providers)
        .map(|(reference, provider)| {
            stored
                .get(&(provider, reference.symbol.clone()))
                .map(|id| (reference, *id))
                .ok_or_else(|| {
                    PortfolioRepositoryError::AssetNotOnboarded(format!(
                        "{}:{}",
                        reference.provider.as_str(),
                        reference.symbol
                    ))
                })
        })
        .collect()
}

/// Inserts the Portfolios the plan creates, in one statement.
async fn insert_portfolios(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    columns: PortfolioColumns,
) -> Result<()> {
    if columns.ids.is_empty() {
        return Ok(());
    }
    columns
        .bind(query(formatcp!(
            "INSERT INTO portfolios ({PORTFOLIO_COLUMNS}, user_id, deleted)
             SELECT w.*, $14, $15 FROM {PORTFOLIO_ROWS}"
        )))
        .bind(user_id)
        .bind(false)
        .execute(&mut **tx)
        .await
        .map_err(insert_conflict_as_stale)?;
    Ok(())
}

/// Replaces the content of the existing Portfolios the plan updates, in one
/// statement.
async fn update_portfolios(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    columns: PortfolioColumns,
) -> Result<()> {
    if columns.ids.is_empty() {
        return Ok(());
    }
    let updated = columns
        .bind(query(formatcp!(
            "UPDATE portfolios AS p
             SET {}, name = w.name, currency = w.currency,
                 last_updated_at = w.last_updated_at, allocation_mode = w.allocation_mode,
                 drift_band = w.drift_band,
                 strategic_allocation_present = w.strategic_allocation_present,
                 updated_at = NOW()
             FROM {PORTFOLIO_ROWS}
             WHERE p.id = w.id AND p.user_id = $14",
            fee_set!("w")
        )))
        .bind(user_id)
        .execute(&mut **tx)
        .await?
        .rows_affected();
    // The rows were locked and verified, so a different count means they are
    // gone: do not report a half-applied plan as done.
    if updated != columns.ids.len() as u64 {
        return Err(PortfolioRepositoryError::StaleSnapshot);
    }
    Ok(())
}

/// Reconciles the Portfolio Asset relationships of all written Portfolios and
/// stores their allocation rows: one delete, one upsert of the relationships
/// and one upsert of the allocations.
///
/// Relationships are identified by (Portfolio, shared asset). Rows whose asset
/// is no longer submitted are deleted (their allocation rows cascade), kept
/// rows keep their identifier, and new rows get a fresh UUIDv7.
async fn write_assets(
    tx: &mut Transaction<'_, Postgres>,
    writes: &[&PortfolioWrite],
    asset_ids: &HashMap<&AssetRef, Uuid>,
) -> Result<()> {
    let updated_ids: Vec<Uuid> = writes
        .iter()
        .filter(|write| write.kind == WriteKind::Update)
        .map(|write| write.id)
        .collect();
    let mut kept_portfolios = Vec::new();
    let mut kept_assets = Vec::new();
    let mut relationships = RelationshipColumns::default();
    for write in writes {
        for stored in &write.content.assets {
            let asset = &stored.asset;
            let assets_data_id = asset_ids[&asset.asset];
            kept_portfolios.push(write.id);
            kept_assets.push(assets_data_id);
            relationships
                .fees
                .push(FeeFields::from_fees(asset.fees.as_ref()));
            relationships.ids.push(Uuid::now_v7());
            relationships.portfolio_ids.push(write.id);
            relationships.assets_data_ids.push(assets_data_id);
            relationships
                .class_overrides
                .push(asset.asset_class_override.map(DbAssetClass::from));
            relationships.quantities.push(asset.quantity);
            relationships.manual_prices.push(asset.manual_price);
            relationships
                .average_buy_prices
                .push(asset.average_buy_price);
        }
    }

    if !updated_ids.is_empty() {
        query(
            "DELETE FROM portfolio_asset AS pa
             WHERE pa.portfolio_id = ANY($1)
               AND NOT EXISTS (
                   SELECT 1
                   FROM UNNEST($2::uuid[], $3::uuid[]) AS kept(portfolio_id, assets_data_id)
                   WHERE kept.portfolio_id = pa.portfolio_id
                     AND kept.assets_data_id = pa.assets_data_id)",
        )
        .bind(&updated_ids)
        .bind(&kept_portfolios)
        .bind(&kept_assets)
        .execute(&mut **tx)
        .await?;
    }
    if relationships.ids.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    let persisted: Vec<(Uuid, Uuid, Uuid)> = relationships
        .bind(query(formatcp!(
            "INSERT INTO portfolio_asset
                 ({FEE_COLUMNS}, id, portfolio_id, assets_data_id, asset_class_override,
                  quantity, manual_price, average_buy_price, created_at, updated_at)
             SELECT r.*, $14, $14
             FROM UNNEST({FEE_ARRAYS}, $7::uuid[], $8::uuid[], $9::uuid[], $10::smallint[],
                         $11::numeric[], $12::numeric[], $13::numeric[])
                  AS r({FEE_COLUMNS}, id, portfolio_id, assets_data_id, asset_class_override,
                       quantity, manual_price, average_buy_price)
             ON CONFLICT (portfolio_id, assets_data_id) DO UPDATE
             SET {},
                 asset_class_override = EXCLUDED.asset_class_override,
                 quantity = EXCLUDED.quantity,
                 manual_price = EXCLUDED.manual_price,
                 average_buy_price = EXCLUDED.average_buy_price,
                 updated_at = EXCLUDED.updated_at
             RETURNING id, portfolio_id, assets_data_id",
            fee_set!("EXCLUDED")
        )))
        .bind(now)
        .map(|row: PgRow| {
            (
                row.get::<Uuid, _>("id"),
                row.get::<Uuid, _>("portfolio_id"),
                row.get::<Uuid, _>("assets_data_id"),
            )
        })
        .fetch_all(&mut **tx)
        .await?;
    let relationship_ids: HashMap<(Uuid, Uuid), Uuid> = persisted
        .into_iter()
        .map(|(id, portfolio_id, assets_data_id)| ((portfolio_id, assets_data_id), id))
        .collect();

    let mut allocation_portfolios = Vec::new();
    let mut allocation_relationships = Vec::new();
    let mut weights = Vec::new();
    for write in writes {
        for stored in &write.content.assets {
            let key = (write.id, asset_ids[&stored.asset.asset]);
            let relationship_id = relationship_ids.get(&key).ok_or_else(|| {
                PortfolioRepositoryError::CorruptData("relationship was not persisted".into())
            })?;
            allocation_portfolios.push(write.id);
            allocation_relationships.push(*relationship_id);
            weights.push(stored.weight);
        }
    }
    query(
        "INSERT INTO portfolio_asset_allocation
             (portfolio_id, portfolio_asset_id, weight, updated_at)
         SELECT a.portfolio_id, a.portfolio_asset_id, a.weight, $4
         FROM UNNEST($1::uuid[], $2::uuid[], $3::numeric[])
              AS a(portfolio_id, portfolio_asset_id, weight)
         ON CONFLICT (portfolio_id, portfolio_asset_id) DO UPDATE
         SET weight = EXCLUDED.weight, updated_at = EXCLUDED.updated_at",
    )
    .bind(&allocation_portfolios)
    .bind(&allocation_relationships)
    .bind(&weights)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The columns of the Portfolio Asset relationships to upsert, bound as
/// `$1` to `$13` (fees first).
#[derive(Default)]
struct RelationshipColumns {
    fees: FeeColumns,
    ids: Vec<Uuid>,
    portfolio_ids: Vec<Uuid>,
    assets_data_ids: Vec<Uuid>,
    class_overrides: Vec<Option<DbAssetClass>>,
    quantities: Vec<Decimal>,
    manual_prices: Vec<Option<Decimal>>,
    average_buy_prices: Vec<Option<Decimal>>,
}

impl RelationshipColumns {
    fn bind<'q>(
        &'q self,
        query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments> {
        self.fees
            .bind(query)
            .bind(&self.ids)
            .bind(&self.portfolio_ids)
            .bind(&self.assets_data_ids)
            .bind(&self.class_overrides)
            .bind(&self.quantities)
            .bind(&self.manual_prices)
            .bind(&self.average_buy_prices)
    }
}

/// Replaces the Strategic Asset Class targets of all written Portfolios: one
/// delete and at most one insert.
async fn replace_class_targets(
    tx: &mut Transaction<'_, Postgres>,
    writes: &[&PortfolioWrite],
) -> Result<()> {
    let updated_ids: Vec<Uuid> = writes
        .iter()
        .filter(|write| write.kind == WriteKind::Update)
        .map(|write| write.id)
        .collect();
    if !updated_ids.is_empty() {
        query("DELETE FROM strategic_asset_class_targets WHERE portfolio_id = ANY($1)")
            .bind(&updated_ids)
            .execute(&mut **tx)
            .await?;
    }

    let mut portfolio_ids = Vec::new();
    let mut classes = Vec::new();
    let mut weights = Vec::new();
    for write in writes {
        for target in &write.content.class_targets {
            portfolio_ids.push(write.id);
            classes.push(DbAssetClass::from(target.asset_class));
            weights.push(target.target_weight);
        }
    }
    if portfolio_ids.is_empty() {
        return Ok(());
    }
    query(
        "INSERT INTO strategic_asset_class_targets
             (portfolio_id, asset_class, target_weight, updated_at)
         SELECT t.portfolio_id, t.asset_class, t.target_weight, NOW()
         FROM UNNEST($1::uuid[], $2::smallint[], $3::numeric[])
              AS t(portfolio_id, asset_class, target_weight)",
    )
    .bind(&portfolio_ids)
    .bind(&classes)
    .bind(&weights)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Stores the plan's tombstones: existing rows are marked deleted and unknown
/// identifiers get a new tombstone row owned by the user.
async fn write_tombstones(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    plan: &SyncPlan,
) -> Result<()> {
    let ids_of = |kind: TombstoneKind| -> Vec<Uuid> {
        plan.tombstones
            .iter()
            .filter(|tombstone| tombstone.kind == kind)
            .map(|tombstone| tombstone.id)
            .collect()
    };
    let marked = ids_of(TombstoneKind::Mark);
    let created = ids_of(TombstoneKind::Create);

    if !marked.is_empty() {
        query(
            "UPDATE portfolios SET deleted = $3, updated_at = NOW()
             WHERE id = ANY($1) AND user_id = $2",
        )
        .bind(&marked)
        .bind(user_id)
        .bind(true)
        .execute(&mut **tx)
        .await?;
    }
    if !created.is_empty() {
        query(
            "INSERT INTO portfolios
                 (id, user_id, name, currency, deleted, last_updated_at,
                  allocation_mode, strategic_allocation_present)
             SELECT t.id, $2, $3, $4, $5, NOW(), $6, $7
             FROM UNNEST($1::uuid[]) AS t(id)",
        )
        .bind(&created)
        .bind(user_id)
        .bind("")
        .bind("")
        .bind(true)
        .bind(DbAllocationMode::Simple)
        .bind(false)
        .execute(&mut **tx)
        .await
        .map_err(insert_conflict_as_stale)?;
    }
    Ok(())
}

impl SqlxPortfolioRepository {
    /// Applies a synchronization plan in one transaction.
    ///
    /// With the rows named by the plan locked and verified, concurrent writers
    /// cannot change what the plan assumed, so the default read-committed
    /// isolation is enough; a Portfolio created concurrently under an
    /// identifier expected to be absent is caught by the unique index.
    pub(super) async fn apply_plan(&self, user_id: Uuid, plan: &SyncPlan) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        lock_and_verify(&mut tx, user_id, &plan.expected).await?;

        let writes: Vec<&PortfolioWrite> = plan.writes.iter().collect();
        // Collected before awaiting: a pending iterator over borrowed closures
        // would make the future unusable inside Axum handlers.
        let referenced: Vec<&AssetRef> = writes
            .iter()
            .flat_map(|write| {
                write
                    .content
                    .assets
                    .iter()
                    .map(|stored| &stored.asset.asset)
            })
            .collect();
        let asset_ids = resolve_asset_ids(&mut tx, referenced).await?;

        insert_portfolios(
            &mut tx,
            user_id,
            PortfolioColumns::of_kind(plan, WriteKind::Create),
        )
        .await?;
        update_portfolios(
            &mut tx,
            user_id,
            PortfolioColumns::of_kind(plan, WriteKind::Update),
        )
        .await?;
        write_assets(&mut tx, &writes, &asset_ids).await?;
        replace_class_targets(&mut tx, &writes).await?;
        write_tombstones(&mut tx, user_id, plan).await?;

        tx.commit().await?;
        Ok(())
    }
}
