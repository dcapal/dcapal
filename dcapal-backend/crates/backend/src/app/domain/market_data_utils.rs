use jiff::Timestamp;
use tracing::{error, warn};

use super::{
    entity::{Market, Price},
    market_quote::{FailureKind, PriceFailure, validate_price},
};
use crate::{
    config::PriceProvider, error::Result as AppResult, ports::outbound::adapter::PriceProviders,
};

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

/// Fetches the latest price of a market from the given provider.
///
/// A price that is missing, zero, negative or non-finite is a failure, never a value:
/// the caller keeps the last known price and records why the refresh failed.
pub async fn fetch_market_price(
    market: &Market,
    providers: &PriceProviders,
    provider: PriceProvider,
) -> Result<Price, PriceFailure> {
    let now = Timestamp::now();
    // The provider is chosen explicitly: a failure is reported, never retried elsewhere.
    let fetched = match provider {
        PriceProvider::CryptoWatch => {
            classify_price(providers.cw.fetch_market_price(market, now).await)
        }
        PriceProvider::Kraken => providers.kraken.fetch_market_price(market, now).await,
        PriceProvider::Yahoo => providers.yahoo.fetch_market_price(market, now).await,
    };

    let outcome = fetched
        .and_then(validate_price)
        .map(|px| Price::new(px, now));
    match &outcome {
        Ok(_) => {}
        Err(failure) if failure.kind == FailureKind::Unavailable => {
            warn!(mkt = market.id, "Price not available (ts={now}): {failure}");
        }
        Err(failure) => {
            error!(mkt = market.id, "Cannot fetch price (ts={now}): {failure}");
        }
    }
    outcome
}

/// Turns the raw answer of a price adapter into a validated price or a visible failure.
fn classify_price(fetched: AppResult<Option<f64>>) -> Result<f64, PriceFailure> {
    match fetched {
        Ok(Some(px)) => validate_price(px),
        Ok(None) => Err(PriceFailure::unavailable(
            "provider has no price for the market",
        )),
        Err(e) => Err(PriceFailure::provider_error(format!("{e:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::domain::entity::{Asset, Crypto, Fiat},
        error::DcaError,
    };

    /// GIVEN adapter answers that are a price, no price, an error and a zero price,
    /// WHEN they are classified,
    /// THEN only the positive price is a value; the rest are failures of distinct kinds.
    #[test]
    fn adapter_answers_are_classified_into_prices_and_failures() {
        assert_eq!(classify_price(Ok(Some(2.5))).unwrap(), 2.5);
        assert_eq!(
            classify_price(Ok(None)).unwrap_err().kind,
            FailureKind::Unavailable
        );
        assert_eq!(
            classify_price(Ok(Some(0.0))).unwrap_err().kind,
            FailureKind::Unavailable
        );
        assert_eq!(
            classify_price(Err(DcaError::Generic("boom".into())))
                .unwrap_err()
                .kind,
            FailureKind::ProviderError
        );
    }

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

    /// GIVEN a Kraken that fails and a Yahoo that would answer any request,
    /// WHEN the price of a crypto market is fetched from Kraken,
    /// THEN the failure is reported as is and Yahoo is never contacted.
    #[tokio::test]
    async fn a_failing_provider_is_never_replaced_by_another() {
        use std::sync::Arc;

        use crate::{
            ports::outbound::adapter::{
                CryptoWatchProvider, IpApi, KrakenProvider, PriceProviders, YahooProvider,
            },
            test_support::{FakeProvider, Reply, install_crypto_provider},
        };

        install_crypto_provider();
        let kraken = FakeProvider::start(|_| Reply::status(503, "down")).await;
        let yahoo = FakeProvider::start(|_| {
            Reply::ok(
                r#"{"chart":{"result":[{"indicators":{"quote":[{"close":[1.0]}]}}],"error":null}}"#,
            )
        })
        .await;
        let config: crate::config::Providers = serde_json::from_value(serde_json::json!({
            "priceProvider": "kraken",
            "cwApiKey": "",
            "ipApiKey": "",
            "krakenBaseUrl": kraken.base_url(),
        }))
        .unwrap();
        let http = reqwest::Client::new();
        let providers = PriceProviders {
            cw: Arc::new(CryptoWatchProvider::new(http.clone(), &config)),
            kraken: Arc::new(KrakenProvider::new(http.clone(), &config)),
            yahoo: Arc::new(
                YahooProvider::new(rquest::Client::new()).with_base_url(&yahoo.base_url()),
            ),
            ipapi: Arc::new(IpApi::new(http, &config)),
        };
        let btc = Asset::Crypto(Crypto::new_with_id("btc".into()));
        let eur = Asset::Fiat(Fiat::new("eur".into(), "Euro".into()));

        let failure = fetch_market_price(&market(btc, eur), &providers, PriceProvider::Kraken)
            .await
            .unwrap_err();

        assert_eq!(failure.kind, FailureKind::ProviderError);
        assert!(yahoo.requests().is_empty());
        assert!(!kraken.requests().is_empty());
    }
}
