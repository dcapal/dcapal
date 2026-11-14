//! Wire types of `GET /import/portfolio/{id}`.

use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
/// Path parameters identifying a stored shared portfolio.
pub struct GetImportedPortfolioPath {
    /// Imported portfolio id.
    pub id: String,
}
