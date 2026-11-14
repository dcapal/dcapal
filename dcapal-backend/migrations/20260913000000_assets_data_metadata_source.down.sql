-- Development/test rollback only. Production recovery remains forward-only.
ALTER TABLE assets_data
    DROP COLUMN IF EXISTS metadata_source;
