use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::error::Result;
use sea_orm::{
    sqlx, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, SqlxPostgresConnector,
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

    pub async fn soft_delete(&self, portfolio_id: Uuid) -> Result<()> {
        let portfolio: Option<portfolio::Model> = portfolio::Entity::find_by_id(portfolio_id)
            .one(&self.db_conn)
            .await?;

        let mut portfolio: portfolio::ActiveModel = portfolio.unwrap().into();

        portfolio.deleted = Set(true);

        let portfolio: portfolio::ActiveModel = portfolio.update(&self.db_conn).await?;

        Ok(())
    }
}
