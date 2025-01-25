use sea_orm::DeriveEntityModel;
use uuid::Uuid;
use crate::DateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct User {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
