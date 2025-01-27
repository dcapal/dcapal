use crate::app::domain::db::user;
use crate::app::infra::claim::Claims;
use crate::error::Result;
use sea_orm::{sqlx, DatabaseConnection, EntityTrait, SqlxPostgresConnector};

pub struct UserRepository {
    pub db_conn: DatabaseConnection,
}

impl UserRepository {
    pub fn new(postgres: sqlx::PgPool) -> Self {
        let db_conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres);
        Self { db_conn }
    }

    pub async fn save_user_if_not_present(&self, claims: &Claims) -> Result<()> {
        use sea_orm::ActiveModelTrait;
        use sea_orm::Set;

        let new_user = user::ActiveModel {
            id: Set(claims.sub),
            username: Set(claims.user_metadata.full_name.clone().unwrap_or_default()),
            email: Set(claims.user_metadata.email.clone()),
            role: Set(claims.role.clone()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        new_user.insert(&self.db_conn).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::claim::Claims;
    use sea_orm::DatabaseBackend;
    use sea_orm::MockDatabase;
    use sea_orm::MockExecResult;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_save_user_if_not_present() {
        let claims = Claims {
            iat: 0,
            exp: 0,
            sub: Uuid::new_v4(),
            user_metadata: crate::app::infra::claim::UserMetadataClaim {
                email: "test@example.com".to_string(),
                full_name: Some("Test User".to_string()),
            },
            role: "user".to_string(),
            session_id: Default::default(),
            aud: "".to_string(),
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![]]) // No user found
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = UserRepository { db_conn: db };

        let result = repo.save_user_if_not_present(&claims).await;
        assert!(result.is_ok());
    }
}
