use crate::app::domain::entity::User;
use crate::app::infra::claim::Claims;
use crate::error::Result;
use crate::AppContext;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use time::Date;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct UserProfileResponse {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: String,
    pub birth_date: Date,
}

impl From<User> for UserProfileResponse {
    fn from(entity: User) -> Self {
        Self {
            first_name: entity.first_name,
            last_name: entity.last_name,
            email: entity.email,
            birth_date: entity.birth_date,
        }
    }
}

/// Get user profile information.
#[utoipa::path(
    get,
    path = "/api/v1/user/profile",
    responses(
        (status = 200, description = "Success get user profile", body = [ProfileResponse]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn get_profile(State(ctx): State<AppContext>, claims: Claims) -> Result<Response> {
    match &ctx.services.user.get_profile(claims.sub).await? {
        Some(user) => Ok(Json(UserProfileResponse::from(user.clone())).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
