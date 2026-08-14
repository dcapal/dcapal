# PostgreSQL 17 to 18 TimescaleDB upgrade runbook

This runbook covers the DcaPal application database on a host where DcaPal is
deployed with Docker Compose. It does not cover the Supabase authentication
database. Supabase is a separate service and must not receive DcaPal
migrations.

The procedure is a one-time physical cutover from a PostgreSQL 17 data
directory to a new PostgreSQL 18 data directory. PostgreSQL major-version
data directories are not reusable across major versions. Use the custom-format
dump and restore below; do not point the PostgreSQL 18 container at the old
directory.

## Deployment layout

The deployment workflow writes this unpinned image tag to the db service:

    timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss

The Compose service is named db. The container is normally named db in
production and db-dev in development. The database data directory is the
host path ./data/db/, mounted into the container at
/home/postgres/pgdata/data.

The backend connects to db:5432 on the Compose network. A deployment may
publish PostgreSQL on a host port such as 45827; that port is only for
host-side access and must not be used in the backend connection string. Local
development currently uses host port 5433 through
docker/docker-compose.dev.yml.

## Safety rules

- Schedule a maintenance window and stop the backend before taking the dump.
- Keep the PostgreSQL 17 data directory and the dump until the PostgreSQL 18
  deployment has passed its application checks.
- Never run docker compose down -v during this procedure.
- Never run PostgreSQL down migrations as a rollback. They remove schema
  objects and may remove data.
- Do not put database passwords directly into commands or this document. Use
  the existing deployed Compose configuration, dcapal.env, or a protected
  shell environment.
- The globals dump below is a recovery artifact. Do not replay it blindly over
  the fresh database: the initialization scripts in config/db/init own the
  deployment roles and their password configuration.

## One-time cutover on the deployed host

### 1. Connect and inspect the deployment

SSH to the host and change to the directory configured as DCAPAL_DIR or
DCAPAL_DIR_DEV in the deployment secrets. The commands below use the
development-style path names from the example deployment; use the actual
directory on the host.

    ssh <deployment-user>@<deployment-host>
    cd <DCAPAL_DIR>

Run the following setup in the same shell as the remaining commands. It uses
the deployment override when that file is present, so it works with the base
database/Redis Compose file and with the full stack written by
.github/workflows/deploy.yml:

    set -euo pipefail

    export DCAPAL_DIR="$PWD"
    export PG18_IMAGE='timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss'
    export STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
    export BACKUP_DIR="$DCAPAL_DIR/data/db-backups/$STAMP"
    export OLD_DATA_DIR="$DCAPAL_DIR/data/db-pg17-$STAMP"
    export COMPOSE_BACKUP="$DCAPAL_DIR/docker-compose.yml.pg17-$STAMP"

    compose() {
      if [[ -f docker/docker-compose.prod.yml ]]; then
        docker compose -f docker-compose.yml -f docker/docker-compose.prod.yml "$@"
      else
        docker compose -f docker-compose.yml "$@"
      fi
    }

    compose config >/dev/null
    compose config --services
    compose ps
    test -d "$DCAPAL_DIR/data/db"

The full-stack deployment override should be present before the migration
step because it supplies the backend image and dcapal.env. If the dcapal
service is not listed by compose config --services, stop here and obtain the
current deployment override before continuing.

    if ! compose config --services | grep -Fxq dcapal; then
      echo 'The dcapal service is not present in the deployment Compose files.' >&2
      echo 'Obtain docker/docker-compose.prod.yml before continuing.' >&2
      exit 1
    fi

    test -f dcapal.env

Confirm that the running database is PostgreSQL 17 before changing anything:

    SOURCE_MAJOR="$(compose exec -T db psql -U postgres -d postgres -Atqc \
      'SHOW server_version_num' | tr -d '[:space:]')"
    case "$SOURCE_MAJOR" in
      17*) ;;
      *)
        echo "Expected PostgreSQL 17, found $SOURCE_MAJOR" >&2
        exit 1
        ;;
    esac

    compose exec -T db psql -U postgres -d postgres -c \
      "SHOW server_version; SELECT extname, extversion
       FROM pg_extension WHERE extname = 'timescaledb';"

### 2. Stop writes and create the backups

Stop the backend service but leave the database running long enough to create
the backups:

    mkdir -p "$BACKUP_DIR"
    chmod 700 "$BACKUP_DIR"
    compose stop dcapal

Create a custom-format database dump and a globals backup. The custom dump
contains the DcaPal schema, migration history, users, portfolios, assets, and
other application data. The globals file records role definitions without
including role passwords:

    compose exec -T db pg_dump \
      -U postgres \
      -d postgres \
      --format=custom \
      --no-owner \
      > "$BACKUP_DIR/dcapal-pg17.dump"

    compose exec -T db pg_dumpall \
      -U postgres \
      --globals-only \
      --no-role-passwords \
      > "$BACKUP_DIR/pg-globals.sql"

    chmod 600 "$BACKUP_DIR/dcapal-pg17.dump" "$BACKUP_DIR/pg-globals.sql"
    test -s "$BACKUP_DIR/dcapal-pg17.dump"
    test -s "$BACKUP_DIR/pg-globals.sql"
    compose exec -T db pg_restore --list \
      < "$BACKUP_DIR/dcapal-pg17.dump" \
      >/dev/null

Record representative counts for the post-restore comparison:

    compose exec -T db psql -U postgres -d postgres -Atqc \
      "SELECT 'users', count(*) FROM users
       UNION ALL SELECT 'portfolios', count(*) FROM portfolios
       UNION ALL SELECT 'portfolio_asset', count(*) FROM portfolio_asset
       ORDER BY 1" \
      | tr -d '\r' \
      > "$BACKUP_DIR/pre-cutover-counts.txt"

### 3. Replace the PostgreSQL 17 data directory

Stop and remove the Compose containers without removing bind-mounted data:

    compose down

Save the PostgreSQL 17 directory by renaming it. Create a new empty directory
at the path used by the Compose bind mount:

    mv "$DCAPAL_DIR/data/db" "$OLD_DATA_DIR"
    mkdir "$DCAPAL_DIR/data/db"
    chmod 700 "$DCAPAL_DIR/data/db"

Save the current Compose file, then change only the database image to the
PostgreSQL 18 TimescaleDB tag. The deployment workflow will write the same
image on the next deploy, so this also keeps the host correct during the
intervening period:

    cp -p docker-compose.yml "$COMPOSE_BACKUP"

    sed -i \
      's#timescale/timescaledb-ha:pg17#timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss#g' \
      docker-compose.yml

    grep -Fq "image: $PG18_IMAGE" \
      docker-compose.yml

Do not start the backend yet. Pull and start only the fresh database:

    compose pull db
    compose up -d --no-deps --wait --wait-timeout 180 db

Verify that the new data directory is PostgreSQL 18 and that the TimescaleDB
extension is available:

    TARGET_MAJOR="$(compose exec -T db psql -U postgres -d postgres -Atqc \
      'SHOW server_version_num' | tr -d '[:space:]')"
    case "$TARGET_MAJOR" in
      18*) ;;
      *)
        echo "Expected PostgreSQL 18, found $TARGET_MAJOR" >&2
        exit 1
        ;;
    esac

    compose exec -T db psql -v ON_ERROR_STOP=1 \
      -U postgres -d postgres -c \
      "SELECT current_setting('server_version') AS server_version,
              extname,
              extversion
       FROM pg_extension
       WHERE extname = 'timescaledb';"

The fresh database initialization must have run the role scripts in
config/db/init. Verify the application login role from dcapal.env exists
before restoring the dump. If it does not, stop and restore the role using the
same deployment secret; do not continue with a missing application role.

    APP_USER="$(sed -n 's/^POSTGRES_USER=//p' dcapal.env)"
    test -n "$APP_USER"
    compose exec -T db psql -v ON_ERROR_STOP=1 \
      -U postgres -d postgres \
      -v app_user="$APP_USER" \
      -c "SELECT rolname, rolcanlogin
          FROM pg_roles
          WHERE rolname = :'app_user';"

### 4. Restore data and run DcaPal migrations

Restore the PostgreSQL 17 custom dump into the empty PostgreSQL 18 database:

    compose exec -T db pg_restore \
      -U postgres \
      -d postgres \
      --no-owner \
      --exit-on-error \
      < "$BACKUP_DIR/dcapal-pg17.dump"

Run the migration binary packaged in the DcaPal backend image. The command
uses the Compose-internal db:5432 address and the credentials already loaded
by dcapal.env; it does not expose a password in the command line:

    compose run --rm --no-deps \
      --entrypoint /bin/sh \
      dcapal \
      -ceu '
        export DATABASE_URL="postgresql://$POSTGRES_USER:$POSTGRES_PASSWORD@$POSTGRES_HOST:$POSTGRES_PORT/$POSTGRES_DB"
        exec /var/dcapal/dcapal-backend/bin/migration
      '

The migration command should complete successfully. Running it again when the
backend starts is expected and should report no new work unless the deployed
application image contains a newer migration.

### 5. Start and verify the complete stack

Start the full Compose stack and wait for its health checks:

    compose up -d --wait --wait-timeout 180
    compose ps

Verify the PostgreSQL version, migration history, and representative DcaPal
tables:

    compose exec -T db psql -v ON_ERROR_STOP=1 -U postgres -d postgres -c \
      "SELECT current_setting('server_version') AS server_version;
       SELECT extname, extversion
       FROM pg_extension WHERE extname = 'timescaledb';
       SELECT count(*) AS applied_migrations
       FROM _sqlx_migrations WHERE success = TRUE;
       SELECT count(*) AS failed_migrations
       FROM _sqlx_migrations WHERE success = FALSE;
       SELECT 'users' AS table_name, count(*) FROM users
       UNION ALL SELECT 'portfolios', count(*) FROM portfolios
       UNION ALL SELECT 'portfolio_asset', count(*) FROM portfolio_asset
       ORDER BY 1;"

Compare the three table counts with
$BACKUP_DIR/pre-cutover-counts.txt. Also verify the normal backend health
endpoint through the host port configured for the deployment. If the backend
uses the default API port, for example:

    curl --fail http://127.0.0.1:8080/

Finally, confirm that the Compose configuration still contains the exact
PostgreSQL 18 image:

    compose config --images | grep -Fx "$PG18_IMAGE"

Keep $BACKUP_DIR, $OLD_DATA_DIR, and $COMPOSE_BACKUP until the normal
application validation and the next deployment have completed successfully.

## Handoff to deploy.yml

The deployment workflow uploads a generated docker-compose.yml containing
the same PostgreSQL 18 image, uploads the production Compose override and
configuration, then runs:

    docker compose -f docker-compose.yml \
      -f ./docker/docker-compose.prod.yml pull
    docker compose -f docker-compose.yml \
      -f ./docker/docker-compose.prod.yml down
    docker compose -f docker-compose.yml \
      -f ./docker/docker-compose.prod.yml up -d

After this runbook completes, that sequence is safe because data/db is a
PostgreSQL 18 data directory. The next deployment does not need to restore the
database again. It will run the backend startup migration against the existing
TimescaleDB database and then start the application.

## Rollback

### Failure before PostgreSQL 18 receives writes

If verification fails before the backend has accepted writes, stop the stack,
move the unused PostgreSQL 18 directory aside, restore the retained
PostgreSQL 17 directory, and restore the saved Compose file:

    compose down
    mv "$DCAPAL_DIR/data/db" "$DCAPAL_DIR/data/db-pg18-failed-$STAMP"
    mv "$OLD_DATA_DIR" "$DCAPAL_DIR/data/db"
    cp -p "$COMPOSE_BACKUP" docker-compose.yml
    compose pull db
    compose up -d --wait --wait-timeout 180

Do not run down migrations. Keep the failed PostgreSQL 18 directory and the
dump for investigation.

### Failure after PostgreSQL 18 has received writes

The retained PostgreSQL 17 directory is now stale and must not be attached as
if it contains those writes. Prefer fixing the PostgreSQL 18 deployment
forward. If a PostgreSQL 17 rollback is required, create a fresh PostgreSQL 17
data directory, restore the pre-cutover custom dump into it, rerun the
migrations, and then start the backend. This restores the pre-cutover state and
does not preserve writes made after the cutover.

    compose down
    mv "$DCAPAL_DIR/data/db" "$DCAPAL_DIR/data/db-pg18-failed-$STAMP"
    cp -p "$COMPOSE_BACKUP" docker-compose.yml
    mkdir "$DCAPAL_DIR/data/db"
    chmod 700 "$DCAPAL_DIR/data/db"
    compose pull db
    compose up -d --no-deps --wait --wait-timeout 180 db
    compose exec -T db pg_restore \
      -U postgres -d postgres --no-owner --exit-on-error \
      < "$BACKUP_DIR/dcapal-pg17.dump"

Run the packaged migration command from the backend service, then start and
verify the full stack. Treat any post-cutover writes that are not present in
the dump as lost unless they are recovered separately.

## Disposable PostgreSQL 17 to 18 proof

For a local or CI-only verification of the procedure, run:

    ./dcapal-backend/scripts/verify-postgres-upgrade.sh

The proof starts disposable TimescaleDB PostgreSQL 17 and 18 containers, runs
the SQLx migrations, inserts representative users, portfolios, and assets,
creates a custom-format dump, restores it into PostgreSQL 18, reruns the
migrations, and checks the extension and restored data. It does not touch the
deployed host or the Supabase PostgreSQL instance.
