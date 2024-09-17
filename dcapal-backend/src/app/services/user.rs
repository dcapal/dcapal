use crate::error::Result;

use crate::app::domain::entity::{InvestmentPreferences, User};
use crate::ports::inbound::rest::user::UpdateProfileRequest;
use crate::ports::outbound::repository::user::UserRepository;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
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

    pub async fn update_profile(&self, user_id: Uuid, req: UpdateProfileRequest) -> Result<()> {
        info!("Update user profile with id: {user_id} request: {req:?}");
        let _ = self
            .user_repository
            .update_current_user(user_id, req)
            .await
            .map_err(|e| {
                error!("Failed to get user profile: {}", e);
                e
            });
        Ok(())
    }

    pub async fn get_investment_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Option<InvestmentPreferences>> {
        let investment_preferences = self
            .user_repository
            .get_investment_preferences(user_id)
            .await
            .map_err(|e| {
                error!("Failed to get user profile: {}", e);
                e
            })?;

        match investment_preferences {
            Some(investment_preferences) => {
                info!(
                    "Successfully retrieved user investment preferences: {}",
                    user_id
                );
                Ok(Some(investment_preferences))
            }
            None => {
                warn!("Investment preferences not found for user_id: {}", user_id);
                Ok(None)
            }
        }
    }

    pub async fn upsert_investment_preferences(
        &self,
        user_id: Uuid,
        req: InvestmentPreferences,
    ) -> Result<()> {
        info!("Update user investment preferences with id: {user_id} request: {req:?}");
        let _ = self
            .user_repository
            .upsert_investment_preferences(user_id, req)
            .await
            .map_err(|e| {
                error!("Failed to get user profile: {}", e);
                e
            });
        Ok(())
    }
}
