-- Rewrite existing Portfolio Asset identifiers after the shared-asset
-- migration has removed duplicate relationships.
CREATE TEMP TABLE portfolio_asset_id_map (
    old_id UUID PRIMARY KEY,
    new_id UUID NOT NULL UNIQUE
) ON COMMIT DROP;

INSERT INTO portfolio_asset_id_map (old_id, new_id)
SELECT id, uuidv7()
FROM portfolio_asset;

CREATE TEMP TABLE portfolio_asset_fk_definitions (
    child_schema NAME NOT NULL,
    child_table NAME NOT NULL,
    constraint_name NAME NOT NULL,
    constraint_definition TEXT NOT NULL
) ON COMMIT DROP;

-- Rewire every single-column foreign key that points at portfolio_asset.id
-- before changing the referenced IDs. Dropping and recreating each constraint
-- keeps referential integrity enforced throughout the migration transaction.
DO $$
DECLARE
    foreign_key RECORD;
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_constraint AS constraint_record
        JOIN pg_attribute AS referenced_column
          ON referenced_column.attrelid = constraint_record.confrelid
         AND referenced_column.attnum = ANY(constraint_record.confkey)
        WHERE constraint_record.contype = 'f'
          AND constraint_record.confrelid = 'portfolio_asset'::regclass
          AND referenced_column.attname = 'id'
          AND (
              cardinality(constraint_record.conkey) <> 1
              OR cardinality(constraint_record.confkey) <> 1
          )
    ) THEN
        RAISE EXCEPTION 'portfolio_asset ID migration only supports single-column foreign keys';
    END IF;

    FOR foreign_key IN
        SELECT
            child_namespace.nspname AS child_schema,
            child_table.relname AS child_table,
            constraint_record.conname AS constraint_name,
            child_column.attname AS child_column,
            pg_get_constraintdef(constraint_record.oid, TRUE)
                || CASE
                    WHEN constraint_record.convalidated THEN ''
                    ELSE ' NOT VALID'
                END AS constraint_definition
        FROM pg_constraint AS constraint_record
        JOIN pg_class AS child_table
            ON child_table.oid = constraint_record.conrelid
        JOIN pg_namespace AS child_namespace
            ON child_namespace.oid = child_table.relnamespace
        JOIN pg_attribute AS child_column
            ON child_column.attrelid = constraint_record.conrelid
           AND child_column.attnum = constraint_record.conkey[1]
        JOIN pg_attribute AS referenced_column
            ON referenced_column.attrelid = constraint_record.confrelid
           AND referenced_column.attnum = constraint_record.confkey[1]
        WHERE constraint_record.contype = 'f'
          AND constraint_record.confrelid = 'portfolio_asset'::regclass
          AND referenced_column.attname = 'id'
          AND cardinality(constraint_record.conkey) = 1
          AND cardinality(constraint_record.confkey) = 1
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I DROP CONSTRAINT %I',
            foreign_key.child_schema,
            foreign_key.child_table,
            foreign_key.constraint_name
        );

        EXECUTE format(
            'UPDATE %I.%I AS child
             SET %I = id_map.new_id
             FROM portfolio_asset_id_map AS id_map
             WHERE child.%I = id_map.old_id',
            foreign_key.child_schema,
            foreign_key.child_table,
            foreign_key.child_column,
            foreign_key.child_column
        );

        INSERT INTO portfolio_asset_fk_definitions (
            child_schema,
            child_table,
            constraint_name,
            constraint_definition
        )
        VALUES (
            foreign_key.child_schema,
            foreign_key.child_table,
            foreign_key.constraint_name,
            foreign_key.constraint_definition
        );
    END LOOP;
END
$$;

UPDATE portfolio_asset AS asset
SET id = id_map.new_id
FROM portfolio_asset_id_map AS id_map
WHERE asset.id = id_map.old_id;

DO $$
DECLARE
    foreign_key RECORD;
BEGIN
    FOR foreign_key IN
        SELECT child_schema, child_table, constraint_name, constraint_definition
        FROM portfolio_asset_fk_definitions
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I ADD CONSTRAINT %I %s',
            foreign_key.child_schema,
            foreign_key.child_table,
            foreign_key.constraint_name,
            foreign_key.constraint_definition
        );
    END LOOP;
END
$$;

-- The shared-asset schema's relation key is the canonical equivalent of
-- (portfolio_id, provider, symbol) after provider and symbol moved to
-- assets_data. Keep the operation safe if the parent migration already added it.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'portfolio_asset'::regclass
          AND conname = 'portfolio_asset_portfolio_assets_data_key'
    ) THEN
        ALTER TABLE portfolio_asset
            ADD CONSTRAINT portfolio_asset_portfolio_assets_data_key
            UNIQUE (portfolio_id, assets_data_id);
    END IF;
END
$$;

-- New identities are supplied by the application. Existing rows were rewritten
-- above with PostgreSQL's native uuidv7() function.
ALTER TABLE portfolio_asset
    ALTER COLUMN id DROP DEFAULT;

ALTER TABLE assets_data
    ALTER COLUMN id DROP DEFAULT;
