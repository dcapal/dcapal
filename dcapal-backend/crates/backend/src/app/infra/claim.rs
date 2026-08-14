use axum::{RequestPartsExt, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{
    Algorithm, DecodingKey, TokenData, Validation, decode_header,
    errors::ErrorKind,
    jwk::{JwkSet, KeyAlgorithm},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppContext, error::DcaError};

const JWT_AUDIENCE_DOMAIN: &str = "authenticated";

fn validation_for(algorithm: Algorithm) -> Validation {
    let mut validation = Validation::new(algorithm);
    validation.set_audience(&[JWT_AUDIENCE_DOMAIN.to_string()]);
    validation
}

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
        jwt_secret: &str,
        jwks: &JwkSet,
    ) -> Result<TokenData<Self>, jsonwebtoken::errors::Error> {
        let header = decode_header(token)?;
        let key = match header.alg {
            Algorithm::HS256 => DecodingKey::from_secret(jwt_secret.as_ref()),
            Algorithm::ES256 => {
                let kid = header.kid.ok_or(ErrorKind::InvalidToken)?;
                let jwk = jwks.find(&kid).ok_or(ErrorKind::InvalidToken)?;
                if jwk.common.key_algorithm != Some(KeyAlgorithm::ES256) {
                    return Err(ErrorKind::InvalidAlgorithm.into());
                }
                DecodingKey::from_jwk(jwk)?
            }
            _ => return Err(ErrorKind::InvalidAlgorithm.into()),
        };

        jsonwebtoken::decode::<Self>(token, &key, &validation_for(header.alg))
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
        let auth = &state.config.app.auth;
        let user_claims = Claims::decode(bearer.token(), &auth.jwt_secret, &auth.jwt_jwks)?.claims;
        let _ = state
            .repos
            .user
            .save_user_if_not_present(&user_claims)
            .await?;
        Ok(user_claims)
    }
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{
        EncodingKey, Header, encode,
        jwk::{Jwk, JwkSet, PublicKeyUse},
    };

    use super::*;

    fn sample_claims() -> Claims {
        Claims {
            iat: 1_700_000_000,
            exp: 4_000_000_000,
            sub: Uuid::from_u128(1),
            session_id: Uuid::from_u128(2),
            role: "authenticated".to_string(),
            aud: JWT_AUDIENCE_DOMAIN.to_string(),
            user_metadata: UserMetadataClaim {
                email: "smoke@example.com".to_string(),
                full_name: Some("Smoke User".to_string()),
            },
        }
    }

    #[test]
    fn decodes_hs256_with_the_shared_secret() {
        let claims = sample_claims();
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test-secret"),
        )
        .unwrap();

        let decoded = Claims::decode(&token, "test-secret", &JwkSet { keys: Vec::new() }).unwrap();

        assert_eq!(decoded.claims.user_metadata.email, "smoke@example.com");
    }

    #[test]
    fn decodes_es256_with_the_matching_jwk() {
        let claims = sample_claims();
        let token = signed_es256_token(Some("test-key"), &claims);
        let decoded = Claims::decode(&token, "unused-secret", &test_jwks("test-key")).unwrap();

        assert_eq!(decoded.claims.sub, claims.sub);
    }

    #[test]
    fn rejects_es256_without_a_matching_key_id() {
        let token = signed_es256_token(Some("test-key"), &sample_claims());

        assert!(Claims::decode(&token, "unused-secret", &test_jwks("different-key"),).is_err());
    }

    #[test]
    fn rejects_es256_without_a_key_id() {
        let token = signed_es256_token(None, &sample_claims());

        assert!(Claims::decode(&token, "unused-secret", &test_jwks("test-key"),).is_err());
    }

    #[test]
    fn rejects_a_malformed_jwk() {
        let token = signed_es256_token(Some("test-key"), &sample_claims());
        let malformed_jwks = serde_json::from_value(serde_json::json!({
            "keys": [{
                "alg": "ES256",
                "crv": "P-256",
                "kid": "test-key",
                "kty": "EC",
                "x": "!",
                "y": "wQg1EytcsEmGrM70Gb53oluoDbVhCZ3Uq3hHMslHVb4"
            }]
        }))
        .unwrap();

        assert!(Claims::decode(&token, "unused-secret", &malformed_jwks).is_err());
    }

    #[test]
    fn rejects_an_unsupported_algorithm() {
        let token = encode(
            &Header::new(Algorithm::HS384),
            &sample_claims(),
            &EncodingKey::from_secret(b"test-secret"),
        )
        .unwrap();

        let error = Claims::decode(&token, "test-secret", &JwkSet { keys: Vec::new() })
            .expect_err("HS384 must not be accepted");

        assert!(matches!(error.kind(), ErrorKind::InvalidAlgorithm));
    }

    fn signed_es256_token(kid: Option<&str>, claims: &Claims) -> String {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = kid.map(str::to_owned);

        encode(
            &header,
            claims,
            &EncodingKey::from_ec_pem(EC_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    fn test_jwks(kid: &str) -> JwkSet {
        let encoding_key = EncodingKey::from_ec_pem(EC_PRIVATE_KEY.as_bytes()).unwrap();
        let mut jwk = Jwk::from_encoding_key(&encoding_key, Algorithm::ES256).unwrap();
        jwk.common.key_id = Some(kid.to_string());
        jwk.common.public_key_use = Some(PublicKeyUse::Signature);
        JwkSet { keys: vec![jwk] }
    }

    const EC_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWTFfCGljY6aw3Hrt\n\
kHmPRiazukxPLb6ilpRAewjW8nihRANCAATDskChT+Altkm9X7MI69T3IUmrQU0L\n\
950IxEzvw/x5BMEINRMrXLBJhqzO9Bm+d6JbqA21YQmd1Kt4RzLJR1W+\n\
-----END PRIVATE KEY-----\n";
}
