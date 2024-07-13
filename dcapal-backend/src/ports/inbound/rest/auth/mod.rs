use std::fmt::{Display, Pointer};

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppContext;

pub async fn protected(
    Extension(jwtauth): Extension<JWTAuthMiddleware>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let json_response = json!({
        "status":  "success",
        "data": serde_json::json!({
            "user": &jwtauth.user
        })
    });

    Ok(Json(json_response))
}

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Email: {}\nCompany:", self.email)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTAuthMiddleware {
    pub user: SupabaseUser,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub message: String,
}

pub async fn validate_jwt(
    State(state): State<AppContext>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let access_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_owned())
            } else {
                None
            }
        });

    let access_token = access_token.ok_or_else(|| {
        let error_response = ErrorResponse {
            status: "fail",
            message: "You are not logged in, please provide token".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(error_response))
    })?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]);

    let jwt_secret_key = state.config.app.auth.jwt_secret.clone();

    let token_data_valid = match decode::<Claims>(
        &access_token,
        &DecodingKey::from_secret(jwt_secret_key.as_ref()),
        &validation,
    ) {
        Ok(token_data) => token_data,
        Err(e) => {
            let error_response = ErrorResponse {
                status: "fail",
                message: format!("Failed to validate jwt token: {}", e),
            };
            return Err((StatusCode::UNAUTHORIZED, Json(error_response)));
        }
    };

    req.extensions_mut().insert(JWTAuthMiddleware {
        user: SupabaseUser {
            id: token_data_valid.claims.email.clone(),
            email: token_data_valid.claims.email.clone(),
        },
    });

    Ok(next.run(req).await)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SupabaseUser {
    id: String,
    email: String,
    // Add other fields as necessary
}

async fn verify_token(token: &str, api_key: &str) -> Result<Claims, reqwest::Error> {
    let client = Client::new();
    let user_info_url = "{supabase_url}/auth/v1/user";
    let response = client
        .get(user_info_url)
        .bearer_auth(token)
        .header("apikey", api_key)
        .send()
        .await?
        .json::<Claims>()
        .await?;

    Ok(response)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    email: String,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
}
