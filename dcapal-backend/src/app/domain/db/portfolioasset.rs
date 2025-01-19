use crate::app::domain::entity::fee::FeeStructure;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sea_orm::{DeriveEntityModel, Related, RelationDef};
use uuid::Uuid;
use crate::app::domain::db::fee::FeeStructure;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "portfolio_asset")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub provider: String,
    pub quantity: BigDecimal,
    pub target_weight: BigDecimal,
    pub price: BigDecimal,
    pub max_fee_impact: Option<BigDecimal>,
    pub fee_structure: FeeStructure,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::portfolio::Entity",
        from = "Column::PortfolioId",
        to = "super::portfolio::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Portfolio,
}

impl Related<super::portfolio::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Portfolio.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::portfolio::Relation::PortfolioAsset.def().rev())
    }
}