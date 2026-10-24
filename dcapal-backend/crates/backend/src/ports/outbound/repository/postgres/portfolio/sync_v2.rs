use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Postgres, QueryBuilder, Transaction, query, query_as, query_scalar};
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio::{
        PortfolioV2, StrategicAssetClassTargetV2, SyncPortfoliosV2Request, SyncPortfoliosV2Response,
    },
};
use crate::ports::outbound::repository::portfolio::{
    OwnershipConflict, PortfolioRepositoryError, Result,
};
use crate::ports::outbound::repository::postgres::convert::{
    app_asset_class, db_allocation_mode, db_asset_class, unnest_refs,
};
use crate::ports::outbound::repository::postgres::types::{
    AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, PortfolioRow,
};

use super::{FeeFields, PortfolioAssetRecord, SqlxPortfolioRepository, sync_v2_snapshot};

/// Looks up the shared asset identifiers of the given asset references.
///
/// The assets were onboarded by the service before the transaction started, so
/// a missing reference is a caller bug rather than a user error.
async fn load_asset_ids<'a>(
    tx: &mut Transaction<'_, Postgres>,
    refs: impl IntoIterator<Item = &'a AssetRef>,
) -> Result<HashMap<&'a AssetRef, Uuid>> {
    let mut seen = HashSet::new();
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

/// Upserts every submitted Portfolio Asset relationship in one statement and
/// returns the persisted rows keyed by shared asset identifier.
///
/// A new relationship gets a fresh identifier; an existing one keeps its
/// identifier (and therefore its allocation row).
async fn upsert_relationships(
    tx: &mut Transaction<'_, Postgres>,
    portfolio: &PortfolioV2,
    asset_ids: &HashMap<&AssetRef, Uuid>,
    now: chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, PortfolioAssetRecord>> {
    if portfolio.assets.is_empty() {
        return Ok(HashMap::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        "INSERT INTO portfolio_asset
             (id, portfolio_id, assets_data_id, asset_class_override,
              quantity, manual_price, average_buy_price, max_fee_impact,
              fee_type, fee_amount, fee_rate, min_fee, max_fee,
              created_at, updated_at) ",
    );
    builder.push_values(&portfolio.assets, |mut row, asset| {
        let fee_fields = FeeFields::from_fees(asset.fees.as_ref());
        row.push_bind(Uuid::now_v7())
            .push_bind(portfolio.id)
            .push_bind(asset_ids[&asset.asset])
            .push_bind(
                asset
                    .asset_class_override
                    .map(db_asset_class)
                    .map(i16::from),
            )
            .push_bind(asset.quantity)
            .push_bind(asset.manual_price)
            .push_bind(asset.average_buy_price)
            .push_bind(fee_fields.max_fee_impact)
            .push_bind(fee_fields.fee_type)
            .push_bind(fee_fields.fee_amount)
            .push_bind(fee_fields.fee_rate)
            .push_bind(fee_fields.min_fee)
            .push_bind(fee_fields.max_fee)
            .push_bind(now)
            .push_bind(now);
    });
    builder.push(
        " ON CONFLICT (portfolio_id, assets_data_id) DO UPDATE
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
    );

    let rows = builder
        .build_query_as::<PortfolioAssetRecord>()
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.assets_data_id, row))
        .collect())
}

/// Upserts the target weight of every relationship in one statement and
/// returns the persisted weights keyed by relationship identifier.
///
/// Weights come from each asset's own target in simple mode and from the
/// relative Strategic target in strategic mode.
async fn upsert_allocations(
    tx: &mut Transaction<'_, Postgres>,
    portfolio: &PortfolioV2,
    mode: DbAllocationMode,
    relationships: &HashMap<Uuid, PortfolioAssetRecord>,
    asset_ids: &HashMap<&AssetRef, Uuid>,
    relative_targets: &HashMap<&AssetRef, Decimal>,
    now: chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, Decimal>> {
    if portfolio.assets.is_empty() {
        return Ok(HashMap::new());
    }

    let mut rows = Vec::with_capacity(portfolio.assets.len());
    for asset in &portfolio.assets {
        let relationship = relationships
            .get(&asset_ids[&asset.asset])
            .ok_or_else(|| sync_v2_snapshot::corrupt_data("relationship was not persisted"))?;
        let weight = match mode {
            DbAllocationMode::Simple => asset.target_weight.unwrap_or_default(),
            DbAllocationMode::Strategic => relative_targets
                .get(&asset.asset)
                .copied()
                .unwrap_or_default(),
        };
        rows.push((relationship.id, weight));
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        "INSERT INTO portfolio_asset_allocation
             (portfolio_id, portfolio_asset_id, weight, updated_at) ",
    );
    builder.push_values(rows, |mut row, (relationship_id, weight)| {
        row.push_bind(portfolio.id)
            .push_bind(relationship_id)
            .push_bind(weight)
            .push_bind(now);
    });
    builder.push(
        " ON CONFLICT (portfolio_id, portfolio_asset_id) DO UPDATE
          SET weight = EXCLUDED.weight, updated_at = EXCLUDED.updated_at
          RETURNING portfolio_asset_id, weight",
    );

    let persisted: Vec<(Uuid, Decimal)> = builder.build_query_as().fetch_all(&mut **tx).await?;
    Ok(persisted.into_iter().collect())
}

/// Replaces the Portfolio's Strategic Asset Class targets with the submitted
/// ones, using one delete and at most one insert.
async fn replace_class_targets(
    tx: &mut Transaction<'_, Postgres>,
    portfolio: &PortfolioV2,
) -> Result<Vec<StrategicAssetClassTargetV2>> {
    query("DELETE FROM strategic_asset_class_targets WHERE portfolio_id = $1")
        .bind(portfolio.id)
        .execute(&mut **tx)
        .await?;
    let targets = match &portfolio.strategic_allocation {
        Some(allocation) if !allocation.asset_class_targets.is_empty() => {
            &allocation.asset_class_targets
        }
        _ => return Ok(Vec::new()),
    };

    let mut builder = QueryBuilder::<Postgres>::new(
        "INSERT INTO strategic_asset_class_targets
             (portfolio_id, asset_class, target_weight, updated_at) ",
    );
    builder.push_values(targets, |mut row, target| {
        row.push_bind(portfolio.id)
            .push_bind(i16::from(db_asset_class(target.asset_class)))
            .push_bind(target.target_weight)
            .push("NOW()");
    });
    builder.push(" RETURNING asset_class, target_weight");
    let persisted: Vec<(i16, Decimal)> = builder.build_query_as().fetch_all(&mut **tx).await?;

    // Report the targets in submitted order regardless of RETURNING order.
    let persisted: HashMap<i16, Decimal> = persisted.into_iter().collect();
    targets
        .iter()
        .map(|target| {
            let code = i16::from(db_asset_class(target.asset_class));
            let target_weight = persisted.get(&code).copied().ok_or_else(|| {
                sync_v2_snapshot::corrupt_data("inserted Strategic Asset Class is missing")
            })?;
            let asset_class = DbAssetClass::try_from(code).map_err(|_| {
                sync_v2_snapshot::corrupt_data("inserted Strategic Asset Class is invalid")
            })?;
            Ok(StrategicAssetClassTargetV2 {
                asset_class: app_asset_class(asset_class),
                target_weight,
            })
        })
        .collect()
}

/// Locks the stored Portfolio row with the given identifier, if any.
async fn lock_portfolio(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<PortfolioRow>> {
    Ok(query_as::<_, PortfolioRow>(
        "SELECT id, user_id, name, currency, deleted, last_updated_at,
                allocation_mode, drift_band, strategic_allocation_present,
                max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                created_at, updated_at
         FROM portfolios
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?)
}

/// Reconciles one validated Portfolio aggregate inside the caller's transaction.
///
/// `snapshot_row` is the already-locked row of the user's own Portfolio when
/// the snapshot holds it. Otherwise the identifier is looked up and locked, so
/// that a row owned by another user is still detected.
async fn upsert_v2_portfolio_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    portfolio_request: &PortfolioV2,
    snapshot_row: Option<&PortfolioRow>,
    asset_ids: &HashMap<&AssetRef, Uuid>,
) -> Result<PortfolioV2> {
    let locked;
    let existing = match snapshot_row {
        Some(row) => Some(row),
        None => {
            locked = lock_portfolio(tx, portfolio_request.id).await?;
            locked.as_ref()
        }
    };

    if existing.is_some_and(|existing| existing.user_id != user_id || existing.deleted) {
        return Err(PortfolioRepositoryError::OwnershipConflict(
            OwnershipConflict::Portfolio {
                id: portfolio_request.id,
            },
        ));
    }

    let mode = db_allocation_mode(portfolio_request.allocation_mode);
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

    // Relationships are identified by (Portfolio, shared asset): drop the rows
    // whose asset is no longer submitted; the allocation rows cascade.
    let retained_asset_ids: Vec<Uuid> = portfolio_request
        .assets
        .iter()
        .map(|asset| asset_ids[&asset.asset])
        .collect();
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
    let now = Utc::now();
    let relationships = upsert_relationships(tx, portfolio_request, asset_ids, now).await?;
    let weights = upsert_allocations(
        tx,
        portfolio_request,
        mode,
        &relationships,
        asset_ids,
        &relative_targets,
        now,
    )
    .await?;

    let mut updated_assets = Vec::with_capacity(portfolio_request.assets.len());
    for asset in &portfolio_request.assets {
        let relationship = relationships
            .get(&asset_ids[&asset.asset])
            .ok_or_else(|| sync_v2_snapshot::corrupt_data("relationship was not persisted"))?;
        let persisted_target_weight = weights
            .get(&relationship.id)
            .copied()
            .ok_or_else(|| sync_v2_snapshot::corrupt_data("allocation was not persisted"))?;
        updated_assets.push(sync_v2_snapshot::v2_asset_response(
            sync_v2_snapshot::V2AssetResponseInput {
                asset: asset.asset.clone(),
                asset_class_override: relationship.asset_class_override,
                quantity: relationship.quantity,
                target_weight: persisted_target_weight,
                manual_price: relationship.manual_price,
                max_fee_impact: relationship.max_fee_impact,
                fee_type: relationship.fee_type.clone(),
                fee_amount: relationship.fee_amount,
                fee_rate: relationship.fee_rate,
                min_fee: relationship.min_fee,
                max_fee: relationship.max_fee,
                average_buy_price: relationship.average_buy_price,
            },
        )?);
    }

    let class_targets = replace_class_targets(tx, portfolio_request).await?;
    sync_v2_snapshot::v2_portfolio_response(persisted_portfolio, updated_assets, class_targets)
}

/// Tells whether a submitted Portfolio must be written: it is new, or it is
/// newer than the stored one, which must not be a tombstone.
fn is_written(portfolio: &PortfolioV2, current: Option<&PortfolioRow>) -> bool {
    current.is_none_or(|current| {
        !current.deleted && current.last_updated_at < portfolio.last_updated_at
    })
}

/// Executes a complete v2 synchronization in one serializable transaction.
pub(super) async fn sync(
    repository: &SqlxPortfolioRepository,
    user_id: Uuid,
    request: &SyncPortfoliosV2Request,
) -> Result<SyncPortfoliosV2Response> {
    let mut tx = repository.pool.begin().await?;
    query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;

    // Lock and load one complete snapshot before resolving incoming records.
    // Stored rows and responses are kept apart so that a response can be moved
    // into the result while the rows stay borrowed.
    let (current_rows, mut current_responses): (Vec<PortfolioRow>, Vec<Option<PortfolioV2>>) =
        sync_v2_snapshot::load(&mut tx, user_id)
            .await?
            .into_iter()
            .map(|(row, response)| (row, Some(response)))
            .unzip();
    let current_by_id: HashMap<Uuid, usize> = current_rows
        .iter()
        .enumerate()
        .map(|(position, row)| (row.id, position))
        .collect();
    let incoming_ids: HashSet<Uuid> = request
        .portfolios
        .iter()
        .map(|portfolio| portfolio.id)
        .collect();
    let requested_deleted_ids: HashSet<Uuid> = request.deleted_portfolios.iter().copied().collect();

    // Only Portfolios that are not tombstoned and not older than the stored
    // ones are written, so only their assets need to be looked up. The
    // references are collected before awaiting: a pending iterator over
    // borrowed closures would make the future unusable inside Axum handlers.
    let referenced: Vec<&AssetRef> = request
        .portfolios
        .iter()
        .filter(|portfolio| {
            is_written(
                portfolio,
                current_by_id
                    .get(&portfolio.id)
                    .map(|&position| &current_rows[position]),
            )
        })
        .flat_map(|portfolio| portfolio.assets.iter().map(|asset| &asset.asset))
        .collect();
    let asset_ids = load_asset_ids(&mut tx, referenced).await?;

    let mut updated_portfolios = Vec::new();
    let mut deleted_portfolios = Vec::new();
    for portfolio in &request.portfolios {
        let position = current_by_id.get(&portfolio.id).copied();
        let current = position.map(|position| &current_rows[position]);
        if !is_written(portfolio, current) {
            let position = position.expect("only stored Portfolios are skipped");
            if current_rows[position].deleted {
                deleted_portfolios.push(portfolio.id);
            } else {
                updated_portfolios.extend(current_responses[position].take());
            }
            continue;
        }
        updated_portfolios
            .push(upsert_v2_portfolio_tx(&mut tx, user_id, portfolio, current, &asset_ids).await?);
    }

    // Return server-side aggregates omitted by the client.
    for (current, response) in current_rows.iter().zip(current_responses) {
        if incoming_ids.contains(&current.id) || requested_deleted_ids.contains(&current.id) {
            continue;
        }
        if current.deleted {
            deleted_portfolios.push(current.id);
        } else {
            updated_portfolios.extend(response);
        }
    }

    let mut acknowledged: HashSet<Uuid> = deleted_portfolios.iter().copied().collect();
    for deleted_id in &request.deleted_portfolios {
        tombstone_portfolio(
            &mut tx,
            user_id,
            *deleted_id,
            current_by_id.contains_key(deleted_id),
        )
        .await?;
        if acknowledged.insert(*deleted_id) {
            deleted_portfolios.push(*deleted_id);
        }
    }

    tx.commit().await?;
    Ok(SyncPortfoliosV2Response {
        updated_portfolios,
        deleted_portfolios,
    })
}

/// Turns the Portfolio with the given identifier into a tombstone of `user_id`.
///
/// `owned` tells that the user's snapshot already holds the Portfolio, so its
/// owner is known. Otherwise the stored owner is looked up: another user's
/// Portfolio is a conflict and an unknown identifier gets a new tombstone row.
async fn tombstone_portfolio(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    id: Uuid,
    owned: bool,
) -> Result<()> {
    let exists = if owned {
        true
    } else {
        let owner: Option<Uuid> = query_scalar("SELECT user_id FROM portfolios WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
        if owner.is_some_and(|owner| owner != user_id) {
            return Err(PortfolioRepositoryError::OwnershipConflict(
                OwnershipConflict::Portfolio { id },
            ));
        }
        owner.is_some()
    };

    if exists {
        query("UPDATE portfolios SET deleted = TRUE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    } else {
        query(
            "INSERT INTO portfolios
                 (id, user_id, name, currency, deleted, last_updated_at,
                  allocation_mode, strategic_allocation_present)
             VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7)",
        )
        .bind(id)
        .bind(user_id)
        .bind("")
        .bind("")
        .bind(true)
        .bind(i16::from(DbAllocationMode::Simple))
        .bind(false)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
