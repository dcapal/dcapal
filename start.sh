#!/bin/bash

# Wait for the database to be ready
until sqlx database ping; do
  echo "Waiting for database..."
  sleep 1
done

# Run migrations
sqlx migrate run

# Start your application
exec /var/dcapal/dcapal-backend/bin/dcapal-backend
