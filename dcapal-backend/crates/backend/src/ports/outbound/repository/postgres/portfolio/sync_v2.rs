use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query, query_as, query_scalar};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio::{
        PortfolioAssetV2, PortfolioV2, StrategicAssetClassTargetV2, SyncPortfoliosV2Request,
        SyncPortfoliosV2Response,
    },
};
use crate::ports::outbound::repository::portfolio::{
    OwnershipConflict, PortfolioRepositoryError, Result,
};
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, PortfolioRow,
};

use super::{FeeFields, PortfolioAssetRecord, SqlxPortfolioRepository, sync_v2_snapshot};

/// Looks up the shared asset identifiers of every asset referenced by a Portfolio.
///
/// The assets were onboarded by the service before the transaction started, so
/// a missing reference is a caller bug rather than a user error.
async fn load_asset_ids(
    tx: &mut Transaction<'_, Postgres>,
    assets: &[PortfolioAssetV2],
) -> Result<HashMap<AssetRef, Uuid>> {
    let mut providers = Vec::with_capacity(assets.len());
    let mut symbols = Vec::with_capacity(assets.len());
    for asset in assets {
        providers.push(i16::from(sync_v2_snapshot::db_provider(
            asset.asset.provider.clone(),
        )?));
        symbols.push(asset.asset.symbol.clone());
    }
    let rows: Vec<(Uuid, i16, String)> = query_as(
        "SELECT ad.id, ad.provider, ad.symbol
         FROM assets_data AS ad
         JOIN UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
           ON ad.provider = wanted.provider AND ad.symbol = wanted.symbol",
    )
    .bind(providers)
    .bind(symbols)
    .fetch_all(&mut **tx)
    .await?;
    let stored: HashMap<(i16, String), Uuid> = rows
        .into_iter()
        .map(|(id, provider, symbol)| ((provider, symbol), id))
        .collect();

    let mut ids = HashMap::with_capacity(assets.len());
    for asset in assets {
        let provider = i16::from(sync_v2_snapshot::db_provider(asset.asset.provider.clone())?);
        let id = stored
            .get(&(provider, asset.asset.symbol.clone()))
            .ok_or_else(|| {
                PortfolioRepositoryError::AssetNotOnboarded(format!(
                    "{}:{}",
                    asset.asset.provider.as_str(),
                    asset.asset.symbol
                ))
            })?;
        ids.insert(asset.asset.clone(), *id);
    }
    Ok(ids)
}

/// Reconciles one validated Portfolio aggregate inside the caller's transaction.
async fn upsert_v2_portfolio_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    portfolio_request: &PortfolioV2,
) -> Result<PortfolioV2> {
    let existing = query_as::<_, PortfolioRow>(
        "SELECT id, user_id, name, currency, deleted, last_updated_at,
                allocation_mode, drift_band, strategic_allocation_present,
                max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                created_at, updated_at
         FROM portfolios
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(portfolio_request.id)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(existing) = &existing {
        if existing.user_id != user_id {
            return Err(PortfolioRepositoryError::OwnershipConflict(
                OwnershipConflict::Portfolio {
                    id: portfolio_request.id,
                },
            ));
        }
        if existing.deleted {
            return Err(PortfolioRepositoryError::OwnershipConflict(
                OwnershipConflict::Portfolio {
                    id: portfolio_request.id,
                },
            ));
        }
    }

    let mode = sync_v2_snapshot::v2_mode(portfolio_request.allocation_mode);
    let fee_fields = FeeFields::from_fees(portfolio_request.fees.as_ref());
    let persisted_portfolio = if existing.is_some() {
        query_as::<_, PortfolioRow>(
            "UPDATE portfolios
             SET name = $2, currency = $3, last_updated_at = $4,
                 allocation_mode = $5, drift_band = $6,
                 strategic_allocation_present = $7,
                 max_fee_impact = $8, fee_type = $9, fee_amount = $10,
                 fee_rate = $11, min_fee = $12, max_fee = $13,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, user_id, name, currency, deleted, last_updated_at,
                       allocation_mode, drift_band, strategic_allocation_present,
                       max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                       created_at, updated_at",
        )
        .bind(portfolio_request.id)
        .bind(&portfolio_request.name)
        .bind(&portfolio_request.quote_currency)
        .bind(portfolio_request.last_updated_at)
        .bind(i16::from(mode))
        .bind(portfolio_request.drift_band)
        .bind(portfolio_request.strategic_allocation.is_some())
        .bind(fee_fields.max_fee_impact)
        .bind(fee_fields.fee_type)
        .bind(fee_fields.fee_amount)
        .bind(fee_fields.fee_rate)
        .bind(fee_fields.min_fee)
        .bind(fee_fields.max_fee)
        .fetch_one(&mut **tx)
        .await?
    } else {
        query_as::<_, PortfolioRow>(
            "INSERT INTO portfolios
                 (id, user_id, name, currency, deleted, last_updated_at,
                  allocation_mode, drift_band, strategic_allocation_present,
                  max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee)
             VALUES ($1, $2, $3, $4, FALSE, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             RETURNING id, user_id, name, currency, deleted, last_updated_at,
                       allocation_mode, drift_band, strategic_allocation_present,
                       max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                       created_at, updated_at",
        )
        .bind(portfolio_request.id)
        .bind(user_id)
        .bind(&portfolio_request.name)
        .bind(&portfolio_request.quote_currency)
        .bind(portfolio_request.last_updated_at)
        .bind(i16::from(mode))
        .bind(portfolio_request.drift_band)
        .bind(portfolio_request.strategic_allocation.is_some())
        .bind(fee_fields.max_fee_impact)
        .bind(fee_fields.fee_type)
        .bind(fee_fields.fee_amount)
        .bind(fee_fields.fee_rate)
        .bind(fee_fields.min_fee)
        .bind(fee_fields.max_fee)
        .fetch_one(&mut **tx)
        .await?
    };

    let asset_ids = load_asset_ids(tx, &portfolio_request.assets).await?;

    // Relationships are identified by (Portfolio, shared asset): drop the rows
    // whose asset is no longer submitted; the allocation rows cascade.
    let retained_asset_ids: Vec<Uuid> = asset_ids.values().copied().collect();
    query(
        "DELETE FROM portfolio_asset
         WHERE portfolio_id = $1 AND assets_data_id <> ALL($2)",
    )
    .bind(portfolio_request.id)
    .bind(&retained_asset_ids)
    .execute(&mut **tx)
    .await?;

    let relative_targets: HashMap<&AssetRef, Decimal> = portfolio_request
        .strategic_allocation
        .as_ref()
        .map(|allocation| {
            allocation
                .asset_targets
                .iter()
                .map(|target| (&target.asset, target.relative_target_weight))
                .collect()
        })
        .unwrap_or_default();
    let mut updated_assets = Vec::with_capacity(portfolio_request.assets.len());

    for asset in &portfolio_request.assets {
        let assets_data_id = asset_ids[&asset.asset];
        let requested_override = asset
            .asset_class_override
            .map(sync_v2_snapshot::db_asset_class)
            .map(i16::from);
        let target_weight = match mode {
            DbAllocationMode::Simple => asset.target_weight.unwrap_or_default(),
            DbAllocationMode::Strategic => relative_targets
                .get(&asset.asset)
                .copied()
                .unwrap_or_default(),
        };
        let fee_fields = FeeFields::from_fees(asset.fees.as_ref());
        let now = Utc::now();

        // A new relationship gets a fresh identifier; an existing one keeps its
        // identifier (and therefore its allocation row).
        let relationship = query_as::<_, PortfolioAssetRecord>(
            "INSERT INTO portfolio_asset
                 (id, portfolio_id, assets_data_id, asset_class_override,
                  quantity, manual_price, average_buy_price, max_fee_impact,
                  fee_type, fee_amount, fee_rate, min_fee, max_fee,
                  created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)
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
             RETURNING id, portfolio_id, assets_data_id, asset_class_override,
                       quantity, manual_price, max_fee_impact, fee_type,
                       fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                       created_at, updated_at",
        )
        .bind(Uuid::now_v7())
        .bind(portfolio_request.id)
        .bind(assets_data_id)
        .bind(requested_override)
        .bind(asset.quantity)
        .bind(asset.manual_price)
        .bind(asset.average_buy_price)
        .bind(fee_fields.max_fee_impact)
        .bind(fee_fields.fee_type)
        .bind(fee_fields.fee_amount)
        .bind(fee_fields.fee_rate)
        .bind(fee_fields.min_fee)
        .bind(fee_fields.max_fee)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;

        let persisted_target_weight: Decimal = query_scalar(
            "INSERT INTO portfolio_asset_allocation
                 (portfolio_id, portfolio_asset_id, weight, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (portfolio_id, portfolio_asset_id) DO UPDATE
             SET weight = EXCLUDED.weight, updated_at = EXCLUDED.updated_at
             RETURNING weight",
        )
        .bind(portfolio_request.id)
        .bind(relationship.id)
        .bind(target_weight)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;
        updated_assets.push(sync_v2_snapshot::v2_asset_response(
            sync_v2_snapshot::V2AssetResponseInput {
                asset: asset.asset.clone(),
                asset_class_override: relationship.asset_class_override,
                quantity: relationship.quantity,
                target_weight: persisted_target_weight,
                manual_price: relationship.manual_price,
                max_fee_impact: relationship.max_fee_impact,
                fee_type: relationship.fee_type,
                fee_amount: relationship.fee_amount,
                fee_rate: relationship.fee_rate,
                min_fee: relationship.min_fee,
                max_fee: relationship.max_fee,
                average_buy_price: relationship.average_buy_price,
            },
        )?);
    }

    query("DELETE FROM strategic_asset_class_targets WHERE portfolio_id = $1")
        .bind(portfolio_request.id)
        .execute(&mut **tx)
        .await?;
    let mut class_targets = Vec::new();
    if let Some(allocation) = &portfolio_request.strategic_allocation {
        for target in &allocation.asset_class_targets {
            let (asset_class, target_weight): (i16, Decimal) = query_as(
                "INSERT INTO strategic_asset_class_targets
                     (portfolio_id, asset_class, target_weight, updated_at)
                 VALUES ($1, $2, $3, NOW())
                 RETURNING asset_class, target_weight",
            )
            .bind(portfolio_request.id)
            .bind(i16::from(sync_v2_snapshot::db_asset_class(
                target.asset_class,
            )))
            .bind(target.target_weight)
            .fetch_one(&mut **tx)
            .await?;
            class_targets.push(StrategicAssetClassTargetV2 {
                asset_class: sync_v2_snapshot::app_asset_class(
                    DbAssetClass::try_from(asset_class).map_err(|_| {
                        sync_v2_snapshot::corrupt_data("inserted Strategic Asset Class is invalid")
                    })?,
                ),
                target_weight,
            });
        }
    }
    sync_v2_snapshot::v2_portfolio_response(persisted_portfolio, updated_assets, class_targets)
}

/// Executes a complete v2 synchronization in one serializable transaction.
pub(super) async fn sync(
    repository: &SqlxPortfolioRepository,
    user_id: Uuid,
    request: SyncPortfoliosV2Request,
) -> Result<SyncPortfoliosV2Response> {
    let mut tx = repository.pool.begin().await?;
    query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;

    // Lock and load one complete snapshot before resolving incoming records.
    let current_snapshot = sync_v2_snapshot::load(&mut tx, user_id).await?;
    let current_by_id: HashMap<Uuid, (_, PortfolioV2)> = current_snapshot
        .iter()
        .cloned()
        .map(|(portfolio, response)| (portfolio.id, (portfolio, response)))
        .collect();
    let incoming_ids: HashSet<Uuid> = request
        .portfolios
        .iter()
        .map(|portfolio| portfolio.id)
        .collect();
    let requested_deleted_ids: HashSet<Uuid> = request.deleted_portfolios.iter().copied().collect();

    let mut updated_portfolios = Vec::new();
    let mut deleted_portfolios = Vec::new();
    for portfolio in &request.portfolios {
        match current_by_id.get(&portfolio.id) {
            Some((current, _)) if current.deleted => deleted_portfolios.push(portfolio.id),
            Some((current, response)) if current.last_updated_at >= portfolio.last_updated_at => {
                updated_portfolios.push(response.clone());
            }
            Some(_) | None => {
                updated_portfolios.push(upsert_v2_portfolio_tx(&mut tx, user_id, portfolio).await?);
            }
        }
    }

    // Return server-side aggregates omitted by the client.
    for (current, response) in &current_snapshot {
        if incoming_ids.contains(&current.id) || requested_deleted_ids.contains(&current.id) {
            continue;
        }
        if current.deleted {
            deleted_portfolios.push(current.id);
        } else {
            updated_portfolios.push(response.clone());
        }
    }

    for deleted_id in &request.deleted_portfolios {
        let owner: Option<Uuid> = query_scalar("SELECT user_id FROM portfolios WHERE id = $1")
            .bind(deleted_id)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(owner) = owner
            && owner != user_id
        {
            return Err(PortfolioRepositoryError::OwnershipConflict(
                OwnershipConflict::Portfolio { id: *deleted_id },
            ));
        }
        if owner.is_some() {
            query("UPDATE portfolios SET deleted = TRUE, updated_at = NOW() WHERE id = $1")
                .bind(deleted_id)
                .execute(&mut *tx)
                .await?;
        } else {
            query(
                "INSERT INTO portfolios
                     (id, user_id, name, currency, deleted, last_updated_at,
                      allocation_mode, strategic_allocation_present)
                 VALUES ($1, $2, '', '', TRUE, NOW(), $3, FALSE)",
            )
            .bind(deleted_id)
            .bind(user_id)
            .bind(i16::from(DbAllocationMode::Simple))
            .execute(&mut *tx)
            .await?;
        }
        if !deleted_portfolios.contains(deleted_id) {
            deleted_portfolios.push(*deleted_id);
        }
    }

    tx.commit().await?;
    Ok(SyncPortfoliosV2Response {
        updated_portfolios,
        deleted_portfolios,
    })
}
