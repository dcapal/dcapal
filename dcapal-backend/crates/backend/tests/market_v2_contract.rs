//! REST contract tests of the v2 market-data endpoints (`/markets/v2/assets/{provider}/search`,
//! `/markets/v2/prices` and `/markets/v2/fx`), run against a real server, the test database and
//! local fakes that stand in for Yahoo and Kraken.

use std::net::SocketAddr;

use jiff::Timestamp;
use serde_json::{Value, json};
use sqlx::PgPool;

mod common;

use common::{FakeProvider, ProviderUrls, Reply, bearer_token, serve};
use dcapal_backend::{
    app::domain::{
        entity::{Asset, Crypto, Fiat, Market, Price},
        market_quote::PriceFailure,
    },
    config::PriceProvider,
    ports::outbound::repository::market_data::{MarketDataRepository, MarketPriceFailure},
};

/// A response reduced to what the contract tests assert on.
struct Answer {
    status: u16,
    cache_control: Option<String>,
    body: Value,
}

async fn get(addr: SocketAddr, path: &str) -> Answer {
    let response = reqwest::Client::new()
        .get(format!("http://{addr}{path}"))
        .bearer_auth(bearer_token())
        .send()
        .await
        .unwrap();
    Answer {
        status: response.status().as_u16(),
        cache_control: response
            .headers()
            .get("cache-control")
            .map(|v| v.to_str().unwrap().to_string()),
        body: response.json().await.unwrap_or(Value::Null),
    }
}

fn fiat(id: &str) -> Asset {
    Asset::Fiat(Fiat::new(id.into(), id.to_uppercase()))
}

/// Stores `base`/`quote` with a fresh price, priced by `provider`.
async fn store_priced(
    repo: &MarketDataRepository,
    base: &Asset,
    quote: &Asset,
    price: f64,
    provider: PriceProvider,
) {
    for asset in [base, quote] {
        repo.store_asset(asset).await.unwrap();
    }
    let market = Market::new(
        dcapal_backend::app::domain::entity::market_id(base.id(), quote.id()),
        base.clone(),
        quote.clone(),
        Some(Price::new(price, Timestamp::now())),
    );
    repo.store_market(&market, provider).await.unwrap();
}

fn chart_meta(meta: &str) -> String {
    format!(r#"{{"chart":{{"result":[{{"meta":{meta}}}],"error":null}}}}"#)
}

/// A Yahoo fake answering searches with the given quotes and chart queries with `chart`.
async fn yahoo(
    search: Reply,
    chart: impl Fn(&str) -> Reply + Send + Sync + 'static,
) -> FakeProvider {
    FakeProvider::start(move |target| {
        if target.starts_with("/v1/finance/search") {
            search.clone()
        } else {
            chart(target)
        }
    })
    .await
}

async fn serve_with_yahoo(pool: &PgPool, yahoo: &FakeProvider) -> SocketAddr {
    common::install_crypto_provider();
    serve(
        pool,
        ProviderUrls {
            yahoo: Some(yahoo.base_url()),
            kraken: None,
        },
    )
    .await
}

/// GIVEN Yahoo answers a search with an ETF and a symbol-less entry,
/// WHEN `yf` assets are searched,
/// THEN the answer is `usable` with stable identity, name, venue, class and unknown price
/// availability, and the query reaches Yahoo percent-encoded.
#[sqlx::test(migrations = "../../migrations")]
async fn yahoo_search_returns_typed_assets(pool: PgPool) {
    let fake = yahoo(
        Reply::ok(
            r#"{"quotes":[{"symbol":"VWCE.DE","longname":"Vanguard FTSE All-World","exchDisp":"XETRA","quoteType":"ETF"},{"longname":"x"}]}"#,
        ),
        |_| Reply::status(404, "{}"),
    )
    .await;
    let addr = serve_with_yahoo(&pool, &fake).await;

    let answer = get(addr, "/markets/v2/assets/yf/search?query=vwce%20all").await;

    assert_eq!(answer.status, 200);
    assert_eq!(
        answer.body,
        json!({
            "provider": "yf", "query": "vwce all", "outcome": "usable", "error": null,
            "results": [{
                "provider": "yf", "symbol": "VWCE.DE", "name": "Vanguard FTSE All-World",
                "venue": "XETRA", "currency": null, "aclass": "equity",
                "priceAvailability": "unknown"
            }]
        })
    );
    assert!(fake.requests()[0].starts_with("/v1/finance/search?q=vwce+all&"));
}

/// GIVEN Yahoo answers an empty list, a server error, and garbage,
/// WHEN `yf` assets are searched,
/// THEN the outcomes are `empty`, `providerError` and `malformed`, each with HTTP 200 and the
/// failure visible in `error`.
#[sqlx::test(migrations = "../../migrations")]
async fn yahoo_search_failures_stay_visible(pool: PgPool) {
    for (reply, outcome, has_error) in [
        (Reply::ok(r#"{"quotes":[]}"#), "empty", false),
        (Reply::status(503, "down"), "providerError", true),
        (Reply::ok("<html>"), "malformed", true),
    ] {
        let fake = yahoo(reply, |_| Reply::status(404, "{}")).await;
        let addr = serve_with_yahoo(&pool, &fake).await;

        let answer = get(addr, "/markets/v2/assets/yf/search?query=x").await;

        assert_eq!(answer.status, 200, "{outcome}");
        assert_eq!(answer.body["outcome"], outcome);
        assert_eq!(answer.body["results"], json!([]));
        assert_eq!(!answer.body["error"].is_null(), has_error, "{outcome}");
    }
}

/// GIVEN a Kraken catalog with a priced and an unpriced asset and a Yahoo fake,
/// WHEN `kraken` assets are searched,
/// THEN the catalog answers with USD currency and price availability, and Yahoo is never asked.
#[sqlx::test(migrations = "../../migrations")]
async fn kraken_search_reads_the_catalog_and_never_asks_yahoo(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    let usd = fiat("usd");
    store_priced(
        &repo,
        &Asset::Crypto(Crypto::new_with_id("btc".into())),
        &usd,
        44_000.0,
        PriceProvider::Kraken,
    )
    .await;
    repo.store_asset(&Asset::Crypto(Crypto::new_with_id("btt".into())))
        .await
        .unwrap();
    let fake = yahoo(Reply::ok(r#"{"quotes":[]}"#), |_| Reply::status(404, "{}")).await;
    let addr = serve_with_yahoo(&pool, &fake).await;

    let answer = get(addr, "/markets/v2/assets/kraken/search?query=bt").await;

    assert_eq!(answer.status, 200);
    let found = answer.body["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["symbol"].as_str().unwrap(),
                r["priceAvailability"].as_str().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(found, [("BTC", "available"), ("BTT", "unavailable")]);
    assert_eq!(answer.body["results"][0]["currency"], "USD");
    assert_eq!(answer.body["results"][0]["aclass"], "crypto");
    assert!(fake.requests().is_empty());
}

/// GIVEN an unsupported provider, a blank query and a missing query,
/// WHEN the search is called,
/// THEN each is a 400 `validation_error` naming the offending field.
#[sqlx::test(migrations = "../../migrations")]
async fn search_rejects_unsupported_providers_and_blank_queries(pool: PgPool) {
    let addr = serve(&pool, ProviderUrls::default()).await;

    for (path, field) in [
        ("/markets/v2/assets/ibkr/search?query=x", "provider"),
        ("/markets/v2/assets/yf/search?query=%20%20", "query"),
        ("/markets/v2/assets/kraken/search", "query"),
    ] {
        let answer = get(addr, path).await;

        assert_eq!(answer.status, 400, "{path}");
        assert_eq!(answer.body["code"], "validation_error");
        assert_eq!(answer.body["errors"][0]["field"], field, "{path}");
    }
}

/// GIVEN a stored BTC/USD price, a Yahoo fake that quotes one ETF and fails another, and an
/// unknown Kraken asset,
/// WHEN their prices are requested together,
/// THEN each entry reports its own provider, currency, status and failure in request order,
/// nothing is converted, and the answer must be revalidated because some prices are missing.
#[sqlx::test(migrations = "../../migrations")]
async fn prices_report_each_asset_with_its_own_provenance(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    store_priced(
        &repo,
        &Asset::Crypto(Crypto::new_with_id("btc".into())),
        &fiat("usd"),
        44_000.25,
        PriceProvider::Kraken,
    )
    .await;
    let fake = yahoo(Reply::ok(r#"{"quotes":[]}"#), |target| {
        if target.contains("/VWCE.DE") {
            Reply::ok(chart_meta(
                r#"{"currency":"EUR","regularMarketPrice":131.2,"regularMarketTime":1700000000}"#,
            ))
        } else if target.contains("/BROKEN") {
            Reply::status(503, "down")
        } else {
            Reply::ok("<html>")
        }
    })
    .await;
    let addr = serve_with_yahoo(&pool, &fake).await;

    let answer = get(
        addr,
        "/markets/v2/prices?asset=kraken:btc&asset=yf:vwce.de&asset=yf:broken&asset=kraken:doge&asset=yf:junk",
    )
    .await;

    assert_eq!(answer.status, 200);
    let prices = answer.body["prices"].as_array().unwrap();
    let btc = &prices[0];
    assert_eq!(btc["asset"], json!({"provider": "kraken", "symbol": "BTC"}));
    assert_eq!(
        (
            btc["price"].as_str(),
            btc["currency"].as_str(),
            btc["status"].as_str(),
            btc["source"].as_str()
        ),
        (
            Some("44000.25"),
            Some("USD"),
            Some("usable"),
            Some("provider")
        )
    );
    assert!(btc["observedAt"].as_str().unwrap().ends_with('Z'));
    let etf = &prices[1];
    assert_eq!(
        (
            etf["price"].as_str(),
            etf["currency"].as_str(),
            etf["provider"].as_str()
        ),
        (Some("131.2"), Some("EUR"), Some("yf"))
    );
    assert_eq!(etf["observedAt"], "2023-11-14T22:13:20Z");
    assert_eq!(prices[2]["status"], "providerError");
    assert_eq!(prices[2]["price"], Value::Null);
    assert_eq!(prices[2]["error"]["kind"], "providerError");
    assert_eq!(prices[3]["status"], "unavailable");
    assert_eq!(prices[4]["status"], "malformed");
    assert_eq!(answer.cache_control.as_deref(), Some("no-cache"));
}

/// GIVEN a stored BTC/USD price and a recorded refresh failure,
/// WHEN the price is requested,
/// THEN the last known price is still returned, flagged `lastKnown` with the failure and its
/// time; a fresh answer for fresh data is privately cacheable for at most five minutes.
#[sqlx::test(migrations = "../../migrations")]
async fn last_known_price_is_flagged_after_a_failed_refresh(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    store_priced(
        &repo,
        &Asset::Crypto(Crypto::new_with_id("btc".into())),
        &fiat("usd"),
        44_000.25,
        PriceProvider::Kraken,
    )
    .await;
    let addr = serve(&pool, ProviderUrls::default()).await;

    let fresh = get(addr, "/markets/v2/prices?asset=kraken:btc").await;
    let cache = fresh.cache_control.unwrap();
    assert!(
        cache.contains("private") && cache.contains("max-age="),
        "{cache}"
    );
    assert_eq!(fresh.body["prices"][0]["source"], "provider");
    assert_eq!(fresh.body["prices"][0]["error"], Value::Null);

    let id = "btc/usd".to_string();
    let failure = PriceFailure::provider_error("Kraken answered HTTP 503");
    repo.record_price_failures(&[MarketPriceFailure {
        market_id: &id,
        failure: &failure,
    }])
    .await
    .unwrap();
    // A new server has an empty quote cache, as after the price worker invalidated it.
    let addr = serve(&pool, ProviderUrls::default()).await;
    let stale = get(addr, "/markets/v2/prices?asset=kraken:btc").await;

    let price = &stale.body["prices"][0];
    assert_eq!(
        (price["price"].as_str(), price["status"].as_str()),
        (Some("44000.25"), Some("usable"))
    );
    assert_eq!(price["source"], "lastKnown");
    assert_eq!(price["error"]["kind"], "providerError");
    // The recorded reason ("Kraken answered HTTP 503") never reaches the client.
    assert_eq!(
        price["error"]["message"],
        "The provider could not be reached or answered with an error."
    );
    assert!(price["error"]["occurredAt"].as_str().is_some());
}

/// GIVEN no references, malformed references and too many references,
/// WHEN prices are requested,
/// THEN each is a 400 `validation_error`.
#[sqlx::test(migrations = "../../migrations")]
async fn prices_reject_invalid_references(pool: PgPool) {
    let addr = serve(&pool, ProviderUrls::default()).await;
    let many = (0..21)
        .map(|i| format!("asset=yf:S{i}"))
        .collect::<Vec<_>>()
        .join("&");

    for path in [
        "/markets/v2/prices".to_string(),
        "/markets/v2/prices?asset=aapl".to_string(),
        "/markets/v2/prices?asset=ibkr:aapl".to_string(),
        format!("/markets/v2/prices?{many}"),
    ] {
        let answer = get(addr, &path).await;

        assert_eq!(answer.status, 400, "{path}");
        assert_eq!(answer.body["code"], "validation_error");
    }
}

/// GIVEN stored EUR/USD and GBP/USD rates,
/// WHEN FX rates are requested direct, inverted, triangulated and for one currency,
/// THEN each reports its derivation, legs, provider and timestamps with a cacheable answer;
/// crypto and missing currencies are 400, and a pair whose market was never priced is `pending`.
#[sqlx::test(migrations = "../../migrations")]
async fn fx_rates_report_their_derivation(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    store_priced(
        &repo,
        &fiat("eur"),
        &fiat("usd"),
        1.25,
        PriceProvider::Yahoo,
    )
    .await;
    store_priced(&repo, &fiat("gbp"), &fiat("usd"), 1.5, PriceProvider::Yahoo).await;
    let addr = serve(&pool, ProviderUrls::default()).await;

    let direct = get(addr, "/markets/v2/fx?base=EUR&quote=USD").await;
    assert_eq!(direct.status, 200);
    assert_eq!(direct.body["rate"], "1.25");
    assert_eq!(direct.body["synthetic"], "none");
    assert_eq!(direct.body["provider"], "yf");
    assert_eq!(direct.body["legs"][0]["base"], "EUR");
    assert!(direct.cache_control.unwrap().contains("max-age="));

    let inverse = get(addr, "/markets/v2/fx?base=usd&quote=eur").await;
    assert_eq!(
        (
            inverse.body["rate"].as_str(),
            inverse.body["synthetic"].as_str()
        ),
        (Some("0.8"), Some("inverse"))
    );

    let cross = get(addr, "/markets/v2/fx?base=EUR&quote=GBP").await;
    assert_eq!(cross.body["synthetic"], "triangulated");
    assert_eq!(cross.body["legs"].as_array().unwrap().len(), 2);
    assert_eq!(cross.body["status"], "usable");

    let same = get(addr, "/markets/v2/fx?base=EUR&quote=EUR").await;
    assert_eq!(same.body["rate"], "1");

    let none = get(addr, "/markets/v2/fx?base=EUR&quote=CHF").await;
    assert_eq!(none.status, 200);
    assert_eq!(
        (none.body["status"].as_str(), none.body["rate"].is_null()),
        (Some("pending"), true)
    );
    assert_eq!(none.cache_control.as_deref(), Some("no-cache"));

    for (path, field) in [
        ("/markets/v2/fx?base=BTC&quote=USD", "base"),
        ("/markets/v2/fx?base=EUR", "quote"),
    ] {
        let answer = get(addr, path).await;
        assert_eq!(answer.status, 400, "{path}");
        assert_eq!(answer.body["errors"][0]["field"], field);
    }
}

/// GIVEN an FX leg whose latest refresh failed,
/// WHEN its rate is requested,
/// THEN the last known rate is returned flagged `lastKnown` with the failure.
#[sqlx::test(migrations = "../../migrations")]
async fn fx_rate_flags_a_failed_refresh(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    store_priced(
        &repo,
        &fiat("eur"),
        &fiat("usd"),
        1.25,
        PriceProvider::Yahoo,
    )
    .await;
    let id = "eur/usd".to_string();
    let failure = PriceFailure::malformed("empty chart.result");
    repo.record_price_failures(&[MarketPriceFailure {
        market_id: &id,
        failure: &failure,
    }])
    .await
    .unwrap();
    let addr = serve(&pool, ProviderUrls::default()).await;

    let answer = get(addr, "/markets/v2/fx?base=EUR&quote=USD").await;

    assert_eq!(answer.body["rate"], "1.25");
    assert_eq!(answer.body["source"], "lastKnown");
    assert_eq!(answer.body["error"]["kind"], "malformed");
}

/// GIVEN a Yahoo-priced EUR/USD market,
/// WHEN the price of `kraken:EUR` is requested,
/// THEN it is `unavailable` and never the fiat market's price.
#[sqlx::test(migrations = "../../migrations")]
async fn kraken_references_do_not_resolve_to_fiat_markets(pool: PgPool) {
    let repo = MarketDataRepository::new(pool.clone());
    store_priced(
        &repo,
        &fiat("eur"),
        &fiat("usd"),
        1.25,
        PriceProvider::Yahoo,
    )
    .await;
    let addr = serve(&pool, ProviderUrls::default()).await;

    let answer = get(addr, "/markets/v2/prices?asset=kraken:EUR").await;

    assert_eq!(answer.body["prices"][0]["status"], "unavailable");
    assert_eq!(answer.body["prices"][0]["price"], Value::Null);
}

/// GIVEN a request without a bearer token and one with a bad token,
/// WHEN each of the three v2 market-data routes is called,
/// THEN every call is answered 401.
#[sqlx::test(migrations = "../../migrations")]
async fn market_routes_require_authentication(pool: PgPool) {
    let addr = serve(&pool, ProviderUrls::default()).await;
    let client = reqwest::Client::new();

    for path in [
        "/markets/v2/assets/yf/search?query=x",
        "/markets/v2/prices?asset=yf:AAPL",
        "/markets/v2/fx?base=EUR&quote=USD",
    ] {
        let anonymous = client
            .get(format!("http://{addr}{path}"))
            .send()
            .await
            .unwrap();
        let forged = client
            .get(format!("http://{addr}{path}"))
            .bearer_auth("not-a-jwt")
            .send()
            .await
            .unwrap();

        assert_eq!(anonymous.status().as_u16(), 401, "{path}");
        assert_eq!(forged.status().as_u16(), 401, "{path}");
    }
}
