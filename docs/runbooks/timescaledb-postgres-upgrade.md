# PostgreSQL and TimescaleDB upgrade runbook

This runbook covers the DcaPal application database. It does not cover the
Supabase authentication database, which is a separate service and must not
receive DcaPal migrations.

## Current target

The application database uses:

    timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss

The Compose service is named `db`. Local development exposes it on host port
5433; the backend container connects to it on port 5432 inside the Compose
network.

## Before an upgrade

Confirm that the source database is healthy and that the application has a
recent backup. Use PostgreSQL's custom dump format so the restore can be
performed by `pg_restore` and objects can be recreated in dependency order:

    pg_dump --format=custom --no-owner \
      --file=dcapal-pre-pg18.dump "$SOURCE_DATABASE_URL"

Keep the source PostgreSQL 17 database available until the restore and
application checks are complete. Do not use SQLx down migrations as a rollback;
they remove schema objects and can remove data.

## Restore into PostgreSQL 18

Start a clean PostgreSQL 18 TimescaleDB target, then restore the custom dump:

    pg_restore --no-owner --exit-on-error \
      --dbname="$TARGET_DATABASE_URL" dcapal-pre-pg18.dump

Run the current SQLx migrations. The migration runner should complete without
adding unexpected pending migrations:

    DATABASE_URL="$TARGET_DATABASE_URL" cargo run -p migration

Verify the target before switching the application:

    psql "$TARGET_DATABASE_URL" -c \
      "SHOW server_version; SELECT extname, extversion FROM pg_extension WHERE extname = 'timescaledb';"

Also verify representative user, saved portfolio, and portfolio asset counts,
then start the backend and confirm its normal health check succeeds.

The repository includes a disposable proof of this procedure:

    ./dcapal-backend/scripts/verify-postgres-upgrade.sh

The proof starts TimescaleDB PostgreSQL 17 and 18 containers, runs the SQLx
migrations, inserts representative rows, creates a custom-format dump, restores
it into PostgreSQL 18, reruns migrations, and checks the extension and data.

## Cutover and rollback

After the target passes verification, point the Compose-managed deployment at
the PostgreSQL 18 database and restart the backend. Keep the source backup and
database until the deployment has passed its normal health and application
checks.

If the target fails before cutover, leave the source untouched, fix the target,
and repeat the restore. If the failure happens after cutover, stop the backend,
point it back to the retained PostgreSQL 17 database or restore, and restart
it. Investigate the failed target before retrying; do not run destructive down
migrations against production.
