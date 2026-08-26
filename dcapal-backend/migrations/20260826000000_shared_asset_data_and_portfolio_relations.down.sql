-- Development/test recovery only. Production recovery is forward-only and
-- must use the pre-deployment backup instead of a down migration.
ALTER TABLE portfolio_asset
    DROP CONSTRAINT IF EXISTS portfolio_asset_portfolio_assets_data_key,
    DROP CONSTRAINT IF EXISTS portfolio_asset_assets_data_fk;

ALTER TABLE portfolio_asset
    ADD COLUMN symbol TEXT,
    ADD COLUMN name TEXT,
    ADD COLUMN asset_class SMALLINT,
    ADD COLUMN currency TEXT,
    ADD COLUMN provider SMALLINT;

UPDATE portfolio_asset AS asset
SET symbol = shared.symbol,
    name = shared.name,
    asset_class = COALESCE(asset.asset_class_override, shared.asset_class),
    currency = shared.currency,
    provider = shared.provider
FROM assets_data AS shared
WHERE shared.id = asset.assets_data_id;

ALTER TABLE portfolio_asset
    DROP COLUMN assets_data_id,
    DROP COLUMN asset_class_override,
    ALTER COLUMN id SET DEFAULT gen_random_uuid();

DROP TABLE assets_data;
