//! Application of a v2 synchronization plan.
//!
//! The plan already holds every decision (which Portfolios are written, the
//! derived allocation rows, the tombstones). This module locks the rows the
//! plan was derived from, checks that they are unchanged, and executes
//! set-based statements covering all Portfolios of the plan at once, so the
//! number of statements does not depend on the number of Portfolios or assets.

use std::collections::HashMap;

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query, query_as};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio_sync::{
        ExpectedPortfolio, PortfolioVersion, PortfolioWrite, SyncPlan, TombstoneKind, WriteKind,
    },
};
use crate::ports::outbound::repository::portfolio::{
    OwnershipConflict, PortfolioRepositoryError, Result,
};
use crate::ports::outbound::repository::postgres::convert::{
    db_allocation_mode, db_asset_class, unnest_refs,
};
use crate::ports::outbound::repository::postgres::types::AllocationMode as DbAllocationMode;

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

/// The columns of the Portfolios to insert or update, one array per column.
#[derive(Default)]
struct PortfolioColumns {
    ids: Vec<Uuid>,
    names: Vec<String>,
    currencies: Vec<String>,
    last_updated_at: Vec<chrono::DateTime<Utc>>,
    allocation_modes: Vec<i16>,
    drift_bands: Vec<Option<Decimal>>,
    strategic_present: Vec<bool>,
    max_fee_impacts: Vec<Option<Decimal>>,
    fee_types: Vec<Option<String>>,
    fee_amounts: Vec<Option<Decimal>>,
    fee_rates: Vec<Option<Decimal>>,
    min_fees: Vec<Option<Decimal>>,
    max_fees: Vec<Option<Decimal>>,
}

impl PortfolioColumns {
    fn of_kind(plan: &SyncPlan, kind: WriteKind) -> Self {
        let mut columns = Self::default();
        for write in plan.writes.iter().filter(|write| write.kind == kind) {
            let portfolio = &write.portfolio;
            let fees = FeeFields::from_fees(portfolio.fees.as_ref());
            columns.ids.push(portfolio.id);
            columns.names.push(portfolio.name.clone());
            columns.currencies.push(portfolio.quote_currency.clone());
            columns.last_updated_at.push(portfolio.last_updated_at);
            columns
                .allocation_modes
                .push(i16::from(db_allocation_mode(portfolio.allocation_mode)));
            columns.drift_bands.push(portfolio.drift_band);
            columns
                .strategic_present
                .push(write.strategic_allocation_present);
            columns.max_fee_impacts.push(fees.max_fee_impact);
            columns.fee_types.push(fees.fee_type);
            columns.fee_amounts.push(fees.fee_amount);
            columns.fee_rates.push(fees.fee_rate);
            columns.min_fees.push(fees.min_fee);
            columns.max_fees.push(fees.max_fee);
        }
        columns
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
    expected: &[ExpectedPortfolio],
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
    let rows: Vec<(Uuid, i16, String)> = query_as(
        "SELECT ad.id, ad.provider, ad.symbol
         FROM assets_data AS ad
         JOIN UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
           ON ad.provider = wanted.provider AND ad.symbol = wanted.symbol",
    )
    .bind(&providers)
    .bind(symbols)
    .fetch_all(&mut **tx)
    .await?;
    let stored: HashMap<(i16, String), Uuid> = rows
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
    query(
        "INSERT INTO portfolios
             (id, user_id, name, currency, deleted, last_updated_at,
              allocation_mode, drift_band, strategic_allocation_present,
              max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee)
         SELECT w.id, $2, w.name, w.currency, $3, w.last_updated_at,
                w.allocation_mode, w.drift_band, w.strategic_allocation_present,
                w.max_fee_impact, w.fee_type, w.fee_amount, w.fee_rate, w.min_fee, w.max_fee
         FROM UNNEST($1::uuid[], $4::text[], $5::text[], $6::timestamptz[],
                     $7::smallint[], $8::numeric[], $9::bool[], $10::numeric[],
                     $11::text[], $12::numeric[], $13::numeric[], $14::numeric[],
                     $15::numeric[])
              AS w(id, name, currency, last_updated_at, allocation_mode, drift_band,
                   strategic_allocation_present, max_fee_impact, fee_type, fee_amount,
                   fee_rate, min_fee, max_fee)",
    )
    .bind(&columns.ids)
    .bind(user_id)
    .bind(false)
    .bind(&columns.names)
    .bind(&columns.currencies)
    .bind(&columns.last_updated_at)
    .bind(&columns.allocation_modes)
    .bind(&columns.drift_bands)
    .bind(&columns.strategic_present)
    .bind(&columns.max_fee_impacts)
    .bind(&columns.fee_types)
    .bind(&columns.fee_amounts)
    .bind(&columns.fee_rates)
    .bind(&columns.min_fees)
    .bind(&columns.max_fees)
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
    let updated = query(
        "UPDATE portfolios AS p
         SET name = w.name, currency = w.currency, last_updated_at = w.last_updated_at,
             allocation_mode = w.allocation_mode, drift_band = w.drift_band,
             strategic_allocation_present = w.strategic_allocation_present,
             max_fee_impact = w.max_fee_impact, fee_type = w.fee_type,
             fee_amount = w.fee_amount, fee_rate = w.fee_rate,
             min_fee = w.min_fee, max_fee = w.max_fee,
             updated_at = NOW()
         FROM UNNEST($1::uuid[], $3::text[], $4::text[], $5::timestamptz[],
                     $6::smallint[], $7::numeric[], $8::bool[], $9::numeric[],
                     $10::text[], $11::numeric[], $12::numeric[], $13::numeric[],
                     $14::numeric[])
              AS w(id, name, currency, last_updated_at, allocation_mode, drift_band,
                   strategic_allocation_present, max_fee_impact, fee_type, fee_amount,
                   fee_rate, min_fee, max_fee)
         WHERE p.id = w.id AND p.user_id = $2",
    )
    .bind(&columns.ids)
    .bind(user_id)
    .bind(&columns.names)
    .bind(&columns.currencies)
    .bind(&columns.last_updated_at)
    .bind(&columns.allocation_modes)
    .bind(&columns.drift_bands)
    .bind(&columns.strategic_present)
    .bind(&columns.max_fee_impacts)
    .bind(&columns.fee_types)
    .bind(&columns.fee_amounts)
    .bind(&columns.fee_rates)
    .bind(&columns.min_fees)
    .bind(&columns.max_fees)
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
        .map(|write| write.portfolio.id)
        .collect();
    let mut kept_portfolios = Vec::new();
    let mut kept_assets = Vec::new();
    let mut relationships = RelationshipColumns::default();
    for write in writes {
        for asset in &write.portfolio.assets {
            let fees = FeeFields::from_fees(asset.fees.as_ref());
            let assets_data_id = asset_ids[&asset.asset];
            kept_portfolios.push(write.portfolio.id);
            kept_assets.push(assets_data_id);
            relationships.ids.push(Uuid::now_v7());
            relationships.portfolio_ids.push(write.portfolio.id);
            relationships.assets_data_ids.push(assets_data_id);
            relationships.class_overrides.push(
                asset
                    .asset_class_override
                    .map(db_asset_class)
                    .map(i16::from),
            );
            relationships.quantities.push(asset.quantity);
            relationships.manual_prices.push(asset.manual_price);
            relationships
                .average_buy_prices
                .push(asset.average_buy_price);
            relationships.max_fee_impacts.push(fees.max_fee_impact);
            relationships.fee_types.push(fees.fee_type);
            relationships.fee_amounts.push(fees.fee_amount);
            relationships.fee_rates.push(fees.fee_rate);
            relationships.min_fees.push(fees.min_fee);
            relationships.max_fees.push(fees.max_fee);
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
    let persisted: Vec<(Uuid, Uuid, Uuid)> = query_as(
        "INSERT INTO portfolio_asset
             (id, portfolio_id, assets_data_id, asset_class_override,
              quantity, manual_price, average_buy_price, max_fee_impact,
              fee_type, fee_amount, fee_rate, min_fee, max_fee,
              created_at, updated_at)
         SELECT r.id, r.portfolio_id, r.assets_data_id, r.asset_class_override,
                r.quantity, r.manual_price, r.average_buy_price, r.max_fee_impact,
                r.fee_type, r.fee_amount, r.fee_rate, r.min_fee, r.max_fee, $14, $14
         FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::smallint[], $5::numeric[],
                     $6::numeric[], $7::numeric[], $8::numeric[], $9::text[],
                     $10::numeric[], $11::numeric[], $12::numeric[], $13::numeric[])
              AS r(id, portfolio_id, assets_data_id, asset_class_override, quantity,
                   manual_price, average_buy_price, max_fee_impact, fee_type,
                   fee_amount, fee_rate, min_fee, max_fee)
         ON CONFLICT (portfolio_id, assets_data_id) DO UPDATE
         SET asset_class_override = EXCLUDED.asset_class_override,
             quantity = EXCLUDED.quantity,
             manual_price = EXCLUDED.manual_price,
             average_buy_price = EXCLUDED.average_buy_price,
             max_fee_impact = EXCLUDED.max_fee_impact,
             fee_type = EXCLUDED.fee_type,
             fee_amount = EXCLUDED.fee_amount,
             fee_rate = EXCLUDED.fee_rate,
             min_fee = EXCLUDED.min_fee,
             max_fee = EXCLUDED.max_fee,
             updated_at = EXCLUDED.updated_at
         RETURNING id, portfolio_id, assets_data_id",
    )
    .bind(&relationships.ids)
    .bind(&relationships.portfolio_ids)
    .bind(&relationships.assets_data_ids)
    .bind(&relationships.class_overrides)
    .bind(&relationships.quantities)
    .bind(&relationships.manual_prices)
    .bind(&relationships.average_buy_prices)
    .bind(&relationships.max_fee_impacts)
    .bind(&relationships.fee_types)
    .bind(&relationships.fee_amounts)
    .bind(&relationships.fee_rates)
    .bind(&relationships.min_fees)
    .bind(&relationships.max_fees)
    .bind(now)
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
        for allocation in &write.allocations {
            let key = (write.portfolio.id, asset_ids[&allocation.asset]);
            let relationship_id = relationship_ids.get(&key).ok_or_else(|| {
                PortfolioRepositoryError::CorruptData("relationship was not persisted".into())
            })?;
            allocation_portfolios.push(write.portfolio.id);
            allocation_relationships.push(*relationship_id);
            weights.push(allocation.weight);
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

/// The columns of the Portfolio Asset relationships to upsert.
#[derive(Default)]
struct RelationshipColumns {
    ids: Vec<Uuid>,
    portfolio_ids: Vec<Uuid>,
    assets_data_ids: Vec<Uuid>,
    class_overrides: Vec<Option<i16>>,
    quantities: Vec<Decimal>,
    manual_prices: Vec<Option<Decimal>>,
    average_buy_prices: Vec<Option<Decimal>>,
    max_fee_impacts: Vec<Option<Decimal>>,
    fee_types: Vec<Option<String>>,
    fee_amounts: Vec<Option<Decimal>>,
    fee_rates: Vec<Option<Decimal>>,
    min_fees: Vec<Option<Decimal>>,
    max_fees: Vec<Option<Decimal>>,
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
        .map(|write| write.portfolio.id)
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
        for target in &write.class_targets {
            portfolio_ids.push(write.portfolio.id);
            classes.push(i16::from(db_asset_class(target.asset_class)));
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
        .bind(i16::from(DbAllocationMode::Simple))
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
            .flat_map(|write| write.portfolio.assets.iter().map(|asset| &asset.asset))
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
