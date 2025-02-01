use crate::app::domain::db::users;
use crate::app::infra::claim::Claims;
use crate::error::Result;
use sea_orm::{sqlx, DatabaseConnection, SqlxPostgresConnector};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

pub struct UserRepository {
    pub db_conn: DatabaseConnection,
}

impl UserRepository {
    pub fn new(postgres: sqlx::PgPool) -> Self {
        let db_conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres);
        Self { db_conn }
    }

    pub async fn save_user_if_not_present(&self, claims: &Claims) -> Result<users::Model> {
        let existing = users::Entity::find_by_id(claims.sub)
            .one(&self.db_conn)
            .await?;
        match existing {
            Some(usr) => {
                let mut active_user: users::ActiveModel = usr.into();
                active_user.email = Set(claims.user_metadata.email.clone());
                active_user.updated_at = Set(chrono::Utc::now().into());
                Ok(active_user.update(&self.db_conn).await?)
            }
            None => {
                let new_user = users::ActiveModel {
                    id: Set(claims.sub),
                    username: Set(claims.user_metadata.full_name.clone()),
                    email: Set(claims.user_metadata.email.clone()),
                    role: Set(claims.role.clone()),
                    created_at: Set(chrono::Utc::now().into()),
                    updated_at: Set(chrono::Utc::now().into()),
                };
                Ok(new_user.insert(&self.db_conn).await?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_save_user_if_not_present_updates_existing_user() {
        let user_id = uuid::Uuid::new_v4();
        let claims = Claims {
            iat: 0,
            exp: 0,
            sub: user_id,
            user_metadata: crate::app::infra::claim::UserMetadataClaim {
                email: "updated-email@example.com".to_string(),
                full_name: Some("Test User".to_string()),
            },
            role: "user".to_string(),
            session_id: Default::default(),
            aud: "".to_string(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query returns the existing DB user
            .append_query_results(vec![vec![users::Model {
                id: user_id,
                username: Some("Existing User".to_string()),
                email: "old-email@example.com".to_string(),
                role: "user".to_string(),
                created_at: Default::default(),
                updated_at: Default::default(),
            }]])
            // Next query result after update
            .append_query_results(vec![vec![users::Model {
                id: user_id,
                username: Some("Existing User".to_string()),
                email: "updated-email@example.com".to_string(),
                role: "user".to_string(),
                created_at: Default::default(),
                updated_at: Default::default(),
            }]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = UserRepository { db_conn: db };

        let updated_user = repo.save_user_if_not_present(&claims).await.unwrap();
        assert_eq!(updated_user.email, "updated-email@example.com");
    }
}
