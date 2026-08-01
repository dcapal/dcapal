use chrono::Utc;
use dcapal_backend::{
    app::infra::claim::{Claims, UserMetadataClaim},
    ports::outbound::repository::{postgres::SqlxUserRepository, user::UserRepository},
};
use sqlx::PgPool;
use uuid::Uuid;

fn claims(user_id: Uuid, email: &str, full_name: Option<&str>) -> Claims {
    Claims {
        iat: Utc::now().timestamp(),
        exp: (Utc::now().timestamp() + 3600) as usize,
        sub: user_id,
        user_metadata: UserMetadataClaim {
            email: email.to_string(),
            full_name: full_name.map(str::to_string),
        },
        role: "user".to_string(),
        session_id: Uuid::nil(),
        aud: "authenticated".to_string(),
    }
}

#[sqlx::test(migrations = "./migrations", fixtures("users"))]
async fn updates_an_existing_user(pool: PgPool) -> dcapal_backend::error::Result<()> {
    let repository = SqlxUserRepository::new(pool.clone());
    let user = repository
        .save_user_if_not_present(&claims(
            Uuid::from_u128(1),
            "updated@example.com",
            Some("Updated User"),
        ))
        .await?;

    assert_eq!(user.id, Uuid::from_u128(1));
    assert_eq!(user.username.as_deref(), Some("Updated User"));
    assert_eq!(user.email, "updated@example.com");

    let stored_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(Uuid::from_u128(1))
        .fetch_one(&pool)
        .await?;
    assert_eq!(stored_email, "updated@example.com");

    Ok(())
}

#[sqlx::test(migrations = "./migrations", fixtures("users"))]
async fn inserts_a_missing_user(pool: PgPool) -> dcapal_backend::error::Result<()> {
    let repository = SqlxUserRepository::new(pool.clone());
    let user = repository
        .save_user_if_not_present(&claims(Uuid::from_u128(3), "new@example.com", None))
        .await?;

    assert_eq!(user.id, Uuid::from_u128(3));
    assert_eq!(user.username, None);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 3);

    Ok(())
}
