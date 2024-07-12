use std::fmt::{Display, Pointer};

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
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

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    Keys::new(secret.as_ref())
});
pub async fn protected(claims: Claims) -> Result<String, AuthError> {
    // Send the protected data to the user
    Ok(format!(
        "Welcome to the protected area :)\nYour data:\n{claims}",
    ))
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
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["authenticated"]);

        let token_data_valid = decode::<Claims>(bearer.token(), &KEYS.decoding, &validation)
            .map_err(|e| {
                eprintln!("Failed to validate jwt token: {}", e);
                AuthError::InvalidToken
            })?;

        // Decode the user data through the Supabase API (may introduce latency)
        // let api_key = parts.headers.get("apikey").unwrap().to_str().unwrap();
        // let token_data = verify_token(bearer.token(), api_key)
        //  .await
        //.map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data_valid.claims)
    }
}

#[derive(Deserialize)]
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
