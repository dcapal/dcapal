use crate::DateTime;
use bigdecimal::BigDecimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;
use crate::app::domain::db::fee_type::FeeType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "portfolio")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub currency: String,
    pub deleted: bool,
    pub last_updated_at: DateTime,
    pub max_fee_impact: Option<BigDecimal>,
    pub fee_type: Option<FeeType>,
    pub fee_amount: Option<BigDecimal>,
    pub fee_rate: Option<BigDecimal>,
    pub min_fee: Option<BigDecimal>,
    pub max_fee: Option<BigDecimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::portfolio_asset::Entity")]
    PortfolioAsset,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::portfolio_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PortfolioAsset.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
