use migration::MIGRATOR;
use sqlx::PgPool;

#[sqlx::test(migrations = false, fixtures("legacy_schema"))]
async fn sqlx_migrations_adopt_the_existing_seaorm_schema(pool: PgPool) -> sqlx::Result<()> {
    MIGRATOR.run(&pool).await?;

    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(migration_count, 4);

    let seaorm_table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('seaql_migrations')::text")
            .fetch_one(&pool)
            .await?;
    assert_eq!(seaorm_table.as_deref(), Some("seaql_migrations"));

    let average_buy_price: Option<String> = sqlx::query_scalar(
        "SELECT data_type
         FROM information_schema.columns
         WHERE table_name = 'portfolio_asset'
           AND column_name = 'average_buy_price'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(average_buy_price.as_deref(), Some("numeric"));

    Ok(())
}
