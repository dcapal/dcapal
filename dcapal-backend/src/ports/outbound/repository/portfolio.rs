use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::error::Result;
use crate::ports::inbound::rest::request::PortfolioRequest;
use crate::ports::inbound::rest::FeeStructure;
use bigdecimal::BigDecimal;
use sea_orm::{
    entity::*, sqlx, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    SqlxPostgresConnector,
};
use uuid::Uuid;
use crate::app::domain::db::fee_type::FeeType;

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

    //TODO: refactoring
    pub async fn upsert(
        &self,
        user_id: Uuid,
        portfolio: PortfolioRequest,
    ) -> Result<portfolio::ActiveModel> {
        let (max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee) =
            if let Some(fees) = portfolio.fees {
                match fees.fee_type {
                    FeeStructure::ZeroFee => (
                        Set(fees.max_fee_impact),
                        Set(Some(FeeType::ZeroFee)),
                        Set(None),
                        Set(None),
                        Set(None),
                        Set(None),
                    ),
                    FeeStructure::Fixed { fee_amount } => (
                        Set(fees.max_fee_impact),
                        Set(Some(FeeType::Fixed)),
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
                        Set(Some(FeeType::Variable)),
                        Set(BigDecimal::from(0)),
                        Set(fee_rate),
                        Set(min_fee),
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

        let portfolio_model = portfolio::ActiveModel {
            id: Set(portfolio.id),
            user_id: Set(user_id),
            name: Set(portfolio.name.clone()),
            currency: Set(portfolio.quote_ccy.clone()),
            deleted: Set(false), // When creating a new portfolio, it is not deleted
            last_updated_at: Set(portfolio.last_updated_at),
            max_fee_impact,
            fee_type,
            fee_amount,
            fee_rate,
            min_fee,
            max_fee,
        };

        let portfolio = portfolio_model.save(&self.db_conn).await?;

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
