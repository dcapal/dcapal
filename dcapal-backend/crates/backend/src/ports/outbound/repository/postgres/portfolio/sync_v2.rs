use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query, query_as, query_scalar};
use uuid::Uuid;

use crate::app::domain::portfolio::{
    PortfolioV2, StrategicAssetClassTargetV2, SyncPortfoliosV2Request, SyncPortfoliosV2Response,
};
use crate::ports::outbound::repository::portfolio::{
    OwnershipConflict, PortfolioRepositoryError, Result,
};
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, PortfolioRow,
    Provider as DbProvider,
};

use super::{
    AssetData, FeeFields, PortfolioAssetRecord, SqlxPortfolioRepository, sync_v2_snapshot,
};

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

    // Portfolio Asset identifiers are relationship-owned. Reject a request
    // that tries to attach a relationship belonging to another Portfolio
    // instead of surfacing a generic primary-key failure during the child
    // reconciliation below.
    let requested_asset_ids: Vec<Uuid> = portfolio_request
        .assets
        .iter()
        .map(|asset| asset.id)
        .collect();
    if !requested_asset_ids.is_empty() {
        let foreign_asset_id: Option<Uuid> = query_scalar(
            "SELECT id
             FROM portfolio_asset
             WHERE id = ANY($1) AND portfolio_id <> $2
             LIMIT 1",
        )
        .bind(&requested_asset_ids)
        .bind(portfolio_request.id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(foreign_asset_id) = foreign_asset_id {
            return Err(PortfolioRepositoryError::OwnershipConflict(
                OwnershipConflict::PortfolioAsset {
                    id: foreign_asset_id,
                },
            ));
        }
    }

    let existing_assets: Vec<(Uuid, Uuid)> = query_as(
        "SELECT id, assets_data_id
         FROM portfolio_asset
         WHERE portfolio_id = $1
         ORDER BY id
         FOR UPDATE",
    )
    .bind(portfolio_request.id)
    .fetch_all(&mut **tx)
    .await?;

    // Resolve all shared identities before changing relationships. Explicit
    // relationship IDs take precedence over identity fallbacks so a client
    // can reassign one row while retaining another row that used its former
    // identity.
    let existing_by_id: HashMap<Uuid, Uuid> = existing_assets.iter().copied().collect();
    let explicit_asset_ids: HashSet<Uuid> = portfolio_request
        .assets
        .iter()
        .filter_map(|asset| existing_by_id.contains_key(&asset.id).then_some(asset.id))
        .collect();
    let mut resolved_shared_assets = Vec::with_capacity(portfolio_request.assets.len());
    let mut relationship_ids = Vec::with_capacity(portfolio_request.assets.len());
    for asset in &portfolio_request.assets {
        let provider = sync_v2_snapshot::db_provider(asset.asset.provider.clone())?;
        let asset_class = sync_v2_snapshot::db_asset_class(asset.asset.asset_class);
        let shared_asset = SqlxPortfolioRepository::upsert_asset_data(
            tx,
            &AssetData::candidate(
                asset.asset.symbol.clone(),
                asset.asset.name.clone(),
                asset_class,
                asset.asset.currency.clone(),
                provider,
            ),
        )
        .await?;
        let existing_by_shared = existing_assets
            .iter()
            .find(|(_, assets_data_id)| *assets_data_id == shared_asset.id)
            .map(|(id, _)| *id);
        let relationship_id = existing_by_id
            .contains_key(&asset.id)
            .then_some(asset.id)
            .or_else(|| existing_by_shared.filter(|id| !explicit_asset_ids.contains(id)))
            .unwrap_or(asset.id);
        resolved_shared_assets.push(shared_asset);
        relationship_ids.push(relationship_id);
    }

    let retained_asset_ids: HashSet<Uuid> = relationship_ids.iter().copied().collect();
    for (existing_id, _) in &existing_assets {
        if !retained_asset_ids.contains(existing_id) {
            query("DELETE FROM portfolio_asset WHERE id = $1")
                .bind(existing_id)
                .execute(&mut **tx)
                .await?;
        }
    }

    // Park identities that are currently owned by another retained row's
    // target. This gives each reassignment a free identity slot, so even a
    // two-row (or larger) identity cycle cannot violate the unique
    // `(portfolio_id, assets_data_id)` constraint mid-loop.
    let mut relationships_needing_parking = HashSet::new();
    for (relationship_id, shared_asset) in relationship_ids.iter().zip(&resolved_shared_assets) {
        let Some(current_shared_id) = existing_by_id.get(relationship_id) else {
            continue;
        };
        if current_shared_id == &shared_asset.id {
            continue;
        }
        if let Some((owner_id, _)) = existing_assets
            .iter()
            .find(|(_, assets_data_id)| *assets_data_id == shared_asset.id)
            && retained_asset_ids.contains(owner_id)
        {
            relationships_needing_parking.insert(*owner_id);
        }
    }
    let mut placeholder_ids = Vec::new();
    for relationship_id in &relationships_needing_parking {
        let placeholder = SqlxPortfolioRepository::upsert_asset_data(
            tx,
            &AssetData::candidate(
                format!("__DCAPAL_RECONCILE_{}", Uuid::now_v7().simple()),
                "reconciliation placeholder".to_string(),
                DbAssetClass::Other,
                "EUR".to_string(),
                DbProvider::YF,
            ),
        )
        .await?;
        query("UPDATE portfolio_asset SET assets_data_id = $2 WHERE id = $1")
            .bind(relationship_id)
            .bind(placeholder.id)
            .execute(&mut **tx)
            .await?;
        placeholder_ids.push(placeholder.id);
    }

    let relative_targets: HashMap<Uuid, Decimal> = portfolio_request
        .strategic_allocation
        .as_ref()
        .map(|allocation| {
            allocation
                .asset_targets
                .iter()
                .map(|target| (target.portfolio_asset_id, target.relative_target_weight))
                .collect()
        })
        .unwrap_or_default();
    let mut updated_assets = Vec::with_capacity(portfolio_request.assets.len());

    for (index, asset) in portfolio_request.assets.iter().enumerate() {
        let shared_asset = resolved_shared_assets[index].clone();
        let requested_override = asset
            .asset_class_override
            .map(sync_v2_snapshot::db_asset_class)
            .filter(|class| *class != shared_asset.asset_class)
            .map(i16::from);
        let relationship_id = relationship_ids[index];

        let target_weight = match mode {
            DbAllocationMode::Simple => asset.target_weight.unwrap_or_default(),
            DbAllocationMode::Strategic => {
                relative_targets.get(&asset.id).copied().unwrap_or_default()
            }
        };
        let fee_fields = FeeFields::from_fees(asset.fees.as_ref());
        let now = Utc::now();

        let relationship = if existing_by_id.contains_key(&relationship_id) {
            query_as::<_, PortfolioAssetRecord>(
                "UPDATE portfolio_asset
                 SET assets_data_id = $2, asset_class_override = $3,
                     quantity = $4, manual_price = $5, average_buy_price = $6,
                     max_fee_impact = $7, fee_type = $8, fee_amount = $9,
                     fee_rate = $10, min_fee = $11, max_fee = $12, updated_at = $13
                 WHERE id = $1
                 RETURNING id, portfolio_id, assets_data_id, asset_class_override,
                           quantity, manual_price, max_fee_impact, fee_type,
                           fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                           created_at, updated_at",
            )
            .bind(relationship_id)
            .bind(shared_asset.id)
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
            .await?
        } else {
            query_as::<_, PortfolioAssetRecord>(
                "INSERT INTO portfolio_asset
                     (id, portfolio_id, assets_data_id, asset_class_override,
                      quantity, manual_price, average_buy_price, max_fee_impact,
                      fee_type, fee_amount, fee_rate, min_fee, max_fee,
                      created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 RETURNING id, portfolio_id, assets_data_id, asset_class_override,
                           quantity, manual_price, max_fee_impact, fee_type,
                           fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                           created_at, updated_at",
            )
            .bind(relationship_id)
            .bind(portfolio_request.id)
            .bind(shared_asset.id)
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
            .bind(now)
            .fetch_one(&mut **tx)
            .await?
        };

        let persisted_target_weight: Decimal = query_scalar(
            "INSERT INTO portfolio_asset_allocation
                 (portfolio_id, portfolio_asset_id, weight, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (portfolio_id, portfolio_asset_id) DO UPDATE
             SET weight = EXCLUDED.weight, updated_at = EXCLUDED.updated_at
             RETURNING weight",
        )
        .bind(portfolio_request.id)
        .bind(relationship_id)
        .bind(target_weight)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;
        updated_assets.push(sync_v2_snapshot::v2_asset_response(
            sync_v2_snapshot::V2AssetResponseInput {
                asset_id: relationship.id,
                symbol: shared_asset.symbol,
                name: shared_asset.name,
                asset_class: shared_asset.asset_class,
                asset_class_override: relationship.asset_class_override,
                currency: shared_asset.currency,
                provider: shared_asset.provider,
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

    if !placeholder_ids.is_empty() {
        query("DELETE FROM assets_data WHERE id = ANY($1)")
            .bind(&placeholder_ids)
            .execute(&mut **tx)
            .await?;
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
