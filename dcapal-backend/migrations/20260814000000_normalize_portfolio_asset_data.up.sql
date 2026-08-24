DELETE FROM portfolio_asset
WHERE UPPER(legacy_provider) NOT IN ('DCAPAL', 'KRAKEN', 'YF', 'YAHOO');

UPDATE portfolio_asset
SET provider = CASE UPPER(legacy_provider)
        WHEN 'DCAPAL' THEN 1
        WHEN 'KRAKEN' THEN 1
        WHEN 'YF' THEN 2
        WHEN 'YAHOO' THEN 2
    END,
    asset_class = CASE UPPER(legacy_asset_class)
        WHEN 'EQUITY' THEN 1
        WHEN 'BOND' THEN 2
        WHEN 'CURRENCY' THEN 3
        WHEN 'CASH' THEN 3
        WHEN 'CRYPTO' THEN 4
        WHEN 'COMMODITY' THEN 5
        ELSE 0
    END,
    manual_price = price,
    symbol = UPPER(symbol);

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

ALTER TABLE portfolio_asset
    DROP COLUMN legacy_provider,
    DROP COLUMN legacy_asset_class,
    DROP COLUMN price;
