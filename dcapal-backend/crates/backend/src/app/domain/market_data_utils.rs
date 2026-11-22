use chrono::Utc;
use tracing::{error, warn};

use super::entity::{Market, Price};
use crate::{config::PriceProvider, ports::outbound::adapter::PriceProviders};

/// Chooses the provider that prices a market.
///
/// Fiat against fiat markets are priced by Yahoo through its hardcoded FX tickers;
/// every other market is priced by the configured crypto provider.
pub fn price_provider_for(market: &Market, crypto_provider: PriceProvider) -> PriceProvider {
    if market.is_fiat() {
        PriceProvider::Yahoo
    } else {
        crypto_provider
    }
}

/// Fetches the latest price of a market from the given provider, logging failures.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::domain::entity::{Asset, Crypto, Fiat};

    fn market(base: Asset, quote: Asset) -> Market {
        Market::new("m".to_string(), base, quote, None)
    }

    #[test]
    fn fiat_markets_are_priced_by_yahoo_and_others_by_the_crypto_provider() {
        // GIVEN an FX market and a crypto market,
        // WHEN the price provider is chosen with Kraken as crypto provider,
        // THEN the FX market uses Yahoo and the crypto market uses Kraken.
        let eur = Asset::Fiat(Fiat::new("eur".into(), "Euro".into()));
        let usd = Asset::Fiat(Fiat::new("usd".into(), "US Dollar".into()));
        let btc = Asset::Crypto(Crypto::new_with_id("btc".into()));

        assert!(matches!(
            price_provider_for(&market(eur.clone(), usd), PriceProvider::Kraken),
            PriceProvider::Yahoo
        ));
        assert!(matches!(
            price_provider_for(&market(btc, eur), PriceProvider::Kraken),
            PriceProvider::Kraken
        ));
    }
}
