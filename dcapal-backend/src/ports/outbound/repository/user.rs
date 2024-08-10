use axum::extract::State;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::error::Result;
use crate::AppContext;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UserEntity {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: String,
    pub birth_date: NaiveDate,
}

pub async fn get_current_user(user_id: Uuid, State(ctx): State<AppContext>) -> Result<UserEntity> {
    let user = sqlx::query!(
        r#"select first_name, last_name, email, birthdate from "users" where id = $1"#,
        user_id
    )
    .fetch_one(&ctx.postgres)
    .await?;

    Ok(UserEntity {
        first_name: user.first_name,
        last_name: user.last_name,
        email: user.email,
        birth_date: Default::default(),
    })
}
