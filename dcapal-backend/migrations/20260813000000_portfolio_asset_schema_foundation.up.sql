ALTER TABLE portfolio_asset
    RENAME COLUMN provider TO legacy_provider;

ALTER TABLE portfolio_asset
    RENAME COLUMN asset_class TO legacy_asset_class;

ALTER TABLE portfolio_asset
    ADD COLUMN provider SMALLINT,
    ADD COLUMN asset_class SMALLINT,
    ADD COLUMN manual_price NUMERIC;
