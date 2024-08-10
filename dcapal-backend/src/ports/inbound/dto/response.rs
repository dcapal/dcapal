use crate::ports::outbound::repository::user::UserEntity;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfileResponse {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: String,
    pub birth_date: NaiveDate,
}

impl From<UserEntity> for UserProfileResponse {
    fn from(entity: UserEntity) -> Self {
        Self {
            first_name: entity.first_name,
            last_name: entity.last_name,
            email: entity.email,
            birth_date: entity.birth_date,
        }
    }
}
