//! Wire types of `POST /import/portfolio`.

use serde::Serialize;
use utoipa::ToSchema;

use crate::ports::outbound::repository::ImportedPortfolio;

#[derive(Debug, Serialize, ToSchema)]
/// Metadata returned after a portfolio is stored for sharing.
pub struct ImportPortfolioResponse {
    pub id: String,
    pub expires_at: String,
}

impl From<ImportedPortfolio> for ImportPortfolioResponse {
    fn from(value: ImportedPortfolio) -> Self {
        Self {
            id: value.id.simple().to_string(),
            expires_at: value.expires_at.to_string(),
        }
    }
}
