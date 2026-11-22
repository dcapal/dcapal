-- The market discovery freshness marker was named after the removed CryptoWatch
-- provider. Rename it so discovery does not repeat right after the deployment.
UPDATE worker_marker SET name = 'kraken-discovery-completed' WHERE name = 'cw-last-fetched';
