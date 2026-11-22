# ADR 0008: v2 market-data contract with provenance and visible failures

- Status: accepted
- Date: 2026-11-22

## Context

The v1 market-data endpoints answer with a bare price or a 404, search is a raw
Yahoo passthrough, and provider failures, stale values and synthetic FX rates are
indistinguishable from "no data". The Portfolio overview and holding flows (#774)
need stable asset identity, the provider and time of every value, and failures that
stay visible. ADR 0007 reserved `market_price.last_error` for this work.

## Decision

- Three JWT-protected endpoints live under `/markets/v2/` next to the asset lookup;
  v1 endpoints are unchanged. Assets are always `{provider: yf|kraken, symbol}`
  references (ADR 0005); wire casing is camelCase, prices and rates are decimal
  strings (ADR 003) and times are RFC 3339.
  - `GET /assets/{provider}/search?query=`: a typed search of one provider, never
    merged across providers. `yf` is a rate-limited (429 when saturated) proxy of
    Yahoo search returning symbol, name, venue and Asset Class derived from
    `quoteType`; Yahoo does not report a currency, so `currency` is null and
    `priceAvailability` is `unknown`. `kraken` searches the Postgres catalog (Kraken
    has no search API); its results are USD-quoted and `priceAvailability` tells
    whether a USD price is stored. The answer carries an `outcome` of `usable`,
    `empty`, `providerError` or `malformed`; provider failures are HTTP 200 with the
    failure in `error`.
  - `GET /prices?asset=<provider>:<symbol>&...` (1 to 20): current prices in the
    asset's own currency, never converted. Kraken crypto is read from its stored
    `{symbol}usd` market (currency `USD`); Yahoo symbols are quoted live from the
    chart metadata in the currency Yahoo lists them in (`GBp` stays pence). Each entry
    has provider, `observedAt`, `fetchedAt`, `source`, `status` and `error`.
  - `GET /fx?base=&quote=`: an FX rate between catalog fiat currencies, with the
    provider, the oldest observation among its legs, how it was derived
    (`synthetic`: `none`, `inverse`, `triangulated`) and its legs. Clients cache each
    pair for the `Cache-Control` max-age and convert values themselves
    (price times rate); there is no per-holding FX call.
- Statuses are `usable`, `unavailable`, `providerError`, `malformed` and `pending`
  (a stored market that was never priced). A missing, zero, negative or non-finite
  price is never a value: it is `unavailable`. The guard lives in application code
  (`validate_price`) and the repository also refuses to store such prices.
- A failed refresh of a stored market is recorded in `last_error` and
  `last_error_at` as `<kind>: <message>` without touching the last known price. The
  next successful price clears it. A price next to a failure is reported `usable`
  with source `lastKnown` and the failure, so stale data is visible rather than
  silent; a fresh price has source `provider`. A manual price is Portfolio data
  (ADR 0003) and is not a market-data source.
- Adapters report typed failures (unavailable, provider error, malformed) and
  never answer with another provider. A Yahoo 200 reply carrying a chart error is a
  provider error, so an error on the 5-minute request no longer falls back to daily
  candles; only "no price" does. Request URLs are percent-encoded. Yahoo and Kraken
  root URLs can be overridden (`yahooBaseUrl`, `krakenBaseUrl` in the provider
  config) so tests run against local HTTP fakes.
- Yahoo equities and funds are not stored in `market_price` (it holds only
  catalog markets); their prices are fetched on request, capped at 20 per call,
  with a ten second timeout, waiting for the shared Yahoo limiter.
- `MarketDataService::get_market` reports storage failures as errors instead of
  "market not found". Consequently v1 `/price` answers 500 rather than 404 when the
  database fails.
- `Cache-Control` of `/prices` and `/fx` is public with a max-age up to the end of the
  five-minute validity window of the stalest value, and `no-cache` when any value is
  missing.

## Consequences

- Current prices are separate from the durable daily history of #779.
- The FX rate is derived from the stored USD-pivot markets on each call, with one
  indexed read, and is independent of the v1 conversion cache.
- Live Yahoo prices cost one rate-limited upstream call per symbol and are not
  persisted.
