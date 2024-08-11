use crate::app::domain::entity::User;
use crate::error::Result;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    pub postgres: sqlx::PgPool,
}

impl UserRepository {
    pub fn new(postgres: sqlx::PgPool) -> Self {
        Self { postgres }
    }

    pub async fn get_current_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query!(
            r#"select first_name, last_name, email, birthdate from "users" where id = $1"#,
            user_id
        )
        .fetch_one(&self.postgres)
        .await?;

        Ok(Some(User {
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            birth_date: user.birthdate,
        }))
    }
}
