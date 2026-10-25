-- Keep allocation semantics separate from the Portfolio Asset relationship.
-- Simple mode stores absolute targets; Strategic mode stores per-class-relative
-- targets.  The mode lives on the Portfolio so rows cannot be mixed.
ALTER TABLE portfolios
    ADD COLUMN allocation_mode SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN drift_band NUMERIC,
    ADD COLUMN strategic_allocation_present BOOLEAN NOT NULL DEFAULT FALSE;

-- Keep every v2 decimal source value exact; the legacy fee columns had a
-- fixed scale that would round otherwise-valid v2 inputs.
ALTER TABLE portfolios
    ALTER COLUMN max_fee_impact TYPE NUMERIC,
    ALTER COLUMN fee_amount TYPE NUMERIC,
    ALTER COLUMN fee_rate TYPE NUMERIC,
    ALTER COLUMN min_fee TYPE NUMERIC,
    ALTER COLUMN max_fee TYPE NUMERIC;

-- A hard delete is only a safety-net operation; when it is used, all
-- Portfolio-owned relationship and allocation rows must disappear together.
ALTER TABLE portfolio_asset
    DROP CONSTRAINT IF EXISTS fk_portfolio_asset_portfolio_id,
    ADD CONSTRAINT fk_portfolio_asset_portfolio_id
        FOREIGN KEY (portfolio_id) REFERENCES portfolios (id) ON DELETE CASCADE;

-- A composite foreign key keeps an allocation tied to the same Portfolio as
-- its Portfolio Asset relationship.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'portfolio_asset'::regclass
          AND conname = 'portfolio_asset_portfolio_id_id_key'
    ) THEN
        ALTER TABLE portfolio_asset
            ADD CONSTRAINT portfolio_asset_portfolio_id_id_key
            UNIQUE (portfolio_id, id);
    END IF;
END
$$;

CREATE TABLE portfolio_asset_allocation (
    portfolio_id UUID NOT NULL,
    portfolio_asset_id UUID NOT NULL,
    weight NUMERIC NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (portfolio_id, portfolio_asset_id),
    CONSTRAINT portfolio_asset_allocation_portfolio_asset_fk
        FOREIGN KEY (portfolio_id, portfolio_asset_id)
        REFERENCES portfolio_asset (portfolio_id, id)
        ON DELETE CASCADE
);

CREATE TABLE strategic_asset_class_targets (
    portfolio_id UUID NOT NULL,
    asset_class SMALLINT NOT NULL,
    target_weight NUMERIC NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (portfolio_id, asset_class),
    CONSTRAINT strategic_asset_class_targets_portfolio_fk
        FOREIGN KEY (portfolio_id) REFERENCES portfolios (id)
        ON DELETE CASCADE
);

-- Existing v1 targets are absolute Portfolio targets. Copy valid values before
-- the legacy column is removed. Range validation lives in the application, and
-- legacy data was never range-checked, so an out-of-range value is malformed:
-- it is dropped (the asset is left without an allocation) rather than clamped
-- or allowed to fail the migration. Reads treat a missing allocation as zero,
-- and the next synchronization must provide a valid target.
DO $$
DECLARE
    skipped_target_count BIGINT;
BEGIN
    SELECT COUNT(*)
    INTO skipped_target_count
    FROM portfolio_asset
    WHERE target_weight < 0 OR target_weight > 100;

    IF skipped_target_count > 0 THEN
        RAISE NOTICE
            'portfolio_asset allocation targets skipped because they are outside 0..100: %',
            skipped_target_count;
    END IF;
END
$$;

INSERT INTO portfolio_asset_allocation (portfolio_id, portfolio_asset_id, weight)
SELECT portfolio_id, id, target_weight
FROM portfolio_asset
WHERE target_weight BETWEEN 0 AND 100
ON CONFLICT (portfolio_id, portfolio_asset_id) DO NOTHING;

ALTER TABLE portfolio_asset
    DROP COLUMN target_weight;
