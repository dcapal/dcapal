-- Development/test rollback only. Production recovery remains forward-only.
ALTER TABLE portfolio_asset
    ADD COLUMN target_weight NUMERIC;

UPDATE portfolio_asset AS asset
SET target_weight = COALESCE(
    (
        SELECT allocation.weight
        FROM portfolio_asset_allocation AS allocation
        WHERE allocation.portfolio_id = asset.portfolio_id
          AND allocation.portfolio_asset_id = asset.id
    ),
    0
);

ALTER TABLE portfolio_asset
    ALTER COLUMN target_weight SET NOT NULL;

DROP TABLE IF EXISTS strategic_asset_class_targets;
DROP TABLE IF EXISTS portfolio_asset_allocation;

ALTER TABLE portfolio_asset
    DROP CONSTRAINT IF EXISTS portfolio_asset_portfolio_id_id_key,
    DROP CONSTRAINT IF EXISTS fk_portfolio_asset_portfolio_id,
    ADD CONSTRAINT fk_portfolio_asset_portfolio_id
        FOREIGN KEY (portfolio_id) REFERENCES portfolios (id);

ALTER TABLE portfolios
    DROP CONSTRAINT IF EXISTS portfolios_allocation_mode_check,
    DROP CONSTRAINT IF EXISTS portfolios_drift_band_check,
    DROP COLUMN IF EXISTS allocation_mode,
    DROP COLUMN IF EXISTS drift_band,
    DROP COLUMN IF EXISTS strategic_allocation_present;

ALTER TABLE portfolios
    ALTER COLUMN max_fee_impact TYPE NUMERIC(20, 10),
    ALTER COLUMN fee_amount TYPE NUMERIC(20, 10),
    ALTER COLUMN fee_rate TYPE NUMERIC(20, 10),
    ALTER COLUMN min_fee TYPE NUMERIC(20, 10),
    ALTER COLUMN max_fee TYPE NUMERIC(20, 10);
