use std::fmt::Display;

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppContext;

const JWT_AUDIENCE_DOMAIN: &str = "authenticated";

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

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Email: {}\nCompany:", self.email)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTAuthMiddleware {
    user: SupabaseUser,
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
    validation.set_audience(&[JWT_AUDIENCE_DOMAIN.to_string()]);

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    email: String,
}
