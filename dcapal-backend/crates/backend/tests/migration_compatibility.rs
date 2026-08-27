use migration::{MIGRATOR, run_migrations};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

const FORMER_SHARED_ASSET_MIGRATION_CHECKSUM: &str = "37b5e148d245306df7d158e6e92ace259ba6c8030063df075b60ec10ac16d62317ff6ec123a56118b65e94b47826327d";

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
    assert_eq!(migration_count, 8);

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

    let foo_asset: (Uuid, String, Decimal) = sqlx::query_as(
        "SELECT pa.id, ad.name, pa.quantity
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 1 AND ad.symbol = 'FOO'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(foo_asset.1, "Foo oldest");
    assert_eq!(foo_asset.2, Decimal::from(2));
    assert_eq!(foo_asset.0.get_version_num(), 7);

    let reference_asset_id: Uuid =
        sqlx::query_scalar("SELECT portfolio_asset_id FROM portfolio_asset_reference")
            .fetch_one(&pool)
            .await?;
    assert_eq!(reference_asset_id, foo_asset.0);

    let unchanged_user_id: Uuid = sqlx::query_scalar("SELECT id FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        unchanged_user_id,
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    );
    let unchanged_portfolio_id: Uuid = sqlx::query_scalar("SELECT id FROM portfolios")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        unchanged_portfolio_id,
        Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap()
    );

    let asset_id_position: i32 = sqlx::query_scalar(
        "SELECT ordinal_position::int
         FROM information_schema.columns
         WHERE table_name = 'portfolio_asset' AND column_name = 'id'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(asset_id_position, 1);

    let unique_constraint_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM pg_constraint
             WHERE conrelid = 'portfolio_asset'::regclass
               AND conname = 'portfolio_asset_portfolio_assets_data_key'
         )",
    )
    .fetch_one(&pool)
    .await?;
    assert!(unique_constraint_exists);

    let duplicate_insert = sqlx::query(
        "INSERT INTO portfolio_asset (
             id, portfolio_id, assets_data_id, quantity, target_weight
         )
         SELECT $1, portfolio_id, assets_data_id, quantity, target_weight
         FROM portfolio_asset
         WHERE id = $2",
    )
    .bind(Uuid::now_v7())
    .bind(foo_asset.0)
    .execute(&pool)
    .await;
    assert!(duplicate_insert.is_err());

    let ids_before_rerun: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM portfolio_asset ORDER BY id")
            .fetch_all(&pool)
            .await?;

    // GIVEN a successfully applied migration, WHEN SQLx runs it again,
    // THEN it skips the recorded migration and preserves the normalized rows.
    MIGRATOR.run(&pool).await?;
    let rerun_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset")
        .fetch_one(&pool)
        .await?;
    assert_eq!(rerun_count, 8);

    let ids_after_rerun: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM portfolio_asset ORDER BY id")
            .fetch_all(&pool)
            .await?;
    assert_eq!(ids_after_rerun, ids_before_rerun);

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

    let portfolio_asset_default: Option<String> = sqlx::query_scalar(
        "SELECT column_default
         FROM information_schema.columns
         WHERE table_name = 'portfolio_asset' AND column_name = 'id'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(portfolio_asset_default, None);

    let assets_data_default: Option<String> = sqlx::query_scalar(
        "SELECT column_default
         FROM information_schema.columns
         WHERE table_name = 'assets_data' AND column_name = 'id'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(assets_data_default, None);

    Ok(())
}

#[sqlx::test(
    migrations = false,
    fixtures("legacy_schema", "legacy_portfolio_assets")
)]
async fn failed_identity_migration_rolls_back_its_changes(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN the shared-asset migration has already completed, WHEN an
    // unexpected failure occurs while the identity migration rewrites a
    // non-v7 ID, THEN its ID and dependent-reference changes roll back.
    MIGRATOR.run_to(20260826000000, &pool).await?;

    let child_asset_id = Uuid::parse_str("40000000-0000-0000-0000-000000000001").unwrap();
    let child_shared_id: Uuid = sqlx::query_scalar(
        "INSERT INTO assets_data (
             id, provider, symbol, name, currency, asset_class
         )
         VALUES (uuidv7(), 99, 'CHILD', 'Child Asset', 'EUR', 1)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO portfolio_asset (
             id, portfolio_id, assets_data_id, quantity, target_weight,
             average_buy_price, created_at, updated_at
         )
         SELECT $1, id, $2, 1, 0, NULL, NOW(), NOW()
         FROM portfolios
         LIMIT 1",
    )
    .bind(child_asset_id)
    .bind(child_shared_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE child_identity_reference (
             portfolio_asset_id UUID NOT NULL,
             CONSTRAINT child_identity_reference_asset_fk
                 FOREIGN KEY (portfolio_asset_id) REFERENCES portfolio_asset (id)
         )",
    )
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO child_identity_reference VALUES ($1)")
        .bind(child_asset_id)
        .execute(&pool)
        .await?;

    sqlx::query(
        "CREATE FUNCTION reject_portfolio_asset_id_rewrite()
         RETURNS trigger AS $$
         BEGIN
             RAISE EXCEPTION 'test identity rewrite failure';
         END;
         $$ LANGUAGE plpgsql;",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TRIGGER reject_portfolio_asset_id_rewrite
         BEFORE UPDATE OF id ON portfolio_asset
         FOR EACH ROW EXECUTE FUNCTION reject_portfolio_asset_id_rewrite();",
    )
    .execute(&pool)
    .await?;

    assert!(MIGRATOR.run(&pool).await.is_err());

    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(migration_count, 7);

    let unchanged_child_id: Uuid =
        sqlx::query_scalar("SELECT id FROM portfolio_asset WHERE id = $1")
            .bind(child_asset_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(unchanged_child_id, child_asset_id);
    let reference_id: Uuid =
        sqlx::query_scalar("SELECT portfolio_asset_id FROM child_identity_reference")
            .fetch_one(&pool)
            .await?;
    assert_eq!(reference_id, child_asset_id);

    let child_migration_applied: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM _sqlx_migrations WHERE version = 20260827000000
         )",
    )
    .fetch_one(&pool)
    .await?;
    assert!(!child_migration_applied);

    Ok(())
}

#[sqlx::test(
    migrations = false,
    fixtures("legacy_schema", "legacy_portfolio_assets")
)]
async fn identity_migration_rewires_dependent_foreign_keys(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN the shared-asset migration has completed and a later row still
    // has a non-v7 ID, WHEN the identity migration runs, THEN it rewrites the
    // row and its dependent foreign-key reference together.
    MIGRATOR.run_to(20260826000000, &pool).await?;

    let old_asset_id = Uuid::parse_str("40000000-0000-0000-0000-000000000002").unwrap();
    let shared_asset_id: Uuid = sqlx::query_scalar(
        "INSERT INTO assets_data (
             id, provider, symbol, name, currency, asset_class
         )
         VALUES (uuidv7(), 99, 'CHILD_SUCCESS', 'Child Success', 'EUR', 1)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO portfolio_asset (
             id, portfolio_id, assets_data_id, quantity, target_weight,
             average_buy_price, created_at, updated_at
         )
         SELECT $1, id, $2, 1, 0, NULL, NOW(), NOW()
         FROM portfolios
         LIMIT 1",
    )
    .bind(old_asset_id)
    .bind(shared_asset_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE child_identity_reference_success (
             portfolio_asset_id UUID NOT NULL,
             CONSTRAINT child_identity_reference_success_asset_fk
                 FOREIGN KEY (portfolio_asset_id) REFERENCES portfolio_asset (id)
         )",
    )
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO child_identity_reference_success VALUES ($1)")
        .bind(old_asset_id)
        .execute(&pool)
        .await?;

    MIGRATOR.run(&pool).await?;

    let rewritten_asset_id: Uuid =
        sqlx::query_scalar("SELECT portfolio_asset_id FROM child_identity_reference_success")
            .fetch_one(&pool)
            .await?;
    assert_ne!(rewritten_asset_id, old_asset_id);
    assert_eq!(rewritten_asset_id.get_version_num(), 7);
    let stored_asset_id: Uuid = sqlx::query_scalar("SELECT id FROM portfolio_asset WHERE id = $1")
        .bind(rewritten_asset_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(stored_asset_id, rewritten_asset_id);

    Ok(())
}

#[sqlx::test(
    migrations = false,
    fixtures("legacy_schema", "legacy_portfolio_assets")
)]
async fn migration_runner_accepts_the_former_shared_asset_checksum(
    pool: PgPool,
) -> sqlx::Result<()> {
    // GIVEN a database in the former migration's completed state, including
    // rewritten IDs and its uniqueness/default settings, WHEN the migration
    // runner starts, THEN it accepts only that known checksum and proceeds
    // without rewriting those already-v7 IDs.
    MIGRATOR.run_to(20260826000000, &pool).await?;
    sqlx::query("UPDATE portfolio_asset SET id = uuidv7()")
        .execute(&pool)
        .await?;
    sqlx::query(
        "ALTER TABLE portfolio_asset
         ALTER COLUMN id SET DEFAULT uuidv7(),
         ADD CONSTRAINT portfolio_asset_portfolio_assets_data_key
             UNIQUE (portfolio_id, assets_data_id)",
    )
    .execute(&pool)
    .await?;
    let ids_before: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM portfolio_asset ORDER BY id")
        .fetch_all(&pool)
        .await?;
    sqlx::query(
        "UPDATE _sqlx_migrations
         SET checksum = decode($1, 'hex')
         WHERE version = 20260826000000",
    )
    .bind(FORMER_SHARED_ASSET_MIGRATION_CHECKSUM)
    .execute(&pool)
    .await?;

    run_migrations(&pool, None).await.unwrap();

    let child_migration_applied: bool = sqlx::query_scalar(
        "SELECT success
         FROM _sqlx_migrations
         WHERE version = 20260827000000",
    )
    .fetch_one(&pool)
    .await?;
    assert!(child_migration_applied);

    let ids_after: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM portfolio_asset ORDER BY id")
        .fetch_all(&pool)
        .await?;
    assert_eq!(ids_after, ids_before);

    let recorded_checksum: String = sqlx::query_scalar(
        "SELECT encode(checksum, 'hex')
         FROM _sqlx_migrations
         WHERE version = 20260826000000",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(recorded_checksum, FORMER_SHARED_ASSET_MIGRATION_CHECKSUM);

    Ok(())
}

#[sqlx::test(
    migrations = false,
    fixtures("legacy_schema", "legacy_portfolio_assets")
)]
async fn migration_runner_rejects_an_unexpected_checksum(pool: PgPool) -> sqlx::Result<()> {
    // GIVEN a database with an unknown checksum for a recorded migration,
    // WHEN the migration runner starts, THEN it preserves SQLx's mismatch
    // error instead of treating the change as a compatibility case.
    MIGRATOR.run_to(20260826000000, &pool).await?;
    sqlx::query(
        "UPDATE _sqlx_migrations
         SET checksum = decode(repeat('00', 48), 'hex')
         WHERE version = 20260826000000",
    )
    .execute(&pool)
    .await?;

    let error = run_migrations(&pool, None).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("migration 20260826000000 was previously applied but has been modified")
    );

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
