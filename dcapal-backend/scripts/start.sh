#!/bin/bash
set -e

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${DB_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=postgres}"
APP_USER="${APP_USER:=postgres}"
APP_USER_PWD="${APP_USER_PWD:=postgres}"
APP_DB_NAME="${APP_DB_NAME:=postgres}"
DB_HOST="${DB_HOST:=postgres-db}"

# Wait for the database to be ready
until PGPASSWORD=$POSTGRES_PASSWORD psql -h "$DB_HOST" -U "$APP_USER" -d "$APP_DB_NAME" -c '\q'; do
  echo >&2 "Postgres is unavailable - sleeping"
  sleep 1
done

echo >&2 "Postgres is up - executing command"

# Create the application database
DATABASE_URL=postgres://${APP_USER}:${APP_USER_PWD}@postgres-db:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
# Run migrations
sqlx migrate run

# Start the application
exec /var/dcapal/dcapal-backend/bin/dcapal-backend
