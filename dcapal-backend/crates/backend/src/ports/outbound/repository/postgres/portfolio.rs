use std::collections::HashMap;

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction, query, query_as};
use uuid::Uuid;

use crate::{
    error::{DcaError, Result},
    ports::{
        inbound::rest::{
            FeeStructure,
            request::{PortfolioAssetRequest, PortfolioRequest, TransactionFeesRequest},
        },
        outbound::repository::{
            portfolio::PortfolioRepository,
            postgres::types::{AssetClass, PortfolioAssetRow, PortfolioRow, Provider},
        },
    },
};

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

impl SqlxPortfolioRepository {
    /// Creates a portfolio repository backed by the provided PostgreSQL pool.
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    async fn upsert_assets_transaction(
        tx: &mut Transaction<'_, Postgres>,
        portfolio_id: Uuid,
        assets: Vec<PortfolioAssetRequest>,
    ) -> Result<Vec<PortfolioAssetRow>> {
        // Keep persisted identity case-insensitive by normalizing every client symbol first.
        let assets: Vec<PortfolioAssetRequest> = assets
            .into_iter()
            .map(|mut asset| {
                asset.symbol = asset.symbol.to_uppercase();
                asset
            })
            .collect();

        // Lock the current asset set so concurrent syncs cannot both reconcile it from stale data.
        let existing_assets = query_as::<_, PortfolioAssetRow>(
            "SELECT id, symbol, portfolio_id, name,
                    CASE asset_class
                        WHEN 0 THEN 'OTHER' WHEN 1 THEN 'EQUITY' WHEN 2 THEN 'BOND'
                        WHEN 3 THEN 'CASH' WHEN 4 THEN 'CRYPTO' WHEN 5 THEN 'COMMODITY'
                        ELSE 'OTHER'
                    END AS asset_class,
                    currency,
                    CASE provider WHEN 1 THEN 'Kraken' WHEN 2 THEN 'YF' END AS provider,
                    quantity, target_weight, manual_price AS price, max_fee_impact, fee_type, fee_amount,
                    fee_rate, min_fee, max_fee, average_buy_price, created_at, updated_at
             FROM portfolio_asset
             WHERE portfolio_id = $1
             ORDER BY id
             FOR UPDATE",
        )
        .bind(portfolio_id)
        .fetch_all(&mut **tx)
        .await?;

        let mut updated_assets = Vec::with_capacity(assets.len());

        for asset in &assets {
            let existing_asset = existing_assets
                .iter()
                .find(|row| row.symbol == asset.symbol);
            let fee_fields = Self::extract_fee_fields(asset.fees.clone());
            let provider = Provider::from_legacy(&asset.provider).ok_or_else(|| {
                DcaError::BadRequest(format!(
                    "Unsupported Portfolio Asset provider: {}",
                    asset.provider
                ))
            })?;
            let asset_class = AssetClass::from_legacy(&asset.aclass);

            let updated = if let Some(existing_asset) = existing_asset {
                query_as::<_, PortfolioAssetRow>(
                    "UPDATE portfolio_asset
                    SET symbol = $2, name = $3, asset_class = $4, currency = $5,
                         provider = $6, quantity = $7, target_weight = $8, manual_price = $9,
                         average_buy_price = $10, max_fee_impact = $11, fee_type = $12,
                         fee_amount = $13, fee_rate = $14, min_fee = $15, max_fee = $16
                     WHERE id = $1
                     RETURNING id, symbol, portfolio_id, name,
                               CASE asset_class
                                   WHEN 0 THEN 'OTHER' WHEN 1 THEN 'EQUITY' WHEN 2 THEN 'BOND'
                                   WHEN 3 THEN 'CASH' WHEN 4 THEN 'CRYPTO' WHEN 5 THEN 'COMMODITY'
                                   ELSE 'OTHER'
                               END AS asset_class,
                               currency, CASE provider WHEN 1 THEN 'Kraken' WHEN 2 THEN 'YF' END AS provider,
                               quantity, target_weight, manual_price AS price, max_fee_impact, fee_type,
                               fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                               created_at, updated_at",
                )
                .bind(existing_asset.id)
                .bind(&asset.symbol)
                .bind(&asset.name)
                .bind(i16::from(asset_class))
                .bind(&asset.base_ccy)
                .bind(i16::from(provider))
                .bind(asset.qty)
                .bind(asset.target_weight)
                .bind(asset.price)
                .bind(Some(asset.average_buy_price))
                .bind(fee_fields.max_fee_impact)
                .bind(fee_fields.fee_type)
                .bind(fee_fields.fee_amount)
                .bind(fee_fields.fee_rate)
                .bind(fee_fields.min_fee)
                .bind(fee_fields.max_fee)
                .fetch_one(&mut **tx)
                .await?
            } else {
                query_as::<_, PortfolioAssetRow>(
                    "INSERT INTO portfolio_asset
                         (id, symbol, portfolio_id, name, asset_class, currency, provider,
                          quantity, target_weight, manual_price, average_buy_price, max_fee_impact,
                          fee_type, fee_amount, fee_rate, min_fee, max_fee)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                             $15, $16, $17)
                     RETURNING id, symbol, portfolio_id, name,
                               CASE asset_class
                                   WHEN 0 THEN 'OTHER' WHEN 1 THEN 'EQUITY' WHEN 2 THEN 'BOND'
                                   WHEN 3 THEN 'CASH' WHEN 4 THEN 'CRYPTO' WHEN 5 THEN 'COMMODITY'
                                   ELSE 'OTHER'
                               END AS asset_class,
                               currency, CASE provider WHEN 1 THEN 'Kraken' WHEN 2 THEN 'YF' END AS provider,
                               quantity, target_weight, manual_price AS price, max_fee_impact, fee_type,
                               fee_amount, fee_rate, min_fee, max_fee, average_buy_price,
                               created_at, updated_at",
                )
                .bind(Uuid::new_v4())
                .bind(&asset.symbol)
                .bind(portfolio_id)
                .bind(&asset.name)
                .bind(i16::from(asset_class))
                .bind(&asset.base_ccy)
                .bind(i16::from(provider))
                .bind(asset.qty)
                .bind(asset.target_weight)
                .bind(asset.price)
                .bind(Some(asset.average_buy_price))
                .bind(fee_fields.max_fee_impact)
                .bind(fee_fields.fee_type)
                .bind(fee_fields.fee_amount)
                .bind(fee_fields.fee_rate)
                .bind(fee_fields.min_fee)
                .bind(fee_fields.max_fee)
                .fetch_one(&mut **tx)
                .await?
            };

            updated_assets.push(updated);
        }

        let current_symbols: Vec<&str> = assets.iter().map(|asset| asset.symbol.as_str()).collect();
        for existing_asset in existing_assets {
            if !current_symbols.contains(&existing_asset.symbol.as_str()) {
                query("DELETE FROM portfolio_asset WHERE id = $1")
                    .bind(existing_asset.id)
                    .execute(&mut **tx)
                    .await?;
            }
        }

        Ok(updated_assets)
    }

    fn extract_fee_fields(fees: Option<TransactionFeesRequest>) -> FeeFields {
        match fees {
            Some(fees) => match fees.fee_structure {
                FeeStructure::ZeroFee => FeeFields {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    ..Default::default()
                },
                FeeStructure::Fixed { fee_amount } => FeeFields {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    fee_amount: Some(fee_amount),
                    ..Default::default()
                },
                FeeStructure::Variable {
                    fee_rate,
                    min_fee,
                    max_fee,
                } => FeeFields {
                    max_fee_impact: fees.max_fee_impact,
                    fee_type: Some(fees.fee_structure.to_string()),
                    fee_rate: Some(fee_rate),
                    min_fee: Some(min_fee),
                    max_fee,
                    ..Default::default()
                },
            },
            None => FeeFields {
                fee_type: Some(FeeStructure::ZeroFee.to_string()),
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl PortfolioRepository for SqlxPortfolioRepository {
    async fn get_user_portfolios_with_assets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(PortfolioRow, Vec<PortfolioAssetRow>)>> {
        let portfolios = query_as::<_, PortfolioRow>(
            "SELECT id, user_id, name, currency, deleted, last_updated_at,
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
            "SELECT id, symbol, portfolio_id, name,
                    CASE asset_class
                        WHEN 0 THEN 'OTHER' WHEN 1 THEN 'EQUITY' WHEN 2 THEN 'BOND'
                        WHEN 3 THEN 'CASH' WHEN 4 THEN 'CRYPTO' WHEN 5 THEN 'COMMODITY'
                        ELSE 'OTHER'
                    END AS asset_class,
                    currency, CASE provider WHEN 1 THEN 'Kraken' WHEN 2 THEN 'YF' END AS provider,
                    quantity, target_weight, manual_price AS price, max_fee_impact, fee_type, fee_amount,
                    fee_rate, min_fee, max_fee, average_buy_price, created_at, updated_at
             FROM portfolio_asset
             WHERE portfolio_id = ANY($1)
             ORDER BY portfolio_id, id",
        )
        .bind(&portfolio_ids)
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
        let mut tx = self.pool.begin().await?;
        let existing = query_as::<_, PortfolioRow>(
            "SELECT id, user_id, name, currency, deleted, last_updated_at,
                    max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                    created_at, updated_at
             FROM portfolios
             WHERE id = $1
             FOR UPDATE",
        )
        .bind(portfolio_req.id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(existing) = &existing
            && existing.user_id != user_id
        {
            return Err(DcaError::BadRequest(
                "Portfolio cannot be updated".to_string(),
            ));
        }

        let fee_fields = Self::extract_fee_fields(portfolio_req.fees.clone());
        let portfolio = if existing.is_some() {
            query_as::<_, PortfolioRow>(
                "UPDATE portfolios
                 SET name = $2, currency = $3, last_updated_at = $4,
                     max_fee_impact = $5, fee_type = $6, fee_amount = $7,
                     fee_rate = $8, min_fee = $9, max_fee = $10
                 WHERE id = $1
                 RETURNING id, user_id, name, currency, deleted, last_updated_at,
                           max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                           created_at, updated_at",
            )
            .bind(portfolio_req.id)
            .bind(&portfolio_req.name)
            .bind(&portfolio_req.quote_ccy)
            .bind(portfolio_req.last_updated_at)
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
                           max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
                           created_at, updated_at",
            )
            .bind(portfolio_req.id)
            .bind(user_id)
            .bind(&portfolio_req.name)
            .bind(&portfolio_req.quote_ccy)
            .bind(portfolio_req.last_updated_at)
            .bind(fee_fields.max_fee_impact)
            .bind(fee_fields.fee_type)
            .bind(fee_fields.fee_amount)
            .bind(fee_fields.fee_rate)
            .bind(fee_fields.min_fee)
            .bind(fee_fields.max_fee)
            .fetch_one(&mut *tx)
            .await?
        };

        let assets =
            Self::upsert_assets_transaction(&mut tx, portfolio_req.id, portfolio_req.assets)
                .await?;

        // A sync must expose the portfolio and its asset set as one consistent change.
        tx.commit().await?;
        Ok((portfolio, assets))
    }
}
