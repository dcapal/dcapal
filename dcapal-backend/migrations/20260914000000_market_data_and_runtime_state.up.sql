-- Moves market data and the small runtime state that lived in Redis into
-- PostgreSQL (see ADR 0007). Nothing is exported from Redis: the market
-- discovery and price workers repopulate the catalog, the markets and the
-- latest prices after deployment.
--
-- Natural text keys are used on purpose: asset and market identifiers are
-- stable provider-derived lowercase strings (`btc`, `btceur`) that the v1 REST
-- API already exposes, so UUIDv7 surrogates would only add a lookup hop.

-- Market-asset catalog: the crypto and fiat assets usable as the base or quote
-- side of a market.
CREATE TABLE market_asset (
    -- Lower-case asset identifier, for example `btc` or `eur`.
    id TEXT NOT NULL PRIMARY KEY,
    -- Asset kind code: 0 = crypto, 1 = fiat.
    kind SMALLINT NOT NULL,
    -- Display symbol for crypto (`BTC`) and English name for fiat (`Euro`).
    symbol TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Serves the per-kind catalog reads behind /assets/crypto and /assets/fiat.
CREATE INDEX market_asset_kind_idx ON market_asset (kind, id);

-- Markets: tradable base/quote pairs of market assets.
CREATE TABLE market (
    -- Lower-case market identifier, the concatenation of base and quote ids.
    id TEXT NOT NULL PRIMARY KEY,
    -- Display pair, for example `BTC/EUR`.
    pair TEXT NOT NULL,
    base_asset_id TEXT NOT NULL REFERENCES market_asset (id),
    quote_asset_id TEXT NOT NULL REFERENCES market_asset (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Latest price of each market: a plain table with one row per market, replaced
-- in place by the price workers. It is deliberately not a hypertable; the
-- TimescaleDB hypertable is reserved for the durable daily history.
CREATE TABLE market_price (
    market_id TEXT NOT NULL PRIMARY KEY REFERENCES market (id) ON DELETE CASCADE,
    -- Value of one unit of the base asset in the quote asset.
    price NUMERIC NOT NULL,
    -- Price provider that supplied the value.
    provider TEXT NOT NULL,
    -- When the provider observed the value.
    observed_at TIMESTAMPTZ NOT NULL,
    -- When the backend stored the value.
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Most recent failure to refresh the price, kept next to the last known value.
    last_error TEXT NULL,
    last_error_at TIMESTAMPTZ NULL
);

-- Last completion time of periodic workers, keyed by worker name.
CREATE TABLE worker_marker (
    name TEXT NOT NULL PRIMARY KEY,
    completed_at TIMESTAMPTZ NOT NULL
);

-- Named monotonic counters, for example the number of imported portfolios.
CREATE TABLE app_counter (
    name TEXT NOT NULL PRIMARY KEY,
    value BIGINT NOT NULL DEFAULT 0
);

-- Number of requests seen per visitor IP address.
CREATE TABLE visitor (
    ip TEXT NOT NULL PRIMARY KEY,
    visits BIGINT NOT NULL DEFAULT 0
);

-- Short-lived portfolio transfers created by the v1 import endpoint. Readers
-- ignore rows past `expires_at`; writers purge them.
CREATE TABLE imported_portfolio (
    -- Random UUIDv4: the identifier is the capability to fetch the portfolio,
    -- so it must not leak creation time like a UUIDv7 would.
    id UUID NOT NULL PRIMARY KEY,
    payload JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX imported_portfolio_expires_at_idx ON imported_portfolio (expires_at);
