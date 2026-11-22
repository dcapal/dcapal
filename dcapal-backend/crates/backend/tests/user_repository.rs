use dcapal_backend::{
    app::infra::claim::{Claims, UserMetadataClaim},
    ports::outbound::repository::{postgres::SqlxUserRepository, user::UserRepository},
};
use jiff::Timestamp;
use sqlx::PgPool;
use uuid::Uuid;

fn claims(user_id: Uuid, email: &str, full_name: Option<&str>) -> Claims {
    Claims {
        iat: Timestamp::now().as_second(),
        exp: (Timestamp::now().as_second() + 3600) as usize,
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

#[sqlx::test(migrations = "../../migrations", fixtures("users"))]
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

#[sqlx::test(migrations = "../../migrations", fixtures("users"))]
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

/// GIVEN an existing user whose name, email and role match the token claims,
/// WHEN the user is saved again (as happens on every authenticated request),
/// THEN the stored row is returned untouched and `updated_at` does not move.
#[sqlx::test(migrations = "../../migrations", fixtures("users"))]
async fn leaves_an_unchanged_user_untouched(pool: PgPool) -> dcapal_backend::error::Result<()> {
    let repository = SqlxUserRepository::new(pool.clone());
    let before: jiff_sqlx::Timestamp =
        sqlx::query_scalar("SELECT updated_at FROM users WHERE id = $1")
            .bind(Uuid::from_u128(1))
            .fetch_one(&pool)
            .await?;
    let before = before.to_jiff();

    let user = repository
        .save_user_if_not_present(&claims(
            Uuid::from_u128(1),
            "existing@example.com",
            Some("Existing User"),
        ))
        .await?;

    assert_eq!(user.updated_at, before);
    let after: jiff_sqlx::Timestamp =
        sqlx::query_scalar("SELECT updated_at FROM users WHERE id = $1")
            .bind(Uuid::from_u128(1))
            .fetch_one(&pool)
            .await?;
    let after = after.to_jiff();
    assert_eq!(after, before);

    Ok(())
}
