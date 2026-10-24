-- Development/test rollback only. Production recovery remains forward-only.
ALTER TABLE assets_data
    DROP CONSTRAINT IF EXISTS assets_data_metadata_source_check,
    DROP COLUMN IF EXISTS metadata_source;
