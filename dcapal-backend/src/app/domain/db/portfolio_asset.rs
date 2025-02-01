use crate::app::domain::db::fee_type::FeeType;
use bigdecimal::BigDecimal;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "portfolio_asset")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub symbol: String,
    pub portfolio_id: Uuid,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
    pub provider: String,
    pub quantity: BigDecimal,
    pub target_weight: BigDecimal,
    pub price: BigDecimal,
    pub max_fee_impact: Option<BigDecimal>,
    pub fee_type: Option<FeeType>,
    pub fee_amount: Option<BigDecimal>,
    pub fee_rate: Option<BigDecimal>,
    pub min_fee: Option<BigDecimal>,
    pub max_fee: Option<BigDecimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
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

impl ActiveModelBehavior for ActiveModel {}
