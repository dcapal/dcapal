-- Moves market data and the small runtime state that lived in Redis into
-- PostgreSQL (see ADR 0007). Nothing is exported from Redis: the market
-- discovery and price workers repopulate the catalog, the markets and the
-- latest prices after deployment.
--
-- Market assets are rows of the shared `assets_data` table (ADR 0003): crypto
-- assets listed by Kraken and the fiat currencies priced by Yahoo. A market is
-- a priced pair of two `assets_data` rows, so there is no separate catalog.

-- Latest price of each market: a plain table with one row per base/quote pair,
-- replaced in place by the price workers. It is deliberately not a hypertable;
-- the TimescaleDB hypertable is reserved for the durable daily history.
--
-- Tables updated in place frequently (latest prices, worker markers, counters,
-- visitors) leave 20% free space per page (`fillfactor = 80`) so updates can stay
-- on the same page (HOT) and avoid index and table bloat.
CREATE TABLE market_price (
    base_asset_id UUID NOT NULL REFERENCES assets_data (id),
    quote_asset_id UUID NOT NULL REFERENCES assets_data (id),
    -- Value of one unit of the base asset in the quote asset. NULL until the
    -- first price has been fetched: the row itself is the tradable market.
    price NUMERIC NULL,
    -- Provider that supplies (and supplied) the price, using the numeric
    -- provider codes of `assets_data.provider`.
    provider SMALLINT NOT NULL,
    -- When the provider observed the value.
    observed_at TIMESTAMPTZ NULL,
    -- When the backend stored the value.
    fetched_at TIMESTAMPTZ NULL,
    -- Most recent failure to refresh the price, kept next to the last known value.
    last_error TEXT NULL,
    last_error_at TIMESTAMPTZ NULL,
    PRIMARY KEY (base_asset_id, quote_asset_id)
) WITH (fillfactor = 80);

-- Fiat currencies are provider-neutral cash priced by Yahoo, so the fiat assets
-- that older versions stored under Kraken (provider 1, class Cash 3) move to
-- Yahoo (provider 2). The statements below never merge holding or allocation
-- values: a Portfolio keeps one relationship per asset.
DO $$
DECLARE
    kraken_provider CONSTANT SMALLINT := 1;
    yf_provider CONSTANT SMALLINT := 2;
    cash_class CONSTANT SMALLINT := 3;
    fiat_symbols CONSTANT TEXT[] :=
        ARRAY['AED', 'AUD', 'CAD', 'CHF', 'EUR', 'GBP', 'JPY', 'USD'];
    renamed_count BIGINT;
    repointed_count BIGINT;
    conflict_count BIGINT;
    deleted_count BIGINT;
    outside_catalog_count BIGINT;
BEGIN
    -- Kraken cash rows whose symbol is not a catalog fiat currency are moved
    -- like the others; report them so a production dry run can review them.
    SELECT COUNT(*)
    INTO outside_catalog_count
    FROM assets_data
    WHERE provider = kraken_provider
      AND asset_class = cash_class
      AND NOT (symbol = ANY (fiat_symbols));

    -- Kraken cash rows without a Yahoo row of the same symbol become that row.
    WITH renamed AS (
        UPDATE assets_data AS kraken
        SET provider = yf_provider,
            updated_at = NOW()
        WHERE kraken.provider = kraken_provider
          AND kraken.asset_class = cash_class
          AND NOT EXISTS (
              SELECT 1
              FROM assets_data AS yf
              WHERE yf.provider = yf_provider
                AND yf.symbol = kraken.symbol
          )
        RETURNING 1
    )
    SELECT COUNT(*) INTO renamed_count FROM renamed;

    -- Relationships on the remaining Kraken cash rows (a Yahoo row of the same
    -- symbol exists) are repointed to the Yahoo row, unless the Portfolio
    -- already holds both. The effective Asset Class is preserved: when the
    -- relationship inherited the Kraken class and the Yahoo default differs,
    -- the Kraken class becomes an explicit override.
    WITH twins AS (
        SELECT kraken.id AS kraken_id,
               yf.id AS yf_id,
               yf.asset_class AS yf_class
        FROM assets_data AS kraken
        JOIN assets_data AS yf
          ON yf.provider = yf_provider AND yf.symbol = kraken.symbol
        WHERE kraken.provider = kraken_provider
          AND kraken.asset_class = cash_class
    ),
    repointed AS (
        UPDATE portfolio_asset AS relationship
        SET assets_data_id = twins.yf_id,
            asset_class_override = CASE
                WHEN relationship.asset_class_override IS NULL
                     AND twins.yf_class <> cash_class
                THEN cash_class
                ELSE relationship.asset_class_override
            END,
            updated_at = NOW()
        FROM twins
        WHERE relationship.assets_data_id = twins.kraken_id
          AND NOT EXISTS (
              SELECT 1
              FROM portfolio_asset AS other
              WHERE other.portfolio_id = relationship.portfolio_id
                AND other.assets_data_id = twins.yf_id
          )
        RETURNING 1
    )
    SELECT COUNT(*) INTO repointed_count FROM repointed;

    -- Portfolios holding both rows keep their relationship on the Kraken row.
    SELECT COUNT(*)
    INTO conflict_count
    FROM portfolio_asset AS relationship
    JOIN assets_data AS kraken ON kraken.id = relationship.assets_data_id
    WHERE kraken.provider = kraken_provider
      AND kraken.asset_class = cash_class;

    -- Only Kraken cash rows left without any relationship are removed.
    WITH deleted AS (
        DELETE FROM assets_data AS kraken
        WHERE kraken.provider = kraken_provider
          AND kraken.asset_class = cash_class
          AND NOT EXISTS (
              SELECT 1
              FROM portfolio_asset AS relationship
              WHERE relationship.assets_data_id = kraken.id
          )
        RETURNING 1
    )
    SELECT COUNT(*) INTO deleted_count FROM deleted;

    RAISE NOTICE 'Kraken cash assets renamed to Yahoo: %', renamed_count;
    RAISE NOTICE 'portfolio relationships repointed to an existing Yahoo asset: %',
        repointed_count;
    RAISE NOTICE 'portfolio relationships left on a Kraken cash asset (Portfolio already holds the Yahoo asset): %',
        conflict_count;
    RAISE NOTICE 'unreferenced Kraken cash assets deleted: %', deleted_count;
    RAISE NOTICE 'Kraken cash assets outside the fiat catalog moved to Yahoo: %',
        outside_catalog_count;
END
$$;

-- Last completion time of periodic workers, keyed by worker name.
CREATE TABLE worker_marker (
    name TEXT NOT NULL PRIMARY KEY,
    completed_at TIMESTAMPTZ NOT NULL
) WITH (fillfactor = 80);

-- Named monotonic counters, for example the number of imported portfolios.
CREATE TABLE app_counter (
    name TEXT NOT NULL PRIMARY KEY,
    value BIGINT NOT NULL DEFAULT 0
) WITH (fillfactor = 80);

-- Number of requests seen per visitor IP address.
CREATE TABLE visitor (
    ip TEXT NOT NULL PRIMARY KEY,
    visits BIGINT NOT NULL DEFAULT 0
) WITH (fillfactor = 80);

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
