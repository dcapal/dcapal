use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::error::Result;
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
    ) -> Result<Vec<(portfolio::Model, Vec<portfolio_asset::Model>)>> {
        let portfolios_with_assets = portfolio::Entity::find()
            .filter(portfolio::Column::UserId.eq(user_id))
            .find_with_related(portfolio_asset::Entity)
            .all(&self.db_conn)
            .await?;

        Ok(portfolios_with_assets)
    }

    pub async fn upsert(&self, portfolio: portfolio::ActiveModel) -> Result<portfolio::ActiveModel> {
        let portfolio = portfolio.save(&self.db_conn).await?;

        Ok(portfolio)
    }

    pub async fn soft_delete(&self, portfolio_id: Uuid) -> Result<()> {
        let portfolio_db: Option<portfolio::Model> = portfolio::Entity::find_by_id(portfolio_id)
            .one(&self.db_conn)
            .await?;

        let mut portfolio: portfolio::ActiveModel = portfolio_db.unwrap().into();

        portfolio.deleted = Set(true);

        portfolio.update(&self.db_conn).await?;

        Ok(())
    }
}
