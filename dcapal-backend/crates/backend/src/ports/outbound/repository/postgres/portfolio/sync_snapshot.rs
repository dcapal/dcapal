//! Loading of the complete v2 synchronization snapshot of one user.
//!
//! This module only maps stored rows to the domain snapshot, strictly: values
//! that cannot belong to a valid aggregate are reported as corrupt data.

use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio::{
        FeeStructure as AppFeeStructure, PortfolioAssetV2, StrategicAssetClassTargetV2,
        TransactionFees,
    },
    portfolio_sync::{
        PortfolioVersion, StoredAggregate, StoredAsset, StoredPortfolio, SyncSnapshot,
    },
};
use crate::ports::outbound::repository::portfolio::PortfolioRepositoryError;
use crate::ports::outbound::repository::portfolio::Result;
use crate::ports::outbound::repository::postgres::convert::{
    app_allocation_mode, app_asset_class, app_provider,
};
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, Provider as DbProvider,
};

use super::SqlxPortfolioRepository;

/// A row from the complete v2 synchronization snapshot: one Portfolio joined
/// with one of its assets, or alone when it has none.
#[derive(Debug, sqlx::FromRow)]
struct SnapshotRow {
    portfolio_id: Uuid,
    portfolio_name: String,
    portfolio_currency: String,
    portfolio_deleted: bool,
    portfolio_last_updated_at: chrono::DateTime<chrono::Utc>,
    portfolio_updated_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(try_from = "i16")]
    portfolio_allocation_mode: DbAllocationMode,
    portfolio_drift_band: Option<Decimal>,
    portfolio_strategic_allocation_present: bool,
    portfolio_max_fee_impact: Option<Decimal>,
    portfolio_fee_type: Option<String>,
    portfolio_fee_amount: Option<Decimal>,
    portfolio_fee_rate: Option<Decimal>,
    portfolio_min_fee: Option<Decimal>,
    portfolio_max_fee: Option<Decimal>,
    asset_id: Option<Uuid>,
    asset_symbol: Option<String>,
    asset_class_override: Option<i16>,
    asset_provider: Option<i16>,
    asset_quantity: Option<Decimal>,
    asset_target_weight: Option<Decimal>,
    asset_manual_price: Option<Decimal>,
    asset_max_fee_impact: Option<Decimal>,
    asset_fee_type: Option<String>,
    asset_fee_amount: Option<Decimal>,
    asset_fee_rate: Option<Decimal>,
    asset_min_fee: Option<Decimal>,
    asset_max_fee: Option<Decimal>,
    asset_average_buy_price: Option<Decimal>,
}

/// A stored Strategic Asset Class target of one Portfolio.
#[derive(Debug, sqlx::FromRow)]
struct ClassTargetRow {
    portfolio_id: Uuid,
    asset_class: i16,
    target_weight: Decimal,
}

fn corrupt_data(message: impl Into<String>) -> PortfolioRepositoryError {
    PortfolioRepositoryError::CorruptData(message.into())
}

fn stored_percentage_is_valid(value: Decimal) -> bool {
    (Decimal::ZERO..=Decimal::from(100)).contains(&value)
}

/// Decodes the fee columns of a Portfolio or Portfolio Asset, rejecting
/// combinations no valid fee policy can produce.
fn decode_fees(
    max_fee_impact: Option<Decimal>,
    fee_type: Option<String>,
    fee_amount: Option<Decimal>,
    fee_rate: Option<Decimal>,
    min_fee: Option<Decimal>,
    max_fee: Option<Decimal>,
) -> Result<Option<TransactionFees>> {
    if let Some(value) = max_fee_impact
        && !stored_percentage_is_valid(value)
    {
        return Err(corrupt_data("Portfolio fee maxFeeImpact is outside 0..100"));
    }

    let fee_structure = match fee_type.as_deref() {
        None => {
            if max_fee_impact.is_some()
                || fee_amount.is_some()
                || fee_rate.is_some()
                || min_fee.is_some()
                || max_fee.is_some()
            {
                return Err(corrupt_data(
                    "Portfolio fee fields are present without a fee type",
                ));
            }
            return Ok(None);
        }
        Some("ZeroFee") => {
            if fee_amount.is_some() || fee_rate.is_some() || min_fee.is_some() || max_fee.is_some()
            {
                return Err(corrupt_data(
                    "ZeroFee Portfolio fee contains unrelated numeric fields",
                ));
            }
            AppFeeStructure::ZeroFee
        }
        Some("Fixed") => {
            if fee_rate.is_some() || min_fee.is_some() || max_fee.is_some() {
                return Err(corrupt_data(
                    "Fixed Portfolio fee contains variable fee fields",
                ));
            }
            let fee_amount = fee_amount
                .ok_or_else(|| corrupt_data("Fixed Portfolio fee is missing feeAmount"))?;
            if fee_amount < Decimal::ZERO {
                return Err(corrupt_data("Fixed Portfolio fee is negative"));
            }
            AppFeeStructure::Fixed { fee_amount }
        }
        Some("Variable") => {
            if fee_amount.is_some() {
                return Err(corrupt_data(
                    "Variable Portfolio fee contains a fixed fee amount",
                ));
            }
            let fee_rate = fee_rate
                .ok_or_else(|| corrupt_data("Variable Portfolio fee is missing feeRate"))?;
            if !stored_percentage_is_valid(fee_rate) {
                return Err(corrupt_data(
                    "Variable Portfolio fee rate is outside 0..100",
                ));
            }
            if min_fee.is_some_and(|value| value < Decimal::ZERO)
                || max_fee.is_some_and(|value| value < Decimal::ZERO)
            {
                return Err(corrupt_data("Variable Portfolio fee bounds are negative"));
            }
            if let (Some(max_fee), Some(min_fee)) = (max_fee, min_fee)
                && max_fee < min_fee
            {
                return Err(corrupt_data(
                    "Variable Portfolio fee maxFee is below minFee",
                ));
            }
            AppFeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            }
        }
        Some(other) => return Err(corrupt_data(format!("unknown Portfolio fee type {other}"))),
    };
    Ok(Some(TransactionFees {
        max_fee_impact,
        fee_structure,
    }))
}

/// Decodes the stored Strategic Asset Class targets of one Portfolio, ignoring
/// repeated classes.
fn decode_class_targets(rows: Vec<ClassTargetRow>) -> Result<Vec<StrategicAssetClassTargetV2>> {
    let mut class_ids = HashSet::new();
    let mut targets = Vec::new();
    for row in rows {
        if !class_ids.insert(row.asset_class) {
            continue;
        }
        let asset_class = DbAssetClass::try_from(row.asset_class).map_err(|_| {
            corrupt_data("Portfolio snapshot contains an invalid Strategic Asset Class")
        })?;
        if !stored_percentage_is_valid(row.target_weight) {
            return Err(corrupt_data(
                "Portfolio snapshot contains an invalid Strategic target weight",
            ));
        }
        targets.push(StrategicAssetClassTargetV2 {
            asset_class: app_asset_class(asset_class),
            target_weight: row.target_weight,
        });
    }
    Ok(targets)
}

fn has_partial_asset_data(row: &SnapshotRow) -> bool {
    row.asset_symbol.is_some()
        || row.asset_class_override.is_some()
        || row.asset_provider.is_some()
        || row.asset_quantity.is_some()
        || row.asset_target_weight.is_some()
        || row.asset_manual_price.is_some()
        || row.asset_max_fee_impact.is_some()
        || row.asset_fee_type.is_some()
        || row.asset_fee_amount.is_some()
        || row.asset_fee_rate.is_some()
        || row.asset_min_fee.is_some()
        || row.asset_max_fee.is_some()
        || row.asset_average_buy_price.is_some()
}

/// Decodes the asset part of a snapshot row, or `None` when the Portfolio has
/// no asset in this row.
fn decode_asset(row: &mut SnapshotRow) -> Result<Option<StoredAsset>> {
    let Some(_) = row.asset_id else {
        if has_partial_asset_data(row) {
            return Err(corrupt_data(
                "Portfolio snapshot contains asset fields without an asset id",
            ));
        }
        return Ok(None);
    };
    let symbol = row
        .asset_symbol
        .take()
        .ok_or_else(|| corrupt_data("Portfolio snapshot is missing an Asset symbol"))?;
    let provider = DbProvider::try_from(
        row.asset_provider
            .ok_or_else(|| corrupt_data("Portfolio snapshot is missing an Asset provider"))?,
    )
    .map_err(|_| corrupt_data("Portfolio snapshot contains an invalid Provider"))?;
    let asset_class_override = row
        .asset_class_override
        .map(|value| {
            DbAssetClass::try_from(value)
                .map(app_asset_class)
                .map_err(|_| corrupt_data("Portfolio Asset has an invalid aclassOverride"))
        })
        .transpose()?;
    let fees = decode_fees(
        row.asset_max_fee_impact,
        row.asset_fee_type.take(),
        row.asset_fee_amount,
        row.asset_fee_rate,
        row.asset_min_fee,
        row.asset_max_fee,
    )?;
    Ok(Some(StoredAsset {
        asset: PortfolioAssetV2 {
            asset: AssetRef {
                provider: app_provider(provider),
                symbol,
            },
            asset_class_override,
            quantity: row
                .asset_quantity
                .ok_or_else(|| corrupt_data("Portfolio snapshot is missing an Asset quantity"))?,
            target_weight: None,
            manual_price: row.asset_manual_price,
            average_buy_price: row.asset_average_buy_price,
            fees,
        },
        weight: row.asset_target_weight.unwrap_or_default(),
    }))
}

/// Builds the stored Portfolio from its first row, with the assets and class
/// targets decoded so far.
fn finish_portfolio(
    head: SnapshotRow,
    assets: Vec<StoredAsset>,
    class_target_rows: Vec<ClassTargetRow>,
) -> Result<StoredPortfolio> {
    let version = PortfolioVersion {
        deleted: head.portfolio_deleted,
        last_updated_at: head.portfolio_last_updated_at,
        updated_at: head.portfolio_updated_at,
    };
    if head.portfolio_deleted {
        return Ok(StoredPortfolio {
            id: head.portfolio_id,
            version,
            aggregate: None,
        });
    }
    Ok(StoredPortfolio {
        id: head.portfolio_id,
        version,
        aggregate: Some(StoredAggregate {
            name: head.portfolio_name,
            quote_currency: head.portfolio_currency,
            allocation_mode: app_allocation_mode(head.portfolio_allocation_mode),
            drift_band: head.portfolio_drift_band,
            fees: decode_fees(
                head.portfolio_max_fee_impact,
                head.portfolio_fee_type,
                head.portfolio_fee_amount,
                head.portfolio_fee_rate,
                head.portfolio_min_fee,
                head.portfolio_max_fee,
            )?,
            strategic_allocation_present: head.portfolio_strategic_allocation_present,
            assets,
            class_targets: decode_class_targets(class_target_rows)?,
        }),
    })
}

/// Groups the joined rows by Portfolio and decodes each one.
fn decode_snapshot(
    rows: Vec<SnapshotRow>,
    class_target_rows: Vec<ClassTargetRow>,
) -> Result<SyncSnapshot> {
    let mut class_targets_by_portfolio = HashMap::<Uuid, Vec<ClassTargetRow>>::new();
    for row in class_target_rows {
        class_targets_by_portfolio
            .entry(row.portfolio_id)
            .or_default()
            .push(row);
    }

    let mut portfolios = Vec::new();
    // The Portfolio being accumulated: its first row, its assets and their ids.
    let mut current: Option<(SnapshotRow, Vec<StoredAsset>)> = None;
    let mut asset_ids = HashSet::new();
    let mut complete = |current: Option<(SnapshotRow, Vec<StoredAsset>)>,
                        portfolios: &mut Vec<StoredPortfolio>|
     -> Result<()> {
        if let Some((head, assets)) = current {
            let class_targets = class_targets_by_portfolio
                .remove(&head.portfolio_id)
                .unwrap_or_default();
            portfolios.push(finish_portfolio(head, assets, class_targets)?);
        }
        Ok(())
    };

    for mut row in rows {
        if current.as_ref().map(|(head, _)| head.portfolio_id) != Some(row.portfolio_id) {
            complete(current.take(), &mut portfolios)?;
            asset_ids.clear();
        }
        let asset_id = row.asset_id;
        let asset = decode_asset(&mut row)?;
        if let Some(asset_id) = asset_id
            && !asset_ids.insert(asset_id)
        {
            return Err(corrupt_data(
                "Portfolio snapshot contains a duplicate Portfolio Asset row",
            ));
        }
        let (_, assets) = current.get_or_insert_with(|| (row, Vec::new()));
        assets.extend(asset);
    }
    complete(current.take(), &mut portfolios)?;
    Ok(SyncSnapshot { portfolios })
}

impl SqlxPortfolioRepository {
    /// Loads and decodes the complete v2 aggregate snapshot for one user in a
    /// single repeatable-read, read-only transaction, so that the Portfolio
    /// rows and their class targets come from the same state.
    pub(super) async fn load_snapshot(&self, user_id: Uuid) -> Result<SyncSnapshot> {
        let mut tx = self.pool.begin().await?;
        query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let rows = query_as::<_, SnapshotRow>(
            "SELECT p.id AS portfolio_id,
                    p.name AS portfolio_name, p.currency AS portfolio_currency,
                    p.deleted AS portfolio_deleted,
                    p.last_updated_at AS portfolio_last_updated_at,
                    p.updated_at AS portfolio_updated_at,
                    p.allocation_mode AS portfolio_allocation_mode,
                    p.drift_band AS portfolio_drift_band,
                    p.strategic_allocation_present AS portfolio_strategic_allocation_present,
                    p.max_fee_impact AS portfolio_max_fee_impact,
                    p.fee_type AS portfolio_fee_type, p.fee_amount AS portfolio_fee_amount,
                    p.fee_rate AS portfolio_fee_rate, p.min_fee AS portfolio_min_fee,
                    p.max_fee AS portfolio_max_fee,
                    pa.id AS asset_id, ad.symbol AS asset_symbol,
                    pa.asset_class_override AS asset_class_override,
                    ad.provider AS asset_provider,
                    pa.quantity AS asset_quantity, paa.weight AS asset_target_weight,
                    pa.manual_price AS asset_manual_price,
                    pa.max_fee_impact AS asset_max_fee_impact, pa.fee_type AS asset_fee_type,
                    pa.fee_amount AS asset_fee_amount, pa.fee_rate AS asset_fee_rate,
                    pa.min_fee AS asset_min_fee, pa.max_fee AS asset_max_fee,
                    pa.average_buy_price AS asset_average_buy_price
             FROM portfolios AS p
             LEFT JOIN portfolio_asset AS pa ON pa.portfolio_id = p.id AND NOT p.deleted
             LEFT JOIN assets_data AS ad ON ad.id = pa.assets_data_id
             LEFT JOIN portfolio_asset_allocation AS paa
               ON paa.portfolio_id = pa.portfolio_id
              AND paa.portfolio_asset_id = pa.id
             WHERE p.user_id = $1
             ORDER BY p.id, pa.id",
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;
        // Class targets are loaded apart so they do not multiply the asset rows;
        // soft-deleted Portfolios are tombstones and need neither assets nor targets.
        let class_target_rows = query_as::<_, ClassTargetRow>(
            "SELECT t.portfolio_id, t.asset_class, t.target_weight
             FROM strategic_asset_class_targets AS t
             JOIN portfolios AS p ON p.id = t.portfolio_id
             WHERE p.user_id = $1 AND NOT p.deleted
             ORDER BY t.portfolio_id, t.asset_class",
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        decode_snapshot(rows, class_target_rows)
    }
}
