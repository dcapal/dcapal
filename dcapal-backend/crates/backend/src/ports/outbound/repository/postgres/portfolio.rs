use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query, query_as};
use uuid::Uuid;

use crate::app::domain::{
    portfolio::{FeeStructure as AppFeeStructure, TransactionFees},
    portfolio_sync::{SyncPlan, SyncSnapshot},
};
use crate::ports::{
    inbound::rest::request::{PortfolioRequest, TransactionFeesRequest},
    outbound::repository::{
        portfolio::{PortfolioRepository, PortfolioRepositoryError, Result},
        postgres::types::{
            AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, PortfolioAssetRow,
            PortfolioRow, Provider as DbProvider,
        },
    },
};
mod sync_apply;
mod sync_snapshot;

/// PostgreSQL persistence for portfolios and their assets.
#[derive(Clone)]
pub struct SqlxPortfolioRepository {
    pool: sqlx::PgPool,
}

#[derive(Default)]
struct FeeFields {
    max_fee_impact: Option<Decimal>,
    fee_type: Option<String>,
    fee_amount: Option<Decimal>,
    fee_rate: Option<Decimal>,
    min_fee: Option<Decimal>,
    max_fee: Option<Decimal>,
}
impl FeeFields {
    /// Maps an optional application fee policy to its persisted columns.
    ///
    /// An absent policy leaves every fee column empty.
    fn from_fees(fees: Option<&TransactionFees>) -> Self {
        match fees {
            Some(fees) => match fees.fee_structure {
                AppFeeStructure::ZeroFee => Self {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    ..Default::default()
                },
                AppFeeStructure::Fixed { fee_amount } => Self {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    fee_amount: Some(fee_amount),
                    ..Default::default()
                },
                AppFeeStructure::Variable {
                    fee_rate,
                    min_fee,
                    max_fee,
                } => Self {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    fee_rate: Some(fee_rate),
                    min_fee,
                    max_fee,
                    ..Default::default()
                },
            },
            None => Self::default(),
        }
    }
}

/// Shared provider metadata returned by the asset-data upsert.
#[derive(Debug, Clone, sqlx::FromRow)]
struct AssetData {
    id: Uuid,
    symbol: String,
    name: String,
    #[sqlx(try_from = "i16")]
    asset_class: DbAssetClass,
    currency: String,
    #[sqlx(try_from = "i16")]
    provider: DbProvider,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
/// Portfolio-specific values returned by relationship writes.
#[derive(Debug, sqlx::FromRow)]
struct PortfolioAssetRecord {
    id: Uuid,
    portfolio_id: Uuid,
    assets_data_id: Uuid,
    asset_class_override: Option<i16>,
    quantity: Decimal,
    manual_price: Option<Decimal>,
    max_fee_impact: Option<Decimal>,
    fee_type: Option<String>,
    fee_amount: Option<Decimal>,
    fee_rate: Option<Decimal>,
    min_fee: Option<Decimal>,
    max_fee: Option<Decimal>,
    average_buy_price: Option<Decimal>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
struct PortfolioAssetInput {
    asset_data: AssetData,
    quantity: Decimal,
    target_weight: Decimal,
    manual_price: Option<Decimal>,
    average_buy_price: Option<Decimal>,
    fees: Option<TransactionFeesRequest>,
}
struct PortfolioInput {
    id: Uuid,
    name: String,
    quote_ccy: String,
    fees: Option<TransactionFeesRequest>,
    last_updated_at: crate::DateTime,
}
impl AssetData {
    fn candidate(
        symbol: String,
        name: String,
        asset_class: DbAssetClass,
        currency: String,
        provider: DbProvider,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            symbol: symbol.to_uppercase(),
            name,
            asset_class,
            currency,
            provider,
            created_at: now,
            updated_at: now,
        }
    }
}
impl PortfolioAssetRecord {
    fn candidate(
        portfolio_id: Uuid,
        assets_data_id: Uuid,
        asset_class_override: Option<DbAssetClass>,
        asset: &PortfolioAssetInput,
        fee_fields: &FeeFields,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            portfolio_id,
            assets_data_id,
            asset_class_override: asset_class_override.map(i16::from),
            quantity: asset.quantity,
            manual_price: asset.manual_price,
            max_fee_impact: fee_fields.max_fee_impact,
            fee_type: fee_fields.fee_type.clone(),
            fee_amount: fee_fields.fee_amount,
            fee_rate: fee_fields.fee_rate,
            min_fee: fee_fields.min_fee,
            max_fee: fee_fields.max_fee,
            average_buy_price: asset.average_buy_price,
            created_at: now,
            updated_at: now,
        }
    }
}
fn combine_asset_rows(
    asset_data: AssetData,
    relationship: PortfolioAssetRecord,
    target_weight: Decimal,
) -> PortfolioAssetRow {
    PortfolioAssetRow {
        id: relationship.id,
        portfolio_id: relationship.portfolio_id,
        assets_data_id: asset_data.id,
        symbol: asset_data.symbol,
        name: asset_data.name,
        asset_class: asset_data.asset_class,
        asset_class_override: relationship.asset_class_override,
        currency: asset_data.currency,
        provider: asset_data.provider,
        quantity: relationship.quantity,
        target_weight,
        manual_price: relationship.manual_price,
        max_fee_impact: relationship.max_fee_impact,
        fee_type: relationship.fee_type,
        fee_amount: relationship.fee_amount,
        fee_rate: relationship.fee_rate,
        min_fee: relationship.min_fee,
        max_fee: relationship.max_fee,
        average_buy_price: relationship.average_buy_price,
        created_at: relationship.created_at,
        updated_at: relationship.updated_at,
    }
}
/// Stores the target weight of one Portfolio Asset relationship, replacing any
/// previous weight.
async fn upsert_allocation(
    tx: &mut Transaction<'_, Postgres>,
    portfolio_id: Uuid,
    relationship_id: Uuid,
    weight: Decimal,
    updated_at: DateTime<Utc>,
) -> Result<()> {
    query(
        "INSERT INTO portfolio_asset_allocation
           (portfolio_id, portfolio_asset_id, weight, updated_at)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (portfolio_id, portfolio_asset_id) DO UPDATE
           SET weight = EXCLUDED.weight, updated_at = EXCLUDED.updated_at",
    )
    .bind(portfolio_id)
    .bind(relationship_id)
    .bind(weight)
    .bind(updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl SqlxPortfolioRepository {
    /// Creates a portfolio repository backed by the provided PostgreSQL pool.
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
    async fn upsert_asset_data(
        tx: &mut Transaction<'_, Postgres>,
        asset_data: &AssetData,
    ) -> Result<AssetData> {
        Ok(query_as::<_, AssetData>(
            "INSERT INTO assets_data
                 (id, provider, symbol, name, currency, asset_class, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (provider, symbol) DO UPDATE
                 SET symbol = EXCLUDED.symbol
             RETURNING id, symbol, name, asset_class, currency, provider,
                       created_at, updated_at",
        )
        .bind(asset_data.id)
        .bind(i16::from(asset_data.provider))
        .bind(&asset_data.symbol)
        .bind(&asset_data.name)
        .bind(&asset_data.currency)
        .bind(i16::from(asset_data.asset_class))
        .bind(asset_data.created_at)
        .bind(asset_data.updated_at)
        .fetch_one(&mut **tx)
        .await?)
    }

    async fn upsert_assets_transaction(
        tx: &mut Transaction<'_, Postgres>,
        portfolio_id: Uuid,
        assets: Vec<PortfolioAssetInput>,
    ) -> Result<Vec<PortfolioAssetRow>> {
        // Lock the current asset set so concurrent syncs cannot both reconcile it from stale data.
        let existing_assets = query_as::<_, PortfolioAssetRow>(
            "SELECT pa.id, pa.portfolio_id, pa.assets_data_id,
                    ad.symbol, ad.name, ad.asset_class, pa.asset_class_override,
                    ad.currency, ad.provider,
                    pa.quantity, COALESCE(paa.weight, 0) AS target_weight, pa.manual_price,
                    pa.max_fee_impact, pa.fee_type, pa.fee_amount,
                    pa.fee_rate, pa.min_fee, pa.max_fee, pa.average_buy_price,
                    pa.created_at, pa.updated_at
             FROM portfolio_asset AS pa
             JOIN assets_data AS ad ON ad.id = pa.assets_data_id
             LEFT JOIN portfolio_asset_allocation AS paa
               ON paa.portfolio_id = pa.portfolio_id
              AND paa.portfolio_asset_id = pa.id
             WHERE pa.portfolio_id = $1
             ORDER BY pa.id
             FOR UPDATE OF pa",
        )
        .bind(portfolio_id)
        .fetch_all(&mut **tx)
        .await?;
        let mut updated_assets = Vec::with_capacity(assets.len());
        for asset in &assets {
            let fee_fields = Self::extract_fee_fields(asset.fees.clone());
            let shared_asset = Self::upsert_asset_data(tx, &asset.asset_data).await?;
            let asset_class_override = (shared_asset.asset_class != asset.asset_data.asset_class)
                .then_some(asset.asset_data.asset_class);
            let existing_asset = existing_assets
                .iter()
                .find(|row| row.assets_data_id == shared_asset.id);
            let candidate = PortfolioAssetRecord::candidate(
                portfolio_id,
                shared_asset.id,
                asset_class_override,
                asset,
                &fee_fields,
            );

            let updated = if let Some(existing_asset) = existing_asset {
                let relationship = query_as::<_, PortfolioAssetRecord>(
                    "UPDATE portfolio_asset
                     SET assets_data_id = $2, asset_class_override = $3,
                         quantity = $4, manual_price = $5,
                         average_buy_price = $6, max_fee_impact = $7, fee_type = $8,
                         fee_amount = $9, fee_rate = $10, min_fee = $11, max_fee = $12,
                         updated_at = $13
                     WHERE id = $1
                     RETURNING id, portfolio_id, assets_data_id, asset_class_override,
                               quantity, manual_price, max_fee_impact, fee_type,
                               fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                               created_at, updated_at",
                )
                .bind(existing_asset.id)
                .bind(candidate.assets_data_id)
                .bind(candidate.asset_class_override)
                .bind(candidate.quantity)
                .bind(candidate.manual_price)
                .bind(candidate.average_buy_price)
                .bind(candidate.max_fee_impact)
                .bind(&candidate.fee_type)
                .bind(candidate.fee_amount)
                .bind(candidate.fee_rate)
                .bind(candidate.min_fee)
                .bind(candidate.max_fee)
                .bind(candidate.updated_at)
                .fetch_one(&mut **tx)
                .await?;

                upsert_allocation(
                    tx,
                    portfolio_id,
                    relationship.id,
                    asset.target_weight,
                    candidate.updated_at,
                )
                .await?;

                combine_asset_rows(shared_asset, relationship, asset.target_weight)
            } else {
                let relationship = query_as::<_, PortfolioAssetRecord>(
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
                .bind(candidate.id)
                .bind(candidate.portfolio_id)
                .bind(candidate.assets_data_id)
                .bind(candidate.asset_class_override)
                .bind(candidate.quantity)
                .bind(candidate.manual_price)
                .bind(candidate.average_buy_price)
                .bind(candidate.max_fee_impact)
                .bind(&candidate.fee_type)
                .bind(candidate.fee_amount)
                .bind(candidate.fee_rate)
                .bind(candidate.min_fee)
                .bind(candidate.max_fee)
                .bind(candidate.created_at)
                .bind(candidate.updated_at)
                .fetch_one(&mut **tx)
                .await?;

                upsert_allocation(
                    tx,
                    portfolio_id,
                    relationship.id,
                    asset.target_weight,
                    candidate.updated_at,
                )
                .await?;

                combine_asset_rows(shared_asset, relationship, asset.target_weight)
            };

            updated_assets.push(updated);
        }
        let current_asset_ids: Vec<Uuid> = updated_assets
            .iter()
            .map(|asset| asset.assets_data_id)
            .collect();
        for existing_asset in existing_assets {
            if !current_asset_ids.contains(&existing_asset.assets_data_id) {
                query("DELETE FROM portfolio_asset WHERE id = $1")
                    .bind(existing_asset.id)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        Ok(updated_assets)
    }
    #[allow(clippy::too_many_arguments)]
    async fn upsert_portfolio(
        &self,
        user_id: Uuid,
        portfolio: PortfolioInput,
        assets: Vec<PortfolioAssetInput>,
    ) -> Result<(PortfolioRow, Vec<PortfolioAssetRow>)> {
        let mut tx = self.pool.begin().await?;
        query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await?;

        let existing = query_as::<_, PortfolioRow>(
            "SELECT id, user_id, name, currency, deleted, last_updated_at,
                    allocation_mode, drift_band, strategic_allocation_present,
                    max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                    created_at, updated_at
             FROM portfolios
             WHERE id = $1
             FOR UPDATE",
        )
        .bind(portfolio.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing) = &existing
            && existing.user_id != user_id
        {
            return Err(PortfolioRepositoryError::CannotUpdate);
        }

        let fee_fields = Self::extract_fee_fields(portfolio.fees);
        let portfolio = if existing.is_some() {
            query_as::<_, PortfolioRow>(
                "UPDATE portfolios
                 SET name = $2, currency = $3, last_updated_at = $4,
                     allocation_mode = $5, strategic_allocation_present = FALSE,
                     max_fee_impact = $6, fee_type = $7, fee_amount = $8,
                     fee_rate = $9, min_fee = $10, max_fee = $11
                 WHERE id = $1
                 RETURNING id, user_id, name, currency, deleted, last_updated_at,
                           allocation_mode, drift_band, strategic_allocation_present,
                           max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                           created_at, updated_at",
            )
            .bind(portfolio.id)
            .bind(&portfolio.name)
            .bind(&portfolio.quote_ccy)
            .bind(portfolio.last_updated_at)
            .bind(i16::from(DbAllocationMode::Simple))
            .bind(fee_fields.max_fee_impact)
            .bind(fee_fields.fee_type)
            .bind(fee_fields.fee_amount)
            .bind(fee_fields.fee_rate)
            .bind(fee_fields.min_fee)
            .bind(fee_fields.max_fee)
            .fetch_one(&mut *tx)
            .await?
        } else {
            query_as::<_, PortfolioRow>(
                "INSERT INTO portfolios
                     (id, user_id, name, currency, deleted, last_updated_at,
                      max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee)
                 VALUES ($1, $2, $3, $4, FALSE, $5, $6, $7, $8, $9, $10, $11)
                 RETURNING id, user_id, name, currency, deleted, last_updated_at,
                           allocation_mode, drift_band, strategic_allocation_present,
                           max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                           created_at, updated_at",
            )
            .bind(portfolio.id)
            .bind(user_id)
            .bind(&portfolio.name)
            .bind(&portfolio.quote_ccy)
            .bind(portfolio.last_updated_at)
            .bind(fee_fields.max_fee_impact)
            .bind(fee_fields.fee_type)
            .bind(fee_fields.fee_amount)
            .bind(fee_fields.fee_rate)
            .bind(fee_fields.min_fee)
            .bind(fee_fields.max_fee)
            .fetch_one(&mut *tx)
            .await?
        };

        let assets = Self::upsert_assets_transaction(&mut tx, portfolio.id, assets).await?;

        // The legacy endpoint exposes only absolute Portfolio Asset targets;
        // keep a v2 Strategic state from changing that contract implicitly.
        query("DELETE FROM strategic_asset_class_targets WHERE portfolio_id = $1")
            .bind(portfolio.id)
            .execute(&mut *tx)
            .await?;

        // A sync must expose the portfolio and its asset set as one consistent change.
        tx.commit().await?;
        Ok((portfolio, assets))
    }
    fn extract_fee_fields(fees: Option<TransactionFeesRequest>) -> FeeFields {
        match fees {
            Some(fees) => FeeFields::from_fees(Some(&TransactionFees::from(fees))),
            None => FeeFields {
                fee_type: Some(AppFeeStructure::ZeroFee.to_string()),
                ..Default::default()
            },
        }
    }
}
#[async_trait]
impl PortfolioRepository for SqlxPortfolioRepository {
    async fn find_owners(&self, portfolio_ids: &[Uuid]) -> Result<HashMap<Uuid, Uuid>> {
        if portfolio_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let owners = query_as::<_, (Uuid, Uuid)>(
            "SELECT id, user_id
             FROM portfolios
             WHERE id = ANY($1)",
        )
        .bind(portfolio_ids.to_vec())
        .fetch_all(&self.pool)
        .await?;
        Ok(owners.into_iter().collect())
    }

    async fn get_user_portfolios_with_assets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(PortfolioRow, Vec<PortfolioAssetRow>)>> {
        let portfolios = query_as::<_, PortfolioRow>(
            "SELECT id, user_id, name, currency, deleted, last_updated_at,
                    allocation_mode, drift_band, strategic_allocation_present,
                    max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                    created_at, updated_at
             FROM portfolios
             WHERE user_id = $1
             ORDER BY id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        if portfolios.is_empty() {
            return Ok(Vec::new());
        }

        let portfolio_ids: Vec<Uuid> = portfolios.iter().map(|portfolio| portfolio.id).collect();
        let assets = query_as::<_, PortfolioAssetRow>(
            "SELECT pa.id, pa.portfolio_id, pa.assets_data_id,
                    ad.symbol, ad.name, ad.asset_class, pa.asset_class_override,
                    ad.currency, ad.provider,
                    pa.quantity,
                    CASE WHEN p.allocation_mode = $2
                         THEN COALESCE(paa.weight, 0)
                              * COALESCE(class_target.target_weight, 0) / 100
                         ELSE COALESCE(paa.weight, 0)
                    END AS target_weight,
                    pa.manual_price,
                    pa.max_fee_impact, pa.fee_type, pa.fee_amount,
                    pa.fee_rate, pa.min_fee, pa.max_fee, pa.average_buy_price,
                    pa.created_at, pa.updated_at
             FROM portfolio_asset AS pa
             JOIN portfolios AS p ON p.id = pa.portfolio_id
             JOIN assets_data AS ad ON ad.id = pa.assets_data_id
             LEFT JOIN portfolio_asset_allocation AS paa
               ON paa.portfolio_id = pa.portfolio_id
              AND paa.portfolio_asset_id = pa.id
             LEFT JOIN strategic_asset_class_targets AS class_target
               ON class_target.portfolio_id = pa.portfolio_id
              AND class_target.asset_class = COALESCE(pa.asset_class_override, ad.asset_class)
             WHERE pa.portfolio_id = ANY($1)
             ORDER BY pa.portfolio_id, pa.id",
        )
        .bind(&portfolio_ids)
        .bind(i16::from(DbAllocationMode::Strategic))
        .fetch_all(&self.pool)
        .await?;

        let mut assets_by_portfolio: HashMap<Uuid, Vec<PortfolioAssetRow>> = HashMap::new();
        for asset in assets {
            assets_by_portfolio
                .entry(asset.portfolio_id)
                .or_default()
                .push(asset);
        }

        Ok(portfolios
            .into_iter()
            .map(|portfolio| {
                let assets = assets_by_portfolio
                    .remove(&portfolio.id)
                    .unwrap_or_default();
                (portfolio, assets)
            })
            .collect())
    }

    async fn soft_delete(&self, user_id: Uuid, portfolio_id: Uuid) -> Result<()> {
        // Keep the ownership check in the write itself so this invariant survives other callers.
        query("UPDATE portfolios SET deleted = TRUE WHERE id = $1 AND user_id = $2")
            .bind(portfolio_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn upsert(
        &self,
        user_id: Uuid,
        portfolio_req: PortfolioRequest,
    ) -> Result<(PortfolioRow, Vec<PortfolioAssetRow>)> {
        let assets = portfolio_req
            .assets
            .into_iter()
            .map(|asset| {
                let provider = DbProvider::from_legacy(&asset.provider).ok_or_else(|| {
                    PortfolioRepositoryError::UnsupportedProvider(asset.provider.clone())
                })?;
                let asset_class = DbAssetClass::from_legacy(&asset.aclass);
                Ok(PortfolioAssetInput {
                    asset_data: AssetData::candidate(
                        asset.symbol,
                        asset.name,
                        asset_class,
                        asset.base_ccy,
                        provider,
                    ),
                    quantity: asset.qty,
                    target_weight: asset.target_weight,
                    manual_price: Some(asset.price),
                    average_buy_price: Some(asset.average_buy_price),
                    fees: asset.fees,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        self.upsert_portfolio(
            user_id,
            PortfolioInput {
                id: portfolio_req.id,
                name: portfolio_req.name,
                quote_ccy: portfolio_req.quote_ccy,
                fees: portfolio_req.fees,
                last_updated_at: portfolio_req.last_updated_at,
            },
            assets,
        )
        .await
    }

    async fn load_sync_snapshot(&self, user_id: Uuid) -> Result<SyncSnapshot> {
        self.load_snapshot(user_id).await
    }

    async fn apply_sync_plan(&self, user_id: Uuid, plan: &SyncPlan) -> Result<()> {
        self.apply_plan(user_id, plan).await
    }
}
