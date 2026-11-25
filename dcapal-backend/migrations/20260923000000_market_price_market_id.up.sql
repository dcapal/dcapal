-- Stores the market id (lower-case base symbol, '/', lower-case quote symbol, such as
-- 'btc/eur') on each market row, so lookups use an index instead of computing the id per row, and
-- adds indexes on the foreign keys that had none.

ALTER TABLE market_price ADD COLUMN market_id TEXT;

UPDATE market_price AS p
SET market_id = LOWER(b.symbol) || '/' || LOWER(q.symbol)
FROM assets_data AS b, assets_data AS q
WHERE b.id = p.base_asset_id AND q.id = p.quote_asset_id;

ALTER TABLE market_price ALTER COLUMN market_id SET NOT NULL;
ALTER TABLE market_price ADD CONSTRAINT market_price_market_id_key UNIQUE (market_id);

-- The primary key already leads with base_asset_id.
CREATE INDEX market_price_quote_asset_id_idx ON market_price (quote_asset_id);
CREATE INDEX portfolio_asset_assets_data_id_idx ON portfolio_asset (assets_data_id);
