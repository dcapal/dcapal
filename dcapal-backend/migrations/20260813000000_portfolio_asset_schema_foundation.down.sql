ALTER TABLE portfolio_asset
    DROP COLUMN IF EXISTS provider,
    DROP COLUMN IF EXISTS asset_class,
    DROP COLUMN IF EXISTS manual_price;

ALTER TABLE portfolio_asset
    RENAME COLUMN legacy_provider TO provider;

ALTER TABLE portfolio_asset
    RENAME COLUMN legacy_asset_class TO asset_class;
