# Finding: Yahoo historical market data and crypto-provider boundary

**Context:** [GitHub issue #762](https://github.com/dcapal/dcapal/issues/762), “Research: Verify Yahoo Finance historical coverage and FX authority”.

**Research date:** 2026-08-05

## Finding

For this MVP, Yahoo Finance is the authoritative provider for non-crypto market data and FX conversion series, and Kraken is the authoritative crypto source. CryptoWatch is not an acceptable fallback or parallel source. A Yahoo failure must remain a Yahoo failure; it must not silently become a Kraken or other-provider result.

This conclusion is partly a repository-contract finding, not a claim that Yahoo publishes a complete public contract for its chart endpoint. Yahoo’s first-party material confirms broad coverage and important restrictions, but does not publish the exact undocumented chart-endpoint limits needed to promise arbitrary intraday history.

## Yahoo coverage and symbols

Yahoo says historical data is available for “most quotes”, but that availability can be limited by data-licensing restrictions; where a requested range exceeds available history, Yahoo displays the available data. Yahoo also says historical prices usually do not go earlier than 1970. Its coverage page identifies exchanges, suffixes, delays, and underlying data providers, and states that the data is informational and must not be redistributed. ([Yahoo Help: download historical data](https://help.yahoo.com/kb/finance/download-historical-data-yahoo-finance-sln2311.html); [Yahoo Help: exchanges and data providers](https://help.yahoo.com/kb/yahoo-finance-plus/exchanges-data-providers-yahoo-finance-sln2310.html))

The repository’s Yahoo adapter uses the chart endpoint `query1.finance.yahoo.com/v8/finance/chart/{symbol}` and the search endpoint `query2.finance.yahoo.com/v1/finance/search`. It maps ordinary assets to `{BASE}-{QUOTE}` and fiat pairs to `{BASE}{QUOTE}=X`, except that USD as the base is represented as `{QUOTE}=X`; it also contains the local `luna -> luna1` symbol exception. ([`yahoo.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L120-L190))

That mapping means the supported FX shape is a Yahoo currency symbol such as `EURUSD=X`, with the repository’s special inversion for USD-base markets. It does not establish that every fiat currency or every cross is covered: Yahoo’s own wording is “most quotes”, and licensing or missing-symbol responses are unsupported cases.

## Intervals, timestamps, and depth

The repository contract exposes only two OHLC frequencies: five minutes and one day. The Yahoo adapter sends them as `interval=5m` and `interval=1d`. For five-minute prices it asks for the 12 preceding five-minute periods ending at the current five-minute boundary; for daily prices it asks from the start of the previous day through the requested timestamp. It sends `period1` and `period2` as Unix seconds. ([`entity.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/app/domain/entity.rs#L122-L169); [`yahoo.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L41-L53))

The adapter currently reads only the last non-null `close` value from the first chart result and does not read Yahoo’s bar timestamps. The stored DcaPal `Price` timestamp is the fetch time (`Utc::now()`), not the source bar timestamp. This is an existing behavior that a future implementation must preserve or deliberately change; it must not be described as source-time fidelity. ([`yahoo.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L91-L117); [`market_data_utils.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/app/domain/market_data_utils.rs#L7-L20))

Yahoo’s first-party help describes selectable date ranges and chart intervals, but it does not publish a precise maximum age for each undocumented chart API interval. Therefore the authoritative finding is:

- `5m` and `1d` are the only intervals this repository promises.
- Daily history may extend back only as far as Yahoo has data for the instrument, subject to the “usually not earlier than 1970” guidance and licensing.
- Five-minute history must be treated as bounded and availability-dependent. The repository must not promise history beyond the range Yahoo actually returns, and it must not fabricate or substitute older bars.
- Exact claims such as “5m is available for exactly N days” are not supported by Yahoo first-party documentation found for this issue and must not become a product guarantee. ([Yahoo Help: change chart period and scale](https://help.yahoo.com/kb/period-scale-screen-charts-yahoo-finance-web-sln28287.html); [Yahoo Help: download historical data](https://help.yahoo.com/kb/finance/download-historical-data-yahoo-finance-sln2311.html))

## Rate limits, terms, and failures

Yahoo’s published API terms say Yahoo may impose rate limits and quotas at its discretion, may suspend or terminate access, may change API specifications or access methods without notice, and provides APIs without a warranty of reliability, accuracy, completeness, or uninterrupted service. The terms also prohibit automated access other than through the APIs, actions that burden Yahoo infrastructure, and redistribution or use outside the permitted terms. No fixed public requests-per-minute limit for the chart endpoint is stated in the cited Yahoo material. ([Yahoo API Terms of Use](https://legal.yahoo.com/us/en/yahoo/terms/product-atos/apiforydn/index.html); [Yahoo API Terms and Conditions](https://legal.yahoo.com/us/en/yahoo/terms/product-atos/apitnc/index.html))

The repository therefore needs bounded request volume, caching, and explicit provider errors, but cannot claim a numeric Yahoo quota. It already spaces price-update requests by 100 ms, but that local delay is an application safeguard, not a Yahoo contractual rate limit. ([`price_updater.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/app/workers/price_updater.rs#L66-L82))

Current Yahoo failure behavior is materially different by path: a transport failure in the internal price path propagates as an error; an HTTP 404 or a chart payload error becomes `None`; other non-success statuses propagate as errors. The proxy path returns the upstream status and body when a request succeeds, and returns `502 Bad Gateway` when the request cannot be sent. Empty or malformed successful chart payloads are errors. ([`yahoo.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L64-L89); [`yahoo.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L91-L149))

This means missing coverage, an unsupported FX symbol, a licensing restriction, a rate-limit response, and a provider outage must remain observable as missing data or a provider error according to the path. They must not trigger a silent provider substitution.

## Crypto source and complete CryptoWatch removal

Kraken is the supported crypto source in the repository. Market discovery obtains online pairs from Kraken `AssetPairs`; price retrieval uses Kraken’s public `OHLC` endpoint with the repository’s five-minute and daily mappings. Kraken documents that OHLC supports minute intervals including 5 and 1440, returns the current uncommitted candle, and returns at most 720 recent entries. Its public API rate-limit guidance documents the call counter, tiered limits, decay, and the `EAPI:Rate limit exceeded` / throttling errors. ([Kraken: tradable asset pairs](https://docs.kraken.com/api-reference/market-data/get-tradable-asset-pairs); [Kraken: OHLC data](https://docs.kraken.com/api-reference/market-data/get-ohlc-data); [Kraken: Spot REST rate limits](https://docs.kraken.com/exchange/guides/rest/ratelimits))

At this repository snapshot, CryptoWatch is still wired into configuration, provider construction, dispatch, and the adapter module, while Kraken already owns crypto discovery. Specifically, the code still has a `CryptoWatch` enum variant and API-key field, constructs a `CryptoWatchProvider`, exposes it through `PriceProviders`, dispatches to it, and retains `cw.rs`. ([`config.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/config.rs#L5-L11); [`config.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/config.rs#L34-L42); [`adapter/mod.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/mod.rs#L1-L29); [`market_data_utils.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/app/domain/market_data_utils.rs#L12-L17); [`lib.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/lib.rs#L144-L152); [`cw.rs`](https://github.com/dcapal/dcapal/blob/f652260/dcapal-backend/src/ports/outbound/adapter/cw.rs))

“Complete CryptoWatch removal” therefore means removing those code and configuration paths and any `CW`/`cw_api_key` deployment or documentation references, then making the provider selection explicit: Yahoo for non-crypto assets and FX, Kraken for crypto. A failed Yahoo request must not fall through to Kraken, and a failed Kraken request must not fall through to Yahoo.

## Decision boundary for implementation

1. Keep Yahoo as the only provider for non-crypto assets and FX conversion series.
2. Keep only `5m` and `1d` in the MVP Yahoo contract; treat coverage and intraday depth as provider-returned availability, not guaranteed history.
3. Use Unix-second `period1`/`period2` requests and the repository’s existing symbol rules, including `=X` FX symbols and the `luna1` exception.
4. Preserve source/provider identity in errors and missing-data outcomes; do not substitute providers.
5. Use Kraken as the sole crypto provider and remove CryptoWatch completely, including configuration, construction, dispatch, module, secrets, and docs.
