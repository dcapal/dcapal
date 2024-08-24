use crate::app::domain::entity::{InvestmentPreferences, User};
use crate::app::infra::claim::Claims;
use crate::app::infra::utils::create_date_response;
use crate::error::Result;
use crate::AppContext;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use garde::Validate;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::info;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponse {
    pub name: String,
    pub email: String,
    pub birth_date: String,
}

impl From<User> for UserProfileResponse {
    fn from(entity: User) -> Self {
        Self {
            name: entity.name,
            email: entity.email,
            birth_date: create_date_response(entity.birth_date),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct UserInvestmentPreferencesResponse {
    pub risk_tolerance: String,
    pub investment_horizon: i32,
    pub investment_mode: String,
    pub investment_goal: String,
    pub ai_enabled: bool,
}

impl From<InvestmentPreferences> for UserInvestmentPreferencesResponse {
    fn from(entity: InvestmentPreferences) -> Self {
        Self {
            risk_tolerance: entity.risk_tolerance.to_string(),
            investment_horizon: entity.investment_horizon,
            investment_mode: entity.investment_mode.to_string(),
            investment_goal: entity.investment_goal.to_string(),
            ai_enabled: entity.ai_enabled,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[garde(skip)]
    pub name: String,
    #[garde(length(min = 8))]
    pub birth_date: String,
    #[garde(email)]
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate, Clone)]
pub struct UpdateUserInvestmentPreferencesRequest {
    #[garde(skip)]
    pub risk_tolerance: String,
    #[garde(range(min = 1, max = 50))]
    pub investment_horizon: i32,
    #[garde(skip)]
    pub investment_mode: String,
    #[garde(skip)]
    pub investment_goal: String,
    #[garde(skip)]
    pub ai_enabled: bool,
}

impl Into<InvestmentPreferences> for UpdateUserInvestmentPreferencesRequest {
    fn into(self) -> InvestmentPreferences {
        InvestmentPreferences {
            risk_tolerance: crate::app::domain::entity::RiskTolerance::from_str(
                &self.risk_tolerance,
            )
            .unwrap(),
            investment_horizon: self.investment_horizon,
            investment_mode: crate::app::domain::entity::InvestmentMode::from_str(
                &self.investment_mode,
            )
            .unwrap(),
            investment_goal: crate::app::domain::entity::InvestmentGoal::from_str(
                &self.investment_goal,
            )
            .unwrap(),
            ai_enabled: self.ai_enabled,
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

/// Get user investment settings.
#[utoipa::path(
    get,
    path = "/api/v1/user/investment-preferences",
    responses(
        (status = 200, description = "Success get user profile", body = [ProfileResponse]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn get_investment_preferences(
    State(ctx): State<AppContext>,
    claims: Claims,
) -> Result<Response> {
    match &ctx
        .services
        .user
        .get_investment_preferences(claims.sub)
        .await?
    {
        Some(user) => {
            Ok(Json(UserInvestmentPreferencesResponse::from(user.clone())).into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct MessageResponse {
    pub message: String,
}

impl MessageResponse {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Update user profile.
#[utoipa::path(
    put,
    path = "/api/v1/user/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Success update profile information", body = [MessageResponse]),
        (status = 400, description = "Invalid data input", body = [AppResponseError]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn update_profile(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Response> {
    info!("Update profile user_id: {}.", claims.sub);
    match &ctx.services.user.update_profile(claims.sub, req).await {
        Ok(_) => {
            info!("Success update profile user user_id: {}.", claims.sub);
            Ok(Json(MessageResponse::new("User profile updated.")).into_response())
        }
        Err(e) => {
            info!("Unsuccessful update profile user: {e:?}");
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}

/// Update user investment preferences.
#[utoipa::path(
    put,
    path = "/api/v1/user/investment-preferences",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Success update profile information", body = [MessageResponse]),
        (status = 400, description = "Invalid data input", body = [AppResponseError]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn update_investment_preferences(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<UpdateUserInvestmentPreferencesRequest>,
) -> Result<Response> {
    info!("Update investment preferences for user_id: {}.", claims.sub);
    match &ctx
        .services
        .user
        .update_investment_preferences(
            claims.sub,
            crate::ports::inbound::rest::user::UpdateUserInvestmentPreferencesRequest::into(req),
        )
        .await
    {
        Ok(_) => {
            info!("Success update profile user user_id: {}.", claims.sub);
            Ok(Json(MessageResponse::new("User profile updated.")).into_response())
        }
        Err(e) => {
            info!("Unsuccessful update profile user: {e:?}");
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}
