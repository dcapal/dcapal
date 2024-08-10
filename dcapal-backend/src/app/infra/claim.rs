use crate::error::DcaError;
use crate::AppContext;
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::Serialize;

use uuid::Uuid;

pub static DECODE_HEADER: Lazy<Validation> = Lazy::new(|| Validation::new(Algorithm::HS256));
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    // issued at
    pub iat: i64,
    // expiration
    pub exp: usize,
    // subject
    pub sub: Uuid,
    // session id
    pub sid: Uuid,
    // role user
    pub role: String,
}

impl Claims {
    pub fn decode(
        token: &str,
        key: &DecodingKey,
    ) -> Result<TokenData<Self>, jsonwebtoken::errors::Error> {
        jsonwebtoken::decode::<Claims>(token, key, &DECODE_HEADER)
    }
}

#[async_trait]
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
        //service::session::check(&state.redis, &user_claims).await?;
        Ok(user_claims)
    }
}
