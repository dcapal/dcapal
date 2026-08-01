use std::env;

use anyhow::{Context, Result, bail};
use migration::MIGRATOR;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = database_url()?;
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .with_context(|| format!("failed to connect to PostgreSQL at {database_url}"))?;

    MIGRATOR.run(&pool).await?;
    pool.close().await;

    Ok(())
}

fn database_url() -> Result<String> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.as_slice() {
        [] => env::var("DATABASE_URL").context("DATABASE_URL is not set"),
        [command, flag, url] if command == "up" && flag == "-u" => Ok(url.clone()),
        _ => bail!("usage: migration [up -u <database-url>]; omit arguments to use DATABASE_URL"),
    }
}
