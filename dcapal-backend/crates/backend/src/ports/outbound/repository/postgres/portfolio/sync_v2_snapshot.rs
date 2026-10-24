use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query_as};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio::{
        AllocationMode as AppAllocationMode, AssetClass as AppAssetClass,
        FeeStructure as AppFeeStructure, PortfolioAssetV2, PortfolioV2, Provider as AppProvider,
        StrategicAllocationV2, StrategicAssetClassTargetV2, StrategicAssetTargetV2,
        TransactionFees,
    },
};
use crate::ports::outbound::repository::portfolio::PortfolioRepositoryError;
use crate::ports::outbound::repository::portfolio::Result;
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, Provider as DbProvider,
};

use super::PortfolioRow;

/// A row from the complete v2 synchronization snapshot.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct V2SnapshotRow {
    pub(super) portfolio_id: Uuid,
    pub(super) portfolio_user_id: Uuid,
    pub(super) portfolio_name: String,
    pub(super) portfolio_currency: String,
    pub(super) portfolio_deleted: bool,
    pub(super) portfolio_last_updated_at: chrono::DateTime<chrono::Utc>,
    pub(super) portfolio_created_at: chrono::DateTime<chrono::Utc>,
    pub(super) portfolio_updated_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(try_from = "i16")]
    pub(super) portfolio_allocation_mode: DbAllocationMode,
    pub(super) portfolio_drift_band: Option<Decimal>,
    pub(super) portfolio_strategic_allocation_present: bool,
    pub(super) portfolio_max_fee_impact: Option<Decimal>,
    pub(super) portfolio_fee_type: Option<String>,
    pub(super) portfolio_fee_amount: Option<Decimal>,
    pub(super) portfolio_fee_rate: Option<Decimal>,
    pub(super) portfolio_min_fee: Option<Decimal>,
    pub(super) portfolio_max_fee: Option<Decimal>,
    pub(super) asset_id: Option<Uuid>,
    pub(super) asset_symbol: Option<String>,
    pub(super) asset_class_override: Option<i16>,
    pub(super) asset_provider: Option<i16>,
    pub(super) asset_quantity: Option<Decimal>,
    pub(super) asset_target_weight: Option<Decimal>,
    pub(super) asset_manual_price: Option<Decimal>,
    pub(super) asset_max_fee_impact: Option<Decimal>,
    pub(super) asset_fee_type: Option<String>,
    pub(super) asset_fee_amount: Option<Decimal>,
    pub(super) asset_fee_rate: Option<Decimal>,
    pub(super) asset_min_fee: Option<Decimal>,
    pub(super) asset_max_fee: Option<Decimal>,
    pub(super) asset_average_buy_price: Option<Decimal>,
    pub(super) class_target_asset_class: Option<i16>,
    pub(super) class_target_weight: Option<Decimal>,
}

pub(super) struct V2AssetWithWeight {
    pub(super) asset: PortfolioAssetV2,
    pub(super) target_weight: Decimal,
}

/// Persisted Portfolio Asset values from which a v2 asset response is built.
pub(super) struct V2AssetResponseInput {
    pub(super) asset: AssetRef,
    pub(super) asset_class_override: Option<i16>,
    pub(super) quantity: Decimal,
    pub(super) target_weight: Decimal,
    pub(super) manual_price: Option<Decimal>,
    pub(super) max_fee_impact: Option<Decimal>,
    pub(super) fee_type: Option<String>,
    pub(super) fee_amount: Option<Decimal>,
    pub(super) fee_rate: Option<Decimal>,
    pub(super) min_fee: Option<Decimal>,
    pub(super) max_fee: Option<Decimal>,
    pub(super) average_buy_price: Option<Decimal>,
}

pub(super) fn corrupt_data(message: impl Into<String>) -> PortfolioRepositoryError {
    PortfolioRepositoryError::CorruptData(message.into())
}

fn stored_percentage_is_valid(value: Decimal) -> bool {
    (Decimal::ZERO..=Decimal::from(100)).contains(&value)
}

pub(super) fn db_asset_class(class: AppAssetClass) -> DbAssetClass {
    match class {
        AppAssetClass::Other => DbAssetClass::Other,
        AppAssetClass::Equity => DbAssetClass::Equity,
        AppAssetClass::Bond => DbAssetClass::Bond,
        AppAssetClass::Cash => DbAssetClass::Cash,
        AppAssetClass::Crypto => DbAssetClass::Crypto,
        AppAssetClass::Commodity => DbAssetClass::Commodity,
    }
}

pub(super) fn app_asset_class(class: DbAssetClass) -> AppAssetClass {
    match class {
        DbAssetClass::Other => AppAssetClass::Other,
        DbAssetClass::Equity => AppAssetClass::Equity,
        DbAssetClass::Bond => AppAssetClass::Bond,
        DbAssetClass::Cash => AppAssetClass::Cash,
        DbAssetClass::Crypto => AppAssetClass::Crypto,
        DbAssetClass::Commodity => AppAssetClass::Commodity,
    }
}

pub(super) fn db_provider(provider: AppProvider) -> Result<DbProvider> {
    match provider {
        AppProvider::YF => Ok(DbProvider::YF),
        AppProvider::Kraken => Ok(DbProvider::Kraken),
        AppProvider::Unsupported(value) => {
            Err(PortfolioRepositoryError::UnsupportedProvider(value))
        }
    }
}

fn app_provider(provider: DbProvider) -> AppProvider {
    match provider {
        DbProvider::YF => AppProvider::YF,
        DbProvider::Kraken => AppProvider::Kraken,
    }
}

pub(super) fn v2_mode(mode: AppAllocationMode) -> DbAllocationMode {
    match mode {
        AppAllocationMode::Simple => DbAllocationMode::Simple,
        AppAllocationMode::Strategic => DbAllocationMode::Strategic,
    }
}

fn asset_class_rank(class: AppAssetClass) -> i16 {
    i16::from(db_asset_class(class))
}

fn v2_fee_response(
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

pub(super) fn v2_asset_response(input: V2AssetResponseInput) -> Result<V2AssetWithWeight> {
    let asset_class_override = input
        .asset_class_override
        .map(|value| {
            DbAssetClass::try_from(value)
                .map(app_asset_class)
                .map_err(|_| corrupt_data("Portfolio Asset has an invalid aclassOverride"))
        })
        .transpose()?;
    let fees = v2_fee_response(
        input.max_fee_impact,
        input.fee_type,
        input.fee_amount,
        input.fee_rate,
        input.min_fee,
        input.max_fee,
    )?;
    Ok(V2AssetWithWeight {
        asset: PortfolioAssetV2 {
            asset: input.asset,
            asset_class_override,
            quantity: input.quantity,
            target_weight: None,
            manual_price: input.manual_price,
            average_buy_price: input.average_buy_price,
            fees,
        },
        target_weight: input.target_weight,
    })
}

pub(super) fn v2_portfolio_response(
    portfolio: PortfolioRow,
    mut assets: Vec<V2AssetWithWeight>,
    mut class_targets: Vec<StrategicAssetClassTargetV2>,
) -> Result<PortfolioV2> {
    assets.sort_by(|a, b| {
        (a.asset.asset.provider.as_str(), &a.asset.asset.symbol)
            .cmp(&(b.asset.asset.provider.as_str(), &b.asset.asset.symbol))
    });
    class_targets.sort_by_key(|target| asset_class_rank(target.asset_class));
    let allocation_mode = match portfolio.allocation_mode {
        DbAllocationMode::Simple => AppAllocationMode::Simple,
        DbAllocationMode::Strategic => AppAllocationMode::Strategic,
    };
    let asset_responses = assets
        .iter_mut()
        .map(|asset| {
            asset.asset.target_weight =
                (allocation_mode == AppAllocationMode::Simple).then_some(asset.target_weight);
            asset.asset.clone()
        })
        .collect();
    let strategic_allocation =
        portfolio
            .strategic_allocation_present
            .then(|| StrategicAllocationV2 {
                asset_class_targets: if allocation_mode == AppAllocationMode::Strategic {
                    class_targets
                } else {
                    Vec::new()
                },
                asset_targets: if allocation_mode == AppAllocationMode::Strategic {
                    assets
                        .iter()
                        .map(|asset| StrategicAssetTargetV2 {
                            asset: asset.asset.asset.clone(),
                            relative_target_weight: asset.target_weight,
                        })
                        .collect()
                } else {
                    Vec::new()
                },
            });

    Ok(PortfolioV2 {
        id: portfolio.id,
        name: portfolio.name,
        quote_currency: portfolio.currency,
        allocation_mode,
        drift_band: portfolio.drift_band,
        fees: v2_fee_response(
            portfolio.max_fee_impact,
            portfolio.fee_type,
            portfolio.fee_amount,
            portfolio.fee_rate,
            portfolio.min_fee,
            portfolio.max_fee,
        )?,
        assets: asset_responses,
        strategic_allocation,
        last_updated_at: portfolio.last_updated_at,
    })
}

fn v2_portfolio_row(snapshot: &V2SnapshotRow) -> PortfolioRow {
    PortfolioRow {
        id: snapshot.portfolio_id,
        user_id: snapshot.portfolio_user_id,
        name: snapshot.portfolio_name.clone(),
        currency: snapshot.portfolio_currency.clone(),
        deleted: snapshot.portfolio_deleted,
        last_updated_at: snapshot.portfolio_last_updated_at,
        allocation_mode: snapshot.portfolio_allocation_mode,
        drift_band: snapshot.portfolio_drift_band,
        strategic_allocation_present: snapshot.portfolio_strategic_allocation_present,
        max_fee_impact: snapshot.portfolio_max_fee_impact,
        fee_type: snapshot.portfolio_fee_type.clone(),
        fee_amount: snapshot.portfolio_fee_amount,
        fee_rate: snapshot.portfolio_fee_rate,
        min_fee: snapshot.portfolio_min_fee,
        max_fee: snapshot.portfolio_max_fee,
        created_at: snapshot.portfolio_created_at,
        updated_at: snapshot.portfolio_updated_at,
    }
}

fn v2_snapshot_portfolios(rows: Vec<V2SnapshotRow>) -> Result<Vec<PortfolioV2>> {
    let mut portfolios = Vec::new();
    let mut portfolio: Option<PortfolioRow> = None;
    let mut assets = Vec::new();
    let mut asset_ids = std::collections::HashSet::new();
    let mut class_targets = Vec::new();
    let mut class_ids = std::collections::HashSet::new();
    let mut portfolio_id = None;

    for row in rows {
        if portfolio_id != Some(row.portfolio_id) {
            if let Some(previous) = portfolio.take() {
                portfolios.push(v2_portfolio_response(previous, assets, class_targets)?);
            }
            portfolio_id = Some(row.portfolio_id);
            portfolio = Some(v2_portfolio_row(&row));
            assets = Vec::new();
            asset_ids.clear();
            class_targets = Vec::new();
            class_ids.clear();
        }

        if row.asset_id.is_none() && has_partial_asset_data(&row) {
            return Err(corrupt_data(
                "Portfolio snapshot contains asset fields without an asset id",
            ));
        }
        if let Some(asset_id) = row.asset_id {
            if !asset_ids.insert(asset_id) {
                if row.class_target_asset_class.is_none() {
                    return Err(corrupt_data(
                        "Portfolio snapshot contains a duplicate Portfolio Asset row",
                    ));
                }
            } else {
                let symbol = row
                    .asset_symbol
                    .ok_or_else(|| corrupt_data("Portfolio snapshot is missing an Asset symbol"))?;
                let provider = DbProvider::try_from(row.asset_provider.ok_or_else(|| {
                    corrupt_data("Portfolio snapshot is missing an Asset provider")
                })?)
                .map_err(|_| corrupt_data("Portfolio snapshot contains an invalid Provider"))?;
                assets.push(v2_asset_response(V2AssetResponseInput {
                    asset: AssetRef {
                        provider: app_provider(provider),
                        symbol,
                    },
                    asset_class_override: row.asset_class_override,
                    quantity: row.asset_quantity.ok_or_else(|| {
                        corrupt_data("Portfolio snapshot is missing an Asset quantity")
                    })?,
                    target_weight: row.asset_target_weight.unwrap_or_default(),
                    manual_price: row.asset_manual_price,
                    max_fee_impact: row.asset_max_fee_impact,
                    fee_type: row.asset_fee_type,
                    fee_amount: row.asset_fee_amount,
                    fee_rate: row.asset_fee_rate,
                    min_fee: row.asset_min_fee,
                    max_fee: row.asset_max_fee,
                    average_buy_price: row.asset_average_buy_price,
                })?);
            }
        }

        match (row.class_target_asset_class, row.class_target_weight) {
            (None, None) => {}
            (Some(asset_class), Some(target_weight)) if class_ids.insert(asset_class) => {
                let asset_class = DbAssetClass::try_from(asset_class).map_err(|_| {
                    corrupt_data("Portfolio snapshot contains an invalid Strategic Asset Class")
                })?;
                if !stored_percentage_is_valid(target_weight) {
                    return Err(corrupt_data(
                        "Portfolio snapshot contains an invalid Strategic target weight",
                    ));
                }
                class_targets.push(StrategicAssetClassTargetV2 {
                    asset_class: app_asset_class(asset_class),
                    target_weight,
                });
            }
            (Some(_), Some(_)) => {}
            _ => {
                return Err(corrupt_data(
                    "Portfolio snapshot contains a partial Strategic target",
                ));
            }
        }
    }
    if let Some(last) = portfolio {
        portfolios.push(v2_portfolio_response(last, assets, class_targets)?);
    }
    Ok(portfolios)
}

fn has_partial_asset_data(row: &V2SnapshotRow) -> bool {
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

/// Loads and decodes the complete v2 aggregate snapshot for one user.
pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<Vec<(PortfolioRow, crate::app::domain::portfolio::PortfolioV2)>> {
    let rows = query_as::<_, V2SnapshotRow>(
        "SELECT p.id AS portfolio_id, p.user_id AS portfolio_user_id,
                p.name AS portfolio_name, p.currency AS portfolio_currency,
                p.deleted AS portfolio_deleted,
                p.last_updated_at AS portfolio_last_updated_at,
                p.created_at AS portfolio_created_at,
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
                pa.average_buy_price AS asset_average_buy_price,
                class_target.asset_class AS class_target_asset_class,
                class_target.target_weight AS class_target_weight
         FROM portfolios AS p
         LEFT JOIN portfolio_asset AS pa ON pa.portfolio_id = p.id
         LEFT JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         LEFT JOIN portfolio_asset_allocation AS paa
           ON paa.portfolio_id = pa.portfolio_id
          AND paa.portfolio_asset_id = pa.id
         LEFT JOIN strategic_asset_class_targets AS class_target
           ON class_target.portfolio_id = p.id
         WHERE p.user_id = $1
         ORDER BY p.id, pa.id, class_target.asset_class
         FOR UPDATE OF p",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await?;
    let portfolio_rows = rows
        .iter()
        .fold(Vec::<PortfolioRow>::new(), |mut rows, row| {
            if rows
                .last()
                .is_none_or(|current| current.id != row.portfolio_id)
            {
                rows.push(v2_portfolio_row(row));
            }
            rows
        });
    let snapshots = v2_snapshot_portfolios(rows)?;
    Ok(portfolio_rows.into_iter().zip(snapshots).collect())
}
