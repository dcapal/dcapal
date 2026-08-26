-- PostgreSQL 18 is required because uuidv7() is used for both identities.
DO $$
BEGIN
    IF current_setting('server_version_num')::integer < 180000 THEN
        RAISE EXCEPTION
            'shared asset data migration requires PostgreSQL 18 or newer (server_version_num=%)',
            current_setting('server_version_num');
    END IF;
END
$$;

CREATE TABLE assets_data (
    id UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    provider SMALLINT NOT NULL,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    currency TEXT NOT NULL,
    asset_class SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT assets_data_symbol_uppercase CHECK (symbol = UPPER(symbol)),
    CONSTRAINT assets_data_provider_symbol_key UNIQUE (provider, symbol)
);

ALTER TABLE portfolio_asset
    ADD COLUMN assets_data_id UUID,
    ADD COLUMN asset_class_override SMALLINT;

-- Normalize before ranking so case variants participate in the same identity.
UPDATE portfolio_asset
SET symbol = UPPER(symbol);

DO $$
DECLARE
    duplicate_count BIGINT;
BEGIN
    WITH ranked_assets AS (
        SELECT
            id,
            ROW_NUMBER() OVER (
                PARTITION BY portfolio_id, provider, symbol
                ORDER BY created_at ASC, id ASC
            ) AS duplicate_rank
        FROM portfolio_asset
    )
    DELETE FROM portfolio_asset AS asset
    USING ranked_assets AS ranked
    WHERE asset.id = ranked.id
      AND ranked.duplicate_rank > 1;

    GET DIAGNOSTICS duplicate_count = ROW_COUNT;
    RAISE NOTICE 'portfolio_asset duplicate relationships deleted: %', duplicate_count;
END
$$;

-- The oldest surviving row is the canonical source for shared metadata. A
-- conflict is reported, but later synchronization must not overwrite it.
CREATE TEMP TABLE portfolio_asset_metadata_seeds ON COMMIT DROP AS
SELECT DISTINCT ON (provider, symbol)
    provider,
    symbol,
    name,
    currency,
    asset_class
FROM portfolio_asset
ORDER BY provider, symbol, created_at ASC, id ASC;

DO $$
DECLARE
    metadata_conflict_count BIGINT;
    class_conflict_count BIGINT;
BEGIN
    SELECT COUNT(*)
    INTO metadata_conflict_count
    FROM portfolio_asset AS asset
    JOIN portfolio_asset_metadata_seeds AS seed
      ON seed.provider = asset.provider
     AND seed.symbol = asset.symbol
    WHERE asset.name IS DISTINCT FROM seed.name
       OR asset.currency IS DISTINCT FROM seed.currency;

    SELECT COUNT(*)
    INTO class_conflict_count
    FROM portfolio_asset AS asset
    JOIN portfolio_asset_metadata_seeds AS seed
      ON seed.provider = asset.provider
     AND seed.symbol = asset.symbol
    WHERE asset.asset_class IS DISTINCT FROM seed.asset_class;

    RAISE NOTICE 'assets_data metadata conflicts detected: %', metadata_conflict_count;
    RAISE NOTICE 'portfolio_asset class overrides detected: %', class_conflict_count;
END
$$;

INSERT INTO assets_data (provider, symbol, name, currency, asset_class)
SELECT provider, symbol, name, currency, asset_class
FROM portfolio_asset_metadata_seeds;

UPDATE portfolio_asset AS asset
SET assets_data_id = shared.id,
    asset_class_override = CASE
        WHEN asset.asset_class IS NOT DISTINCT FROM shared.asset_class THEN NULL
        ELSE asset.asset_class
    END
FROM assets_data AS shared
WHERE shared.provider = asset.provider
  AND shared.symbol = asset.symbol;

-- Existing Portfolio Asset IDs predate the canonical UUIDv7 policy. There are
-- currently no dependent tables, so this map also documents the rewrite point
-- for future dependent-reference updates.
CREATE TEMP TABLE portfolio_asset_id_map ON COMMIT DROP AS
SELECT id AS old_id, uuidv7() AS new_id
FROM portfolio_asset;

UPDATE portfolio_asset AS asset
SET id = id_map.new_id
FROM portfolio_asset_id_map AS id_map
WHERE asset.id = id_map.old_id;

ALTER TABLE portfolio_asset
    ALTER COLUMN id SET DEFAULT uuidv7(),
    ALTER COLUMN assets_data_id SET NOT NULL;

ALTER TABLE portfolio_asset
    ADD CONSTRAINT portfolio_asset_assets_data_fk
        FOREIGN KEY (assets_data_id) REFERENCES assets_data (id),
    ADD CONSTRAINT portfolio_asset_portfolio_assets_data_key
        UNIQUE (portfolio_id, assets_data_id);

ALTER TABLE portfolio_asset
    DROP COLUMN symbol,
    DROP COLUMN name,
    DROP COLUMN asset_class,
    DROP COLUMN currency,
    DROP COLUMN provider;
