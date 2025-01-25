use crate::app::domain::db::portfolio::Relation::PortfolioAsset;
use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::error::Result;
use sea_orm::{sqlx, DatabaseConnection, EntityTrait, QueryFilter, SqlxPostgresConnector};
use uuid::Uuid;

#[derive(Clone)]
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
    ) -> Result<Vec<(portfolio::Model, Vec<portfolio_asset::Model>)>> {

        let portfolios_with_assets = portfolio::Entity::find()
            .filter(portfolio::Column::UserId.eq(user_id))
            .find_with_related(PortfolioAsset)
            .all(&self.db_conn)
            .await?;

        Ok(portfolios_with_assets)
    }

    pub async fn upsert(
        &self,
        user_id: Uuid,
        portfolio: portfolio::ActiveModel,
    ) -> Result<portfolio::Model> {

        let portfolio = portfolio
            .upsert()
            .filter(portfolio::Column::UserId.eq(user_id))
            .exec(&self.db_conn)
            .await?;

        Ok(portfolio)
    }

    pub async fn soft_delete(&self, user_id: Uuid, portfolio_id: Uuid) -> Result<()> {
        portfolio::Entity::update()
            .set(portfolio::Column::Deleted, true)
            .filter(portfolio::Column::UserId.eq(user_id))
            .filter(portfolio::Column::Id.eq(portfolio_id))
            .exec(&self.db_conn)
            .await?;

        Ok(())
    }
}
