use std::env;

use anyhow::{Context, Result, bail};
use migration::run_migrations;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let (database_url, target_version) = migration_options()?;
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .with_context(|| format!("failed to connect to PostgreSQL at {database_url}"))?;

    run_migrations(&pool, target_version).await?;
    pool.close().await;

    Ok(())
}

fn migration_options() -> Result<(String, Option<i64>)> {
    let args: Vec<String> = env::args().skip(1).collect();

    // Keep the explicit URL form for operators who used the previous migration command.
    match args.as_slice() {
        [] => Ok((
            env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            None,
        )),
        [command, flag, url] if command == "up" && flag == "-u" => Ok((url.clone(), None)),
        [command, flag, version] if command == "up-to" && flag == "-v" => Ok((
            env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            Some(
                version
                    .parse()
                    .context("migration version must be an integer")?,
            ),
        )),
        _ => bail!(
            "usage: migration [up -u <database-url>] | migration up-to -v <version>; omit arguments to use DATABASE_URL"
        ),
    }
}
