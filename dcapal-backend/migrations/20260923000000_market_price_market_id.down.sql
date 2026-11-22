DROP INDEX IF EXISTS portfolio_asset_assets_data_id_idx;
DROP INDEX IF EXISTS market_price_quote_asset_id_idx;
ALTER TABLE market_price DROP CONSTRAINT IF EXISTS market_price_market_id_key;
ALTER TABLE market_price DROP COLUMN IF EXISTS market_id;
