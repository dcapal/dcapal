use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, query_as};

use crate::{
    app::infra::claim::Claims,
    error::Result,
    ports::outbound::repository::{postgres::types::UserRow, user::UserRepository},
};

/// PostgreSQL implementation of user persistence backed by a SQLx pool.
#[derive(Clone)]
pub struct SqlxUserRepository {
    pool: PgPool,
}

impl SqlxUserRepository {
    /// Creates a user repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqlxUserRepository {
    async fn save_user_if_not_present(&self, claims: &Claims) -> Result<UserRow> {
        let existing = query_as::<_, UserRow>(
            "SELECT id, username, email, role, created_at, updated_at
             FROM users
             WHERE id = $1",
        )
        .bind(claims.sub)
        .fetch_optional(&self.pool)
        .await?;

        let now = Utc::now();

        if existing.is_some() {
            Ok(query_as::<_, UserRow>(
                "UPDATE users
                 SET username = $2, email = $3, role = $4, updated_at = $5
                 WHERE id = $1
                 RETURNING id, username, email, role, created_at, updated_at",
            )
            .bind(claims.sub)
            .bind(claims.user_metadata.full_name.clone())
            .bind(&claims.user_metadata.email)
            .bind(&claims.role)
            .bind(now)
            .fetch_one(&self.pool)
            .await?)
        } else {
            Ok(query_as::<_, UserRow>(
                "INSERT INTO users (id, username, email, role, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $5)
                 RETURNING id, username, email, role, created_at, updated_at",
            )
            .bind(claims.sub)
            .bind(claims.user_metadata.full_name.clone())
            .bind(&claims.user_metadata.email)
            .bind(&claims.role)
            .bind(now)
            .fetch_one(&self.pool)
            .await?)
        }
    }
}
