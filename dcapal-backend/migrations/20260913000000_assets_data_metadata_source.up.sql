-- Records where the name, currency and asset class of a shared asset came from:
-- 0 = supplied by a client (v1 synchronization or rows that predate this
-- column), 1 = confirmed by the asset's provider. The default keeps older
-- backend binaries, whose INSERTs omit the column, working against this schema.
ALTER TABLE assets_data
    ADD COLUMN metadata_source SMALLINT NOT NULL DEFAULT 0;
