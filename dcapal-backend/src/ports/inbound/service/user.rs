use crate::error::Result;
use crate::ports::inbound::dto::response::UserProfileResponse;
use crate::ports::outbound::repository::user::get_current_user;
use crate::AppContext;
use axum::extract::State;
use uuid::Uuid;

pub async fn get_profile(
    State(ctx): State<AppContext>,
    user_id: Uuid,
) -> Result<UserProfileResponse> {
    let user = get_current_user(user_id, State(ctx)).await?;
    Ok(UserProfileResponse::from(user))
}
