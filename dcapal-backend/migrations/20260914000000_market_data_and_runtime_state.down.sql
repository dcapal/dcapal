-- Development/test rollback only. Production recovery remains forward-only.
-- The Kraken to Yahoo move of fiat assets in the up migration is not reverted.
DROP TABLE IF EXISTS imported_portfolio;
DROP TABLE IF EXISTS visitor;
DROP TABLE IF EXISTS app_counter;
DROP TABLE IF EXISTS worker_marker;
DROP TABLE IF EXISTS market_price;
