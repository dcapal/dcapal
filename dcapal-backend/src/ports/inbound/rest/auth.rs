use std::fmt::Display;

use serde::{Deserialize, Serialize};

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
