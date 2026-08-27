use std::borrow::Cow;

use sqlx::PgPool;
use sqlx::migrate::{MigrateError, Migration, Migrator};

/// The embedded SQLx migrations for the backend database.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

const SHARED_ASSET_MIGRATION_VERSION: i64 = 20260826000000;
const FORMER_SHARED_ASSET_MIGRATION_CHECKSUM: [u8; 48] = [
    0x37, 0xb5, 0xe1, 0x48, 0xd2, 0x45, 0x30, 0x6d, 0xf7, 0xd1, 0x58, 0xe6, 0xe9, 0x2a, 0xce, 0x25,
    0x9b, 0xa6, 0xc8, 0x03, 0x00, 0x63, 0xdf, 0x07, 0x5b, 0x60, 0xec, 0x10, 0xac, 0x16, 0xd6, 0x23,
    0x17, 0xff, 0x6e, 0xc1, 0x23, 0xa5, 0x61, 0x18, 0xb6, 0x5e, 0x94, 0xb4, 0x78, 0x26, 0x32, 0x7d,
];

/// Runs migrations while accepting the former shared-asset migration checksum.
///
/// The shared-asset migration was deployed once with an implementation that
/// also rewrote Portfolio Asset IDs. Its replacement keeps that migration's
/// current production behavior in source control and moves the identity work
/// to the following migration. Existing databases with the former checksum
/// must still reach that following migration without weakening SQLx's normal
/// checksum validation for any other change.
pub async fn run_migrations(pool: &PgPool, target_version: Option<i64>) -> anyhow::Result<()> {
    if former_checksum_is_recorded(pool).await? {
        let compatible_migrator = migrator_with_former_checksum();
        return run_migrator(&compatible_migrator, pool, target_version)
            .await
            .map_err(Into::into);
    }

    match run_migrator(&MIGRATOR, pool, target_version).await {
        Err(MigrateError::VersionMismatch(version))
            if version == SHARED_ASSET_MIGRATION_VERSION
                && former_checksum_is_recorded(pool).await? =>
        {
            let compatible_migrator = migrator_with_former_checksum();
            run_migrator(&compatible_migrator, pool, target_version)
                .await
                .map_err(Into::into)
        }
        result => result.map_err(Into::into),
    }
}

async fn run_migrator(
    migrator: &Migrator,
    pool: &PgPool,
    target_version: Option<i64>,
) -> Result<(), MigrateError> {
    let mut connection = pool.acquire().await.map_err(MigrateError::Execute)?;
    let result = migrator
        .run_direct(target_version, &mut *connection, false)
        .await;
    if result.is_err() {
        // SQLx holds its session advisory lock when run_direct returns an
        // error. Close the connection so a compatibility retry cannot inherit
        // that lock from the pool.
        let _ = connection.close().await;
    }

    result
}

async fn former_checksum_is_recorded(pool: &PgPool) -> sqlx::Result<bool> {
    let migrations_table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await?;
    if !migrations_table_exists {
        return Ok(false);
    }

    let checksum: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = $1 AND success")
            .bind(SHARED_ASSET_MIGRATION_VERSION)
            .fetch_optional(pool)
            .await?;

    Ok(checksum.as_deref() == Some(FORMER_SHARED_ASSET_MIGRATION_CHECKSUM.as_slice()))
}

fn migrator_with_former_checksum() -> Migrator {
    let migrations = MIGRATOR
        .iter()
        .cloned()
        .map(|mut migration| {
            if migration.version == SHARED_ASSET_MIGRATION_VERSION
                && migration.migration_type.is_up_migration()
            {
                migration.checksum = Cow::Owned(FORMER_SHARED_ASSET_MIGRATION_CHECKSUM.to_vec());
            }
            migration
        })
        .collect::<Vec<Migration>>();

    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: MIGRATOR.ignore_missing,
        locking: MIGRATOR.locking,
        no_tx: MIGRATOR.no_tx,
        table_name: MIGRATOR.table_name.clone(),
        create_schemas: MIGRATOR.create_schemas.clone(),
    }
}
