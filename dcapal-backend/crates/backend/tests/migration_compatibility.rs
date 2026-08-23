use migration::MIGRATOR;
use sqlx::PgPool;

#[sqlx::test(migrations = false, fixtures("legacy_schema"))]
async fn sqlx_migrations_adopt_the_existing_seaorm_schema(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN a production-shaped SeaORM database, WHEN SQLx applies its migrations,
    // THEN the legacy text fields and the nullable canonical schema foundation coexist.
    MIGRATOR.run(&pool).await?;

    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(migration_count, 5);

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

    let columns: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT column_name, data_type, is_nullable
         FROM information_schema.columns
         WHERE table_name = 'portfolio_asset'
           AND column_name IN (
               'legacy_provider', 'legacy_asset_class', 'provider', 'asset_class',
               'manual_price'
           )
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        columns,
        vec![
            ("asset_class".into(), "smallint".into(), "YES".into()),
            ("legacy_asset_class".into(), "text".into(), "NO".into()),
            ("legacy_provider".into(), "text".into(), "NO".into()),
            ("manual_price".into(), "numeric".into(), "YES".into()),
            ("provider".into(), "smallint".into(), "YES".into()),
        ]
    );

    let check_constraint_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM pg_constraint
         WHERE conrelid = 'portfolio_asset'::regclass
           AND contype = 'c'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(check_constraint_count, 0);

    Ok(())
}
