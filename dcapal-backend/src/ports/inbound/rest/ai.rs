use crate::app::domain::entity::Ai;
use crate::app::infra::claim::Claims;
use crate::ports::inbound::rest::portfolio::PortfolioRequest;
use crate::AppContext;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct AiResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AiRequest {
    pub message: String,
    pub portfolio: PortfolioRequest,
}

impl From<Ai> for AiResponse {
    fn from(entity: Ai) -> Self {
        Self {
            message: entity.response,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/ai/chatbot",
    responses(
        (status = 200, description = "Success get user profile", body = [ProfileResponse]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn get_chatbot_advice(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<AiRequest>,
) -> crate::error::Result<Response> {
    match &ctx
        .services
        .ai
        .get_ai_response(claims.sub, req.message, req.portfolio.into())
        .await?
    {
        Some(response) => Ok(Json(AiResponse::from(response.clone())).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
