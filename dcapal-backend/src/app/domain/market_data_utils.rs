use chrono::Utc;
use tracing::{error, warn};

use crate::{config::PriceProvider, ports::outbound::adapter::PriceProviders};

use super::entity::{Market, Price};

pub async fn fetch_market_price(
    market: &Market,
    providers: &PriceProviders,
    provider: PriceProvider,
) -> Option<Price> {
    let now = Utc::now();
    let price = match provider {
        PriceProvider::CryptoWatch => providers.cw.fetch_market_price(market, now).await,
        PriceProvider::Kraken => providers.kraken.fetch_market_price(market, now).await,
        PriceProvider::Yahoo => providers.yahoo.fetch_market_price(market, now).await,
    };

    match price {
        Ok(Some(px)) => Some(Price::new(px, now)),
        Ok(None) => {
            warn!(
                "Cannot fetch {} price for any frequency (ts={now})",
                market.id
            );
            None
        }
        Err(e) => {
            error!(
                "Cannot fetch {} price for any frequency (ts={now}): {e:?}",
                market.id
            );
            None
        }
    }
}
