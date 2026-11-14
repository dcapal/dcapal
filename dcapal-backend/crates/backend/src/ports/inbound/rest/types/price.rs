//! Wire types of `GET /price/{asset}`.

use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
/// Quote-currency parameter for a conversion-price request.
pub struct GetPriceQuery {
    /// Quote currency.
    pub quote: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
/// Base-asset parameter for a conversion-price request.
pub struct GetPricePath {
    /// Base asset symbol.
    pub asset: String,
}
