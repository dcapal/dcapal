use crate::app::domain::db::{portfolio_asset, portfolios};
use crate::error::{DcaError, Result};
use crate::ports::inbound::rest::request::{
    PortfolioAssetRequest, PortfolioRequest, TransactionFeesRequest,
};
use crate::ports::inbound::rest::FeeStructure;
use rust_decimal::Decimal;
use sea_orm::{
    entity::*, sqlx, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, SqlxPostgresConnector, TransactionTrait,
};
use uuid::Uuid;

pub struct PortfolioRepository {
    pub db_conn: DatabaseConnection,
}

#[derive(Default)]
struct FeeFields {
    max_fee_impact: ActiveValue<Option<Decimal>>,
    fee_type: ActiveValue<Option<String>>,
    fee_amount: ActiveValue<Option<Decimal>>,
    fee_rate: ActiveValue<Option<Decimal>>,
    min_fee: ActiveValue<Option<Decimal>>,
    max_fee: ActiveValue<Option<Decimal>>,
}

impl PortfolioRepository {
    pub fn new(postgres: sqlx::PgPool) -> Self {
        let db_conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres);
        Self { db_conn }
    }

    pub async fn get_user_portfolios_with_assets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(portfolios::Model, Vec<portfolio_asset::Model>)>> {
        let portfolios_with_assets = portfolios::Entity::find()
            .filter(portfolios::Column::UserId.eq(user_id))
            .find_with_related(portfolio_asset::Entity)
            .all(&self.db_conn)
            .await?;

        Ok(portfolios_with_assets)
    }

    pub async fn soft_delete(&self, portfolio_id: Uuid) -> Result<()> {
        if let Some(portfolio_db) = portfolios::Entity::find_by_id(portfolio_id)
            .one(&self.db_conn)
            .await?
        {
            let mut portfolio: portfolios::ActiveModel = portfolio_db.into();
            portfolio.deleted = Set(true);
            portfolio.update(&self.db_conn).await?;
        }

        Ok(())
    }

    pub async fn upsert(
        &self,
        user_id: Uuid,
        portfolio_req: PortfolioRequest,
    ) -> Result<(portfolios::Model, Vec<portfolio_asset::Model>)> {
        let fee_fields = Self::extract_fee_fields(portfolio_req.fees.clone());
        let portfolio_req = portfolio_req.clone();

        let result = self
            .db_conn
            .transaction::<_, (portfolios::Model, Vec<portfolio_asset::Model>), DcaError>(|txn| {
                Box::pin(async move {
                    let existing_portfolio = portfolios::Entity::find_by_id(portfolio_req.id)
                        .one(txn)
                        .await?;

                    let mut portfolio_model = if let Some(existing) = existing_portfolio.clone() {
                        existing.into_active_model()
                    } else {
                        portfolios::ActiveModel {
                            id: Set(portfolio_req.id),
                            user_id: Set(user_id),
                            deleted: Set(false),
                            created_at: Default::default(),
                            updated_at: Default::default(),
                            ..Default::default()
                        }
                    };

                    portfolio_model.name = Set(portfolio_req.name.clone());
                    portfolio_model.currency = Set(portfolio_req.quote_ccy.clone());
                    portfolio_model.last_updated_at = Set(portfolio_req.last_updated_at.into());
                    portfolio_model.max_fee_impact = fee_fields.max_fee_impact;
                    portfolio_model.fee_type = fee_fields.fee_type;
                    portfolio_model.fee_amount = fee_fields.fee_amount;
                    portfolio_model.fee_rate = fee_fields.fee_rate;
                    portfolio_model.min_fee = fee_fields.min_fee;
                    portfolio_model.max_fee = fee_fields.max_fee;

                    let portfolio = if existing_portfolio.is_some() {
                        portfolio_model.update(txn).await?
                    } else {
                        portfolio_model.insert(txn).await?
                    };

                    let assets = Self::upsert_assets_transaction(
                        txn,
                        portfolio_req.id,
                        portfolio_req.assets,
                    )
                    .await?;

                    Ok((portfolio, assets))
                })
            })
            .await?;

        Ok(result)
    }

    async fn upsert_assets_transaction(
        txn: &DatabaseTransaction,
        portfolio_id: Uuid,
        assets: Vec<PortfolioAssetRequest>,
    ) -> Result<Vec<portfolio_asset::Model>> {
        let mut updated_assets = Vec::new();

        let existing_assets = portfolio_asset::Entity::find()
            .filter(portfolio_asset::Column::PortfolioId.eq(portfolio_id))
            .all(txn)
            .await?;

        for asset in &assets {
            let existing_asset = existing_assets.iter().find(|a| a.symbol == asset.symbol);

            let mut asset_model = if let Some(existing) = existing_asset {
                existing.clone().into_active_model()
            } else {
                portfolio_asset::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    portfolio_id: Set(portfolio_id),
                    created_at: Default::default(),
                    updated_at: Default::default(),
                    ..Default::default()
                }
            };

            let fee_fields = Self::extract_fee_fields(asset.clone().fees);

            asset_model.symbol = Set(asset.symbol.clone());
            asset_model.name = Set(asset.name.clone());
            asset_model.asset_class = Set(asset.aclass.clone());
            asset_model.currency = Set(asset.base_ccy.clone());
            asset_model.provider = Set(asset.provider.clone());
            asset_model.quantity = Set(asset.qty);
            asset_model.target_weight = Set(asset.target_weight);
            asset_model.price = Set(asset.price);
            asset_model.max_fee_impact = fee_fields.max_fee_impact;
            asset_model.fee_type = fee_fields.fee_type;
            asset_model.fee_amount = fee_fields.fee_amount;
            asset_model.fee_rate = fee_fields.fee_rate;
            asset_model.min_fee = fee_fields.min_fee;
            asset_model.max_fee = fee_fields.max_fee;

            let updated = if existing_asset.is_some() {
                asset_model.update(txn).await?
            } else {
                asset_model.insert(txn).await?
            };

            updated_assets.push(updated);
        }

        let current_symbols: Vec<String> = assets.into_iter().map(|a| a.symbol).collect();
        for existing in existing_assets {
            if !current_symbols.contains(&existing.symbol) {
                portfolio_asset::Entity::delete_by_id(existing.id)
                    .exec(txn)
                    .await?;
            }
        }

        Ok(updated_assets)
    }

    fn extract_fee_fields(fees: Option<TransactionFeesRequest>) -> FeeFields {
        if let Some(fees) = fees {
            match fees.fee_structure {
                FeeStructure::ZeroFee => FeeFields {
                    max_fee_impact: Set(fees.max_fee_impact),
                    fee_type: Set(Some(fees.fee_structure.to_string())),
                    fee_amount: Set(None),
                    fee_rate: Set(None),
                    min_fee: Set(None),
                    max_fee: Set(None),
                },
                FeeStructure::Fixed { fee_amount } => FeeFields {
                    max_fee_impact: Set(fees.max_fee_impact),
                    fee_type: Set(Some(fees.fee_structure.to_string())),
                    fee_amount: Set(Some(fee_amount)),
                    fee_rate: Set(None),
                    min_fee: Set(None),
                    max_fee: Set(None),
                },
                FeeStructure::Variable {
                    fee_rate,
                    min_fee,
                    max_fee,
                } => FeeFields {
                    max_fee_impact: Set(fees.max_fee_impact),
                    fee_type: Set(Some(fees.fee_structure.to_string())),
                    fee_amount: Set(None),
                    fee_rate: Set(Some(fee_rate)),
                    min_fee: Set(Some(min_fee)),
                    max_fee: Set(max_fee),
                },
            }
        } else {
            FeeFields {
                max_fee_impact: Set(None),
                fee_type: Set(Some(FeeStructure::ZeroFee.to_string())),
                fee_amount: Set(None),
                fee_rate: Set(None),
                min_fee: Set(None),
                max_fee: Set(None),
            }
        }
    }
}
