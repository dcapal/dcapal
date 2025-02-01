use crate::DateTime;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::portfolio::Entity")]
    Portfolio,
}

impl Related<super::portfolio::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Portfolio.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
