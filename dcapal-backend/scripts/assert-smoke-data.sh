#!/usr/bin/env bash
set -euo pipefail

BACKEND_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-postgres}"
export POSTGRES_PASSWORD
SMOKE_USER_EMAIL="${SMOKE_USER_EMAIL:-smoke@example.com}"
SMOKE_PORTFOLIO_ID="${SMOKE_PORTFOLIO_ID:-11111111-1111-4111-8111-111111111111}"
SMOKE_PORTFOLIO_NAME="${SMOKE_PORTFOLIO_NAME:-Smoke portfolio}"

cd "$BACKEND_DIR"

# Runs queries against the TimescaleDB service used by the smoke test.
compose() {
  local compose_files=(
    -f docker-compose.yml
  )

  docker compose "${compose_files[@]}" "$@"
}

# Returns a compact query result for the seeded smoke records.
query() {
  compose exec -T db psql \
    -U postgres \
    -d postgres \
    -v ON_ERROR_STOP=1 \
    -v smoke_email="$SMOKE_USER_EMAIL" \
    -v smoke_portfolio_id="$SMOKE_PORTFOLIO_ID" \
    -v smoke_portfolio_name="$SMOKE_PORTFOLIO_NAME" \
    -Atq <<SQL | tr -d '[:space:]'
$1
SQL
}

# Fails the smoke test when a database value differs from its expected value.
assert_value() {
  local description="$1"
  local expected="$2"
  local actual="$3"

  if [[ "$actual" != "$expected" ]]; then
    printf 'Smoke data assertion failed for %s: expected %s, got %s\n' \
      "$description" "$expected" "$actual" >&2
    exit 1
  fi
}

assert_value \
  "PostgreSQL major version" \
  "18" \
  "$(query "SELECT split_part(current_setting('server_version'), '.', 1)")"

assert_value \
  "authenticated smoke user" \
  "1" \
  "$(query "SELECT COUNT(*) FROM users WHERE email = :'smoke_email' AND role = 'authenticated'")"

assert_value \
  "saved smoke portfolio" \
  "1" \
  "$(query "
    SELECT COUNT(*)
    FROM portfolios AS p
    JOIN users AS u ON u.id = p.user_id
    WHERE u.email = :'smoke_email'
      AND p.id = :'smoke_portfolio_id'::uuid
      AND p.name = :'smoke_portfolio_name'
      AND p.deleted = FALSE
  ")"

assert_value \
  "saved smoke portfolio asset" \
  "1" \
  "$(query "
    SELECT COUNT(*)
    FROM portfolio_asset AS a
    JOIN portfolios AS p ON p.id = a.portfolio_id
    JOIN users AS u ON u.id = p.user_id
    WHERE u.email = :'smoke_email'
      AND p.id = :'smoke_portfolio_id'::uuid
      AND a.symbol = 'VWCE.MI'
      AND a.asset_class = 1
      AND a.currency = 'usd'
      AND a.provider = 2
      AND a.quantity = 1
      AND a.target_weight = 100
      AND a.manual_price = 100
  ")"

printf 'Timescale smoke data verified for %s and %s.\n' \
  "$SMOKE_USER_EMAIL" "$SMOKE_PORTFOLIO_ID"
