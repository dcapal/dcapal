#!/bin/bash
set -e

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${POSTGRES_PORT:=5432}"
APP_USER="${POSTGRES_USER:=postgres}"
APP_USER_PWD="${POSTGRES_PASSWORD:=postgres}"
APP_DB_NAME="${POSTGRES_DB:=postgres}"
DB_HOST="${POSTGRES_HOST:=postgres}"

# Wait for the database to be ready
# Set the password first
export PGPASSWORD="${APP_USER_PWD}"

# Then remove PGPASSWORD from the psql command
until psql -h "$DB_HOST" -p "$DB_PORT" -U "$APP_USER" -d "$APP_DB_NAME" -c '\q'; do
    echo >&2 "Postgres is unavailable - sleeping"
    sleep 1
done

echo >&2 "Postgres is up - executing command"

# Create the application database
DATABASE_URL=postgresql://${APP_USER}:${APP_USER_PWD}@${DB_HOST}:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
# Run migrations
# cargo run --bin migration -- refresh -u $DATABASE_URL
/var/dcapal/dcapal-backend/bin/migration refresh -u "$DATABASE_URL"

# Start the application
exec /var/dcapal/dcapal-backend/bin/dcapal-backend
