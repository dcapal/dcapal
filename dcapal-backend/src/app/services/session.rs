use sea_orm::sqlx;
use sea_orm::sqlx::PgPool;
use crate::app::infra::claim::Claims;
use crate::error::Result;

pub async fn save_user_if_not_present(postgres: &PgPool, claims: &Claims) -> Result<()> {
    let mut tx = postgres.begin().await?;

    // insert auth user if not present
    sqlx::query!(
        r#"
        INSERT INTO auth.users (id, aud, role, email)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT DO NOTHING
        "#,
        claims.sub,
        claims.aud,
        claims.role,
        claims.user_metadata.email
    )
    .execute(&mut *tx)
    .await?;

    let user_id = &claims.sub;
    sqlx::query!(
        r#"
        INSERT INTO public.users (id, name, email)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        claims.user_metadata.full_name,
        claims.user_metadata.email
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
