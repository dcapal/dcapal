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
    assert_eq!(migration_count, 7);

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
               'manual_price', 'price', 'assets_data_id', 'asset_class_override'
           )
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        columns,
        vec![
            (
                "asset_class_override".into(),
                "smallint".into(),
                "YES".into()
            ),
            ("assets_data_id".into(), "uuid".into(), "NO".into()),
            ("manual_price".into(), "numeric".into(), "YES".into()),
        ]
    );

    let assets: Vec<(String, i16, i16, Decimal)> = sqlx::query_as(
        "SELECT ad.symbol, ad.provider, ad.asset_class, pa.manual_price
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         ORDER BY ad.symbol",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(assets.len(), 8);
    assert_eq!(
        assets,
        vec![
            ("BTC".into(), 2, 4, Decimal::from(50),),
            ("CASH".into(), 2, 3, Decimal::from(40),),
            ("EQUITY".into(), 2, 1, Decimal::from(110),),
            ("EUR".into(), 2, 3, Decimal::from(30),),
            ("FOO".into(), 1, 2, Decimal::from(20),),
            ("GOLD".into(), 2, 5, Decimal::from(60),),
            ("OTHER".into(), 1, 0, Decimal::from(70),),
            ("TIE".into(), 1, 2, Decimal::from(90),),
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

    let uuidv7_relation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset
         WHERE SUBSTRING(id::text FROM 15 FOR 1) = '7'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(uuidv7_relation_count, 8);

    let uuidv7_shared_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM assets_data
         WHERE SUBSTRING(id::text FROM 15 FOR 1) = '7'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(uuidv7_shared_count, 8);

    Ok(())
}

#[sqlx::test(migrations = false, fixtures("legacy_schema", "legacy_shared_assets"))]
async fn shared_assets_are_reused_across_portfolios(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN legacy rows for the same provider and symbol in several Portfolios,
    // WHEN the corrective migration runs, THEN it creates one shared asset and
    // keeps one relationship per Portfolio with its own holding values.
    MIGRATOR.run(&pool).await?;

    let shared_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets_data")
        .fetch_one(&pool)
        .await?;
    let relationship_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset")
        .fetch_one(&pool)
        .await?;
    assert_eq!(shared_count, 4);
    assert_eq!(relationship_count, 7);

    let canonical: (String, String, i16, i16) = sqlx::query_as(
        "SELECT symbol, name, provider, asset_class
         FROM assets_data
         WHERE provider = 1 AND symbol = 'BTC'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(canonical, ("BTC".into(), "Bitcoin".into(), 1, 4));

    let btc_rows: Vec<(Uuid, Decimal, Option<i16>)> = sqlx::query_as(
        "SELECT pa.id, pa.manual_price, pa.asset_class_override
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 1 AND ad.symbol = 'BTC'
         ORDER BY pa.portfolio_id",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(btc_rows.len(), 2);
    assert_eq!(btc_rows[0].1, Decimal::from(100));
    assert_eq!(btc_rows[1].1, Decimal::from(500));
    assert_eq!(btc_rows[0].2, None);
    assert_eq!(btc_rows[1].2, Some(1));
    assert!(
        btc_rows
            .iter()
            .all(|(id, _, _)| id.to_string().as_bytes()[14] == b'7')
    );

    let metadata_conflict_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 1 AND ad.symbol = 'BTC' AND ad.name = 'Bitcoin'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(metadata_conflict_count, 2);

    let provider_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets_data WHERE symbol = 'BTC' AND provider = 2",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(provider_count, 0);

    let duplicate_constraint_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM pg_constraint
         WHERE conrelid = 'portfolio_asset'::regclass
           AND conname = 'portfolio_asset_portfolio_assets_data_key'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(duplicate_constraint_count, 1);

    Ok(())
}

#[sqlx::test(migrations = false, fixtures("legacy_schema", "legacy_shared_assets"))]
async fn shared_asset_migration_has_a_development_down_path_and_can_rerun(
    pool: PgPool,
) -> sqlx::Result<()> {
    // GIVEN the corrective migration has run, WHEN development resets only
    // that migration and reapplies it, THEN the relationship data remains
    // recoverable and the migration can be rerun cleanly.
    MIGRATOR.run(&pool).await?;
    MIGRATOR.undo(&pool, 20260814000000).await?;

    let assets_table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('assets_data')::text")
            .fetch_one(&pool)
            .await?;
    assert_eq!(assets_table, None);

    MIGRATOR.run(&pool).await?;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT
             (SELECT COUNT(*) FROM assets_data),
             (SELECT COUNT(*) FROM portfolio_asset)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(counts, (4, 7));

    Ok(())
}
