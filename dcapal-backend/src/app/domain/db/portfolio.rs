use bigdecimal::BigDecimal;
use sea_orm::{DeriveEntityModel, DeriveRelation, EnumIter, Related, RelationDef};
use time::Date;
use uuid::Uuid;
use crate::app::domain::db::fee::FeeStructure;
use crate::DateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "portfolio")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub currency: String,
    pub deleted: bool,
    pub last_updated_at: DateTime,
    pub max_fee_impact: BigDecimal,
    pub fee_structure: FeeStructure,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    PortfolioAsset,
}

impl Related<super::portfolio_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PortfolioAsset.def()
    }
}