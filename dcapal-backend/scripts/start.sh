#!/usr/bin/env bash
set -Eeuo pipefail

# The container entrypoint receives the database contract from Compose.
DB_PORT="${POSTGRES_PORT:-5432}"
APP_USER="${POSTGRES_USER:-postgres}"
APP_USER_PWD="${POSTGRES_PASSWORD:-postgres}"
APP_DB_NAME="${POSTGRES_DB:-postgres}"
DB_HOST="${POSTGRES_HOST:-db}"

# Wait for the database to be ready
export PGPASSWORD="${APP_USER_PWD}"

# Wait for the database server to accept connections. This check does not
# confuse a bad password with a database that is still starting.
until pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$APP_USER" -d "$APP_DB_NAME" >/dev/null 2>&1; do
    echo >&2 "Postgres is unavailable - sleeping"
    sleep 1
done

echo >&2 "Postgres is up - executing command"

if ! psql -h "$DB_HOST" -p "$DB_PORT" -U "$APP_USER" -d "$APP_DB_NAME" -c '\q' >/dev/null; then
    echo >&2 "Postgres is reachable, but its credentials were rejected. Check POSTGRES_* values or run make local-reset."
    exit 1
fi

DATABASE_URL="${DATABASE_URL:-postgresql://${APP_USER}:${APP_USER_PWD}@${DB_HOST}:${DB_PORT}/${APP_DB_NAME}}"
export DATABASE_URL
# Run migrations
migration_log=/tmp/dcapal-migration.log
rm -f "$migration_log"
if ! /var/dcapal/dcapal-backend/bin/migration up -u "$DATABASE_URL" > "$migration_log" 2>&1; then
    sed -E 's#(postgresql://[^:]+:)[^@]+@#\1<REDACTED>@#g' "$migration_log" >&2 || true
    echo >&2 "Database migrations failed. If the database password is mismatched, run make local-reset."
    exit 1
fi
rm -f "$migration_log"

# Start the application
exec /var/dcapal/dcapal-backend/bin/dcapal-backend
