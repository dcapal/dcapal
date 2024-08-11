use crate::error::Result;

use crate::app::domain::entity::User;
use crate::ports::outbound::repository::user::UserRepository;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct UserService {
    user_repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(user_repository: Arc<UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn get_profile(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = self
            .user_repository
            .get_current_user(user_id)
            .await
            .map_err(|e| {
                error!("Failed to get user profile: {}", e);
                e
            })?;

        match user {
            Some(user) => {
                info!("Successfully retrieved user profile: {}", user_id);
                Ok(Some(user))
            }
            None => {
                warn!("User not found: {}", user_id);
                Ok(None)
            }
        }
    }
}
