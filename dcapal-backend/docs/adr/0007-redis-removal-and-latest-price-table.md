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
- Tables `market_asset`, `market` and `market_price` replace the Redis hashes.
  They use the lower-case provider-derived ids the v1 API already exposes as
  natural text keys, so the ids are not UUIDv7 surrogates.
- `market_price` is a **plain table with one row per market** (price,
  provider, `observed_at`, `fetched_at`, `last_error`, `last_error_at`),
  replaced in place. It is deliberately not a hypertable: the TimescaleDB
  hypertable is reserved for the durable daily history of #779. The error
  columns are created now and written by #774.
- The 5-minute price worker flushes prices in batches of 50 markets, one
  `INSERT ... SELECT FROM UNNEST ... ON CONFLICT` per batch. Prices of markets
  deleted by reconciliation in the meantime are skipped.
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
