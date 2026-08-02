use async_trait::async_trait;

use crate::{
    app::infra::claim::Claims, error::Result, ports::outbound::repository::postgres::types::UserRow,
};

/// Persistence operations for application users.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Saves the claims for a user, creating the row when it does not exist.
    async fn save_user_if_not_present(&self, claims: &Claims) -> Result<UserRow>;
}
