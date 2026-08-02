/// The embedded SQLx migrations for the backend database.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");
