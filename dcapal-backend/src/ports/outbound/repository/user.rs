use async_trait::async_trait;

use crate::{
    app::infra::claim::Claims, error::Result, ports::outbound::repository::postgres::types::UserRow,
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save_user_if_not_present(&self, claims: &Claims) -> Result<UserRow>;
}
