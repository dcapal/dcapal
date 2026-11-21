use jiff::Timestamp;
use uuid::Uuid;

/// A row from the `users` table.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct UserRow {
    /// The authenticated user's identifier.
    pub id: Uuid,
    /// The user's optional display name.
    pub username: Option<String>,
    /// The user's email address.
    pub email: String,
    /// The role assigned to the user.
    pub role: String,
    /// When the database row was created.
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub created_at: Timestamp,
    /// When the database row was last changed.
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub updated_at: Timestamp,
}
