//! PostgreSQL repository implementations and their persistence row types.

/// Portfolio persistence backed by PostgreSQL.
pub mod portfolio;
/// PostgreSQL row representations used by the repository interfaces.
pub mod types;
/// User persistence backed by PostgreSQL.
pub mod user;

pub use portfolio::SqlxPortfolioRepository;
pub use user::SqlxUserRepository;
