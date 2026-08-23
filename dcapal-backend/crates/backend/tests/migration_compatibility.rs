use migration::MIGRATOR;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(
    migrations = false,
    fixtures("legacy_schema", "legacy_portfolio_assets")
)]
async fn sqlx_migrations_adopt_the_existing_seaorm_schema(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN a production-shaped SeaORM database with legacy Portfolio Asset rows,
    // WHEN SQLx applies its migrations, THEN the canonical rows are normalized.
    MIGRATOR.run(&pool).await?;

    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(migration_count, 6);

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
               'manual_price', 'price'
           )
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        columns,
        vec![
            ("asset_class".into(), "smallint".into(), "YES".into()),
            ("manual_price".into(), "numeric".into(), "YES".into()),
            ("provider".into(), "smallint".into(), "YES".into()),
        ]
    );

    let assets: Vec<(Uuid, String, i16, i16, Decimal)> = sqlx::query_as(
        "SELECT id, symbol, provider, asset_class, manual_price
         FROM portfolio_asset
         ORDER BY symbol",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(assets.len(), 8);
    assert_eq!(
        assets,
        vec![
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000005").unwrap(),
                "BTC".into(),
                2,
                4,
                Decimal::from(50),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000004").unwrap(),
                "CASH".into(),
                2,
                3,
                Decimal::from(40),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000011").unwrap(),
                "EQUITY".into(),
                2,
                1,
                Decimal::from(110),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000003").unwrap(),
                "EUR".into(),
                2,
                3,
                Decimal::from(30),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000006").unwrap(),
                "GOLD".into(),
                2,
                5,
                Decimal::from(60),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000002").unwrap(),
                "FOO".into(),
                1,
                2,
                Decimal::from(20),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000007").unwrap(),
                "OTHER".into(),
                1,
                0,
                Decimal::from(70),
            ),
            (
                Uuid::parse_str("20000000-0000-0000-0000-000000000009").unwrap(),
                "TIE".into(),
                1,
                2,
                Decimal::from(90),
            ),
        ]
    );

    // GIVEN a successfully applied migration, WHEN SQLx runs it again,
    // THEN it skips the recorded migration and preserves the normalized rows.
    MIGRATOR.run(&pool).await?;
    let rerun_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset")
        .fetch_one(&pool)
        .await?;
    assert_eq!(rerun_count, 8);

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
