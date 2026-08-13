use axum::{RequestPartsExt, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppContext, error::DcaError};

const JWT_AUDIENCE_DOMAIN: &str = "authenticated";
pub static DECODE_HEADER: Lazy<Validation> = Lazy::new(|| {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[JWT_AUDIENCE_DOMAIN.to_string()]);
    validation
});

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    // issued at
    pub iat: i64,
    // expiration
    pub exp: usize,
    // subject
    pub sub: Uuid,
    // session id
    pub session_id: Uuid,
    // role user
    pub role: String,
    // audience
    pub aud: String,
    // user metadata
    pub user_metadata: UserMetadataClaim,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMetadataClaim {
    pub email: String,
    pub full_name: Option<String>,
}

impl Claims {
    pub fn decode(
        token: &str,
        key: &DecodingKey,
    ) -> Result<TokenData<Self>, jsonwebtoken::errors::Error> {
        jsonwebtoken::decode::<Claims>(token, key, &DECODE_HEADER)
    }
}

impl FromRequestParts<AppContext> for Claims {
    type Rejection = DcaError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let jwt_secret = state.config.app.auth.jwt_secret.clone();
        let user_claims = Claims::decode(
            bearer.token(),
            &DecodingKey::from_secret(jwt_secret.as_ref()),
        )?
        .claims;
        let _ = state
            .repos
            .user
            .save_user_if_not_present(&user_claims)
            .await?;
        Ok(user_claims)
    }
}
