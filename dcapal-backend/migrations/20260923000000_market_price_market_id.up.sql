-- Stores the v1 market id (lower-case base symbol followed by lower-case quote symbol)
-- on each market row, so lookups use an index instead of computing the id per row, and
-- adds indexes on the foreign keys that had none.

ALTER TABLE market_price ADD COLUMN market_id TEXT;

UPDATE market_price AS p
SET market_id = LOWER(b.symbol) || LOWER(q.symbol)
FROM assets_data AS b, assets_data AS q
WHERE b.id = p.base_asset_id AND q.id = p.quote_asset_id;

-- Concatenated ids are not guaranteed unique across different pairs. Keep the most
-- recently priced market of any collision. The others are deleted for good: the
-- application refuses to store a market whose id belongs to a different pair.
DELETE FROM market_price AS p
USING (
    SELECT base_asset_id, quote_asset_id,
           ROW_NUMBER() OVER (
               PARTITION BY market_id
               ORDER BY fetched_at DESC NULLS LAST, base_asset_id, quote_asset_id
           ) AS rank
    FROM market_price
) AS ranked
WHERE ranked.base_asset_id = p.base_asset_id
  AND ranked.quote_asset_id = p.quote_asset_id
  AND ranked.rank > 1;

ALTER TABLE market_price ALTER COLUMN market_id SET NOT NULL;
ALTER TABLE market_price ADD CONSTRAINT market_price_market_id_key UNIQUE (market_id);

-- The primary key already leads with base_asset_id.
CREATE INDEX market_price_quote_asset_id_idx ON market_price (quote_asset_id);
CREATE INDEX portfolio_asset_assets_data_id_idx ON portfolio_asset (assets_data_id);
