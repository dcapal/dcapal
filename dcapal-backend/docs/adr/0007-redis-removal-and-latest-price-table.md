# ADR 0007: Remove Redis; store market data in PostgreSQL

- Status: accepted
- Date: 2026-11-15

## Context

Redis held the market-asset catalog, the markets with their last price, and a
few runtime counters (discovery marker, visitor counts, imported-portfolio
count and the 60 second portfolio transfers). Running a second stateful
service for a few hundred small records added deployment, ACL and backup
surface, and the issues that follow (#774 price provenance, #779 daily
history) need relational data (joins with `assets_data`, last error next to
the last value) that Redis cannot give.

## Decision

- Redis is removed from the backend, configuration, Compose files, CI and
  deployment. Nothing is exported: the market discovery worker rebuilds the
  catalog and markets, and the price worker refills prices within one cycle.
- Market assets are rows of the shared `assets_data` table (ADR 0003), not a
  parallel catalog. Crypto assets discovered from Kraken are stored as provider
  Kraken, class Crypto, provider-confirmed. Fiat currencies (AED, AUD, CAD, CHF,
  EUR, GBP, JPY, USD, with English names) are provider Yahoo, class Cash,
  provider-confirmed. The v1 REST ids remain the lower-cased symbol (asset) and
  the concatenation of base and quote ids (market); the catalog lists contain
  only provider-confirmed rows, and for fiat only the eight catalog symbols, so
  client-supplied rows and other Yahoo currency tickers never appear in
  `/assets/crypto` or `/assets/fiat`.
- Discovery never renames a provider-confirmed row (ADR 0005). A client-supplied
  Kraken crypto row keeps its name and is only marked confirmed. Fiat seeding is
  the one writer besides v2 resolution that replaces client-supplied metadata
  (ADR 0003): the catalog is the authority for these eight symbols.
- `market_price` is a **plain table with one row per market**, keyed by
  `(base_asset_id, quote_asset_id)` (both foreign keys to `assets_data`), with
  price, the numeric provider code that supplied it (`assets_data.provider`
  codes), `observed_at`, `fetched_at`, `last_error` and `last_error_at`; the row
  itself is the tradable market, so there is no `market` table. Price and
  timestamps are `NULL` until the first price is stored. Rows are replaced in
  place and the table is deliberately not a hypertable: the TimescaleDB
  hypertable is reserved for the durable daily history of #779. The error
  columns are created now and written by #774. Market id lookups compare
  `LOWER(symbol)` concatenations; the table has a few thousand rows at most.
- FX (fiat against fiat) is priced by Yahoo, not Kraken. Seven tickers are
  hardcoded next to the Yahoo adapter (`EURUSD=X`, `GBPUSD=X`, `AUDUSD=X`,
  `CADUSD=X`, `CHFUSD=X`, `JPYUSD=X`, `AEDUSD=X`, each USD per unit); inverse and
  cross rates are derived by the conversion service, and no Yahoo search happens
  at startup. Kraken discovery drops fiat/fiat pairs. The price worker picks the
  provider per market (fiat/fiat: Yahoo, otherwise the configured crypto
  provider) and stores the provider actually used. CryptoWatch has no provider
  code, so its prices cannot be stored.
- The discovery worker also seeds the fiat assets and the FX markets at start
  and before every discovery, so FX never depends on Kraken. Hourly reconciliation
  deletes only markets outside the Kraken snapshot that are not fiat/fiat.
- **FX startup gate.** The server does not go online without FX rates:
  `DcaServer::start` runs `ensure_fx_rates` before it binds the listener (not in
  `try_new`, so tests can build the server without it). The step idempotently
  upserts the eight fiat assets and the seven FX markets (the seeding is shared
  with the discovery worker, which keeps calling it), returns immediately when
  all seven FX markets already have a price, even a stale one, and otherwise
  fetches the missing prices from Yahoo with the hardcoded tickers. Each ticker
  is retried with exponential backoff (1 s doubling up to 30 s) inside a total
  budget of two minutes; when the budget is spent startup fails with
  `DcaError::StartupFailure`, the process exits non-zero and the orchestrator
  restarts it. Yahoo reachability is thus required on the first boot against
  an empty database only. The price fetcher is injected (`FxPriceFetcher`) so
  the gate is tested with a fake provider and a paused clock. The price worker
  keeps refreshing FX prices every five minutes.
- The migration moves only the eight catalog fiat symbols (AED, AUD, CAD, CHF,
  EUR, GBP, JPY, USD) of existing Kraken cash rows of `assets_data` to Yahoo:
  renamed in place when no Yahoo row of the symbol exists; otherwise the
  Portfolio relationships are repointed to the Yahoo row (keeping the effective
  Asset Class as an override), except for Portfolios that already hold both
  rows, which stay on the Kraken row. Only unreferenced Kraken fiat cash rows
  are deleted and holdings are never merged. Kraken cash rows of any other
  symbol (for example a stablecoin a user holds as cash) are left untouched;
  the migration reports their count with `RAISE NOTICE`, along with the other
  counts, for the production dry run. The v1 synchronization maps incoming
  Kraken (or legacy `DCAPal`) cash rows of catalog fiat symbols to Yahoo and
  returns them to clients with provider `YF`; non-fiat Kraken cash keeps
  provider `DCAPal`.
- The 5-minute price worker flushes prices in batches of 50 markets, one
  `UPDATE ... FROM UNNEST` per batch. Prices of markets deleted by reconciliation
  in the meantime are skipped.
- Prices are stored as `NUMERIC` through their shortest round-trip text, so an
  `f64` is read back bit-identical.
- `MarketDataRepository` keeps its public methods and now uses a `PgPool`.
  `MarketDataService` caches markets with `moka`: FX markets (fiat against
  fiat) are never evicted (no TTL, no capacity bound, replaced in place by the
  price worker); all other markets sit in a cache bounded to 10,000 entries
  with LRU eviction. Neither cache has a TTL.
- The runtime state moves to `worker_marker`, `app_counter`, `visitor` and
  `imported_portfolio`. Imported portfolios keep UUIDv4 identifiers because
  the id is the capability to fetch the portfolio. Readers ignore expired rows
  and each insert purges them.
- The unused per-IP geolocation cache methods were dropped instead of ported.

## Consequences

- Production visitor and import counters restart from zero, and v1 consumers
  see missing markets and prices until the first discovery and price passes
  finish.
- Every non-loopback request now performs one PostgreSQL upsert for the visitor
  counter, where it performed one Redis `HINCRBY`.
- Deployment secrets `REDIS_*` and the `config/redis` files are no longer used.
