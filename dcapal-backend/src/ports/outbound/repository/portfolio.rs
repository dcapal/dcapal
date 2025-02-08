use crate::app::domain::db::{portfolio_asset, portfolios};
use crate::error::Result;
use crate::ports::inbound::rest::request::{PortfolioAssetRequest, PortfolioRequest};
use crate::ports::inbound::rest::FeeStructure;
use sea_orm::{
    entity::*, sqlx, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    SqlxPostgresConnector,
};
use uuid::Uuid;

pub struct PortfolioRepository {
    pub db_conn: DatabaseConnection,
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

    async fn upsert_assets(
        &self,
        portfolio_id: Uuid,
        assets: Vec<PortfolioAssetRequest>,
    ) -> Result<Vec<portfolio_asset::Model>> {
        let mut updated_assets = Vec::new();

        // Get existing assets for this portfolio
        let existing_assets = portfolio_asset::Entity::find()
            .filter(portfolio_asset::Column::PortfolioId.eq(portfolio_id))
            .all(&self.db_conn)
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

            // Update asset fields
            asset_model.symbol = Set(asset.symbol.clone());
            asset_model.name = Set(asset.name.clone());
            asset_model.asset_class = Set(asset.aclass.clone());
            asset_model.currency = Set(asset.base_ccy.clone());
            asset_model.provider = Set(asset.provider.clone());
            asset_model.quantity = Set(asset.qty);
            asset_model.target_weight = Set(asset.target_weight);
            asset_model.price = Set(asset.price);

            // Handle fees similar to portfolio
            //...
            let updated = if existing_asset.is_some() {
                asset_model.update(&self.db_conn).await?
            } else {
                asset_model.insert(&self.db_conn).await?
            };

            updated_assets.push(updated);
        }

        // Delete assets that are no longer present
        let current_symbols: Vec<String> = assets.into_iter().map(|a| a.symbol).collect();
        for existing in existing_assets {
            if !current_symbols.contains(&existing.symbol) {
                portfolio_asset::Entity::delete_by_id(existing.id)
                    .exec(&self.db_conn)
                    .await?;
            }
        }

        Ok(updated_assets)
    }

    //TODO: refactoring
    pub async fn upsert(
        &self,
        user_id: Uuid,
        portfolio_req: PortfolioRequest,
    ) -> Result<(portfolios::Model, Vec<portfolio_asset::Model>)> {
        let (max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee) =
            if let Some(fees) = portfolio_req.fees {
                match fees.fee_structure {
                    FeeStructure::ZeroFee => (
                        Set(fees.max_fee_impact),
                        Set(Some(fees.fee_structure.to_string())),
                        Set(None),
                        Set(None),
                        Set(None),
                        Set(None),
                    ),
                    FeeStructure::Fixed { fee_amount } => (
                        Set(fees.max_fee_impact),
                        Set(Some(fees.fee_structure.to_string())),
                        Set(Some(fee_amount)),
                        Set(None),
                        Set(None),
                        Set(None),
                    ),
                    FeeStructure::Variable {
                        fee_rate,
                        min_fee,
                        max_fee,
                    } => (
                        Set(fees.max_fee_impact),
                        Set(Some(fees.fee_structure.to_string())),
                        Set(None),
                        Set(Some(fee_rate)),
                        Set(Some(min_fee)),
                        Set(max_fee),
                    ),
                }
            } else {
                (
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                )
            };

        let existing_portfolio = portfolios::Entity::find_by_id(portfolio_req.id)
            .one(&self.db_conn)
            .await?;

        let mut portfolio_model = if let Some(existing) = existing_portfolio.clone() {
            existing.into_active_model()
        } else {
            portfolios::ActiveModel {
                id: Set(portfolio_req.id),
                user_id: Set(user_id),
                deleted: Set(false), // When creating a new portfolio, it is not deleted
                created_at: Default::default(),
                updated_at: Default::default(),
                ..Default::default()
            }
        };

        portfolio_model.name = Set(portfolio_req.name.clone());
        portfolio_model.currency = Set(portfolio_req.quote_ccy.clone());
        portfolio_model.last_updated_at = Set(portfolio_req.last_updated_at.into());
        portfolio_model.max_fee_impact = max_fee_impact;
        portfolio_model.fee_type = fee_type;
        portfolio_model.fee_amount = fee_amount;
        portfolio_model.fee_rate = fee_rate;
        portfolio_model.min_fee = min_fee;
        portfolio_model.max_fee = max_fee;

        let portfolio = if existing_portfolio.is_some() {
            portfolio_model.update(&self.db_conn).await?
        } else {
            portfolio_model.insert(&self.db_conn).await?
        };

        // Handle portfolio assets
        let assets = self
            .upsert_assets(portfolio_req.id, portfolio_req.assets)
            .await?;

        Ok((portfolio, assets))
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
}
