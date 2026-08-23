#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/dcapal-pg-upgrade.XXXXXX")"
SOURCE_NAME="dcapal-pg17-upgrade-$$"
TARGET_NAME="dcapal-pg18-upgrade-$$"
PASSWORD="${PG_UPGRADE_PASSWORD:-pg-upgrade-test}"
SOURCE_IMAGE="${PG17_IMAGE:-timescale/timescaledb-ha:pg17}"
TARGET_IMAGE="${PG18_IMAGE:-timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss}"

# Removes the disposable source and target databases and their temporary files.
cleanup() {
  docker rm -f "$SOURCE_NAME" "$TARGET_NAME" >/dev/null 2>&1 || true
  rm -rf "$TEMP_DIR"
}

trap cleanup EXIT

# Starts a disposable PostgreSQL database from the requested Timescale image.
start_database() {
  local name="$1"
  local image="$2"

  docker run \
    --detach \
    --name "$name" \
    --publish-all \
    --env POSTGRES_PASSWORD="$PASSWORD" \
    "$image" >/dev/null
}

# Waits until the database has completed initialization and accepts queries.
wait_for_database() {
  local name="$1"

  for _ in $(seq 1 90); do
    if docker logs "$name" 2>&1 |
      grep -F 'PostgreSQL init process complete; ready for start up.' >/dev/null \
      && docker exec "$name" psql \
        -v ON_ERROR_STOP=1 \
        -U postgres \
        -d postgres \
        -c 'SELECT 1' >/dev/null 2>&1; then
        return 0
    fi
    sleep 1
  done

  docker logs "$name" >&2 || true
  printf 'Timed out waiting for %s to accept PostgreSQL connections.\n' "$name" >&2
  exit 1
}

# Returns the host port Docker mapped to the container's PostgreSQL port.
mapped_port() {
  docker port "$1" 5432/tcp | head -n 1 | awk -F: '{print $NF}'
}

# Runs the packaged DcaPal migrations against a supplied database URL.
run_migrations() {
  local database_url="$1"

  cd "$ROOT_DIR"
  DATABASE_URL="$database_url" cargo run --quiet -p migration
}

# Restores a custom-format dump while leaving Timescale-owned catalog data intact.
restore_dump() {
  local name="$1"
  local dump_path="$2"
  local restore_list="$TEMP_DIR/${name}-restore.list"

  # TimescaleDB extensions and their internal catalog data are owned by the
  # target image. Exclude their archive entries before restoring it. The dump
  # also excludes this data at creation time; this list keeps the restore safe
  # for an archive created before that flag was added.
  docker exec -i "$name" pg_restore --list < "$dump_path" |
    sed -E \
      -e '/EXTENSION .*timescaledb(_toolkit)?/d' \
      -e '/TABLE DATA .*_timescaledb_(catalog|config)/d' \
    > "$restore_list"
  docker exec -i "$name" sh -c 'cat > /tmp/dcapal-restore.list' < "$restore_list"
  docker exec -i "$name" pg_restore \
    --use-list=/tmp/dcapal-restore.list \
    -U postgres \
    -d postgres \
    --no-owner \
    --exit-on-error \
    < "$dump_path"
}

start_database "$SOURCE_NAME" "$SOURCE_IMAGE"
wait_for_database "$SOURCE_NAME"
SOURCE_PORT="$(mapped_port "$SOURCE_NAME")"
SOURCE_URL="postgresql://postgres:${PASSWORD}@127.0.0.1:${SOURCE_PORT}/postgres"

run_migrations "$SOURCE_URL"

docker exec -i "$SOURCE_NAME" psql -v ON_ERROR_STOP=1 -U postgres -d postgres <<'SQL'
INSERT INTO users (id, email, role)
VALUES ('22222222-2222-4222-8222-222222222222', 'upgrade@example.com', 'authenticated');

INSERT INTO portfolios (id, user_id, name, currency, last_updated_at)
VALUES (
    '33333333-3333-4333-8333-333333333333',
    '22222222-2222-4222-8222-222222222222',
    'Upgrade portfolio',
    'usd',
    '2026-01-01T00:00:00Z'
);

INSERT INTO portfolio_asset (
    id,
    symbol,
    portfolio_id,
    name,
    asset_class,
    currency,
    provider,
    quantity,
    target_weight,
    manual_price,
    average_buy_price
)
VALUES (
    '44444444-4444-4444-8444-444444444444',
    'VWCE.MI',
    '33333333-3333-4333-8333-333333333333',
    'Upgrade asset',
    1,
    'usd',
    2,
    1,
    100,
    100,
    100
);
SQL

docker exec "$SOURCE_NAME" pg_dump \
  -U postgres \
  -d postgres \
  --format=custom \
  --no-owner \
  --exclude-table-data='_timescaledb_catalog.*' \
  --exclude-table-data='_timescaledb_config.*' \
  > "$TEMP_DIR/dcapal.dump"

start_database "$TARGET_NAME" "$TARGET_IMAGE"
wait_for_database "$TARGET_NAME"
TARGET_PORT="$(mapped_port "$TARGET_NAME")"
TARGET_URL="postgresql://postgres:${PASSWORD}@127.0.0.1:${TARGET_PORT}/postgres"

restore_dump "$TARGET_NAME" "$TEMP_DIR/dcapal.dump"

run_migrations "$TARGET_URL"

TARGET_VERSION="$(docker exec "$TARGET_NAME" psql -Atqc "SHOW server_version_num" -U postgres -d postgres)"
if [[ "$TARGET_VERSION" != 18* ]]; then
  printf 'Expected PostgreSQL 18 after restore, got %s.\n' "$TARGET_VERSION" >&2
  exit 1
fi

TARGET_EXTENSION_COUNT="$(docker exec "$TARGET_NAME" psql -Atqc \
  "SELECT COUNT(*) FROM pg_extension WHERE extname = 'timescaledb'" \
  -U postgres -d postgres)"
if [[ "$TARGET_EXTENSION_COUNT" != "1" ]]; then
  printf 'Expected TimescaleDB to be installed after restore.\n' >&2
  exit 1
fi

TARGET_ROW_COUNT="$(docker exec "$TARGET_NAME" psql -Atqc \
  "SELECT COUNT(*) FROM portfolios WHERE id = '33333333-3333-4333-8333-333333333333'" \
  -U postgres -d postgres)"
if [[ "$TARGET_ROW_COUNT" != "1" ]]; then
  printf 'Expected the representative portfolio to survive the restore.\n' >&2
  exit 1
fi

docker exec -i "$TARGET_NAME" psql -v ON_ERROR_STOP=1 -U postgres -d postgres <<'SQL'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM users
        WHERE id = '22222222-2222-4222-8222-222222222222'
          AND email = 'upgrade@example.com'
          AND role = 'authenticated'
    ) THEN
        RAISE EXCEPTION 'restored user is missing or changed';
    END IF;

    IF (
        SELECT COUNT(*)
        FROM portfolios AS p
        JOIN users AS u ON u.id = p.user_id
        WHERE p.id = '33333333-3333-4333-8333-333333333333'
          AND u.id = '22222222-2222-4222-8222-222222222222'
          AND p.name = 'Upgrade portfolio'
          AND p.currency = 'usd'
          AND p.deleted = FALSE
    ) <> 1 THEN
        RAISE EXCEPTION 'restored linked portfolio is missing or changed';
    END IF;

    IF (
        SELECT COUNT(*)
        FROM portfolio_asset AS a
        JOIN portfolios AS p ON p.id = a.portfolio_id
        WHERE a.id = '44444444-4444-4444-8444-444444444444'
          AND p.id = '33333333-3333-4333-8333-333333333333'
          AND a.symbol = 'VWCE.MI'
          AND a.name = 'Upgrade asset'
          AND a.asset_class = 1
          AND a.currency = 'usd'
          AND a.provider = 2
          AND a.quantity = 1
          AND a.target_weight = 100
          AND a.manual_price = 100
          AND a.average_buy_price = 100
    ) <> 1 THEN
        RAISE EXCEPTION 'restored portfolio asset is missing or changed';
    END IF;

    IF (SELECT COUNT(*) FROM _sqlx_migrations) < 6
       OR EXISTS (SELECT 1 FROM _sqlx_migrations WHERE success = FALSE) THEN
        RAISE EXCEPTION 'migration history is incomplete or contains a failed migration';
    END IF;
END
$$;
SQL

printf 'PostgreSQL 17 to 18 custom-format restore verified successfully.\n'
