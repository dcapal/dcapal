#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$(cd "$BACKEND_DIR/.." && pwd)"

LOCAL_STATE_DIR="${LOCAL_STATE_DIR:-$ROOT_DIR/.local}"
LOCAL_ENV_FILE="${LOCAL_ENV_FILE:-$BACKEND_DIR/docker/local.env}"
SUPABASE_ENV_FILE="$LOCAL_STATE_DIR/supabase.env"
SUPABASE_WORKDIR="$BACKEND_DIR/config"
WASM_PACK_VERSION="0.13.1"
WASM_BINDGEN_VERSION="0.2.126"
SUPABASE_VERSION="2.110.0"
TOOL_ROOT="$LOCAL_STATE_DIR/rust-tools"
TOOL_BIN="$TOOL_ROOT/bin"
BACKEND_LOG="$LOCAL_STATE_DIR/backend.log"
SUPABASE_LOG_FILE="$LOCAL_STATE_DIR/supabase.log"

log() {
  printf '[local-stack] %s\n' "$*"
}

die() {
  printf '[local-stack] ERROR: %s\n' "$*" >&2
  exit 1
}

ensure_local_env() {
  mkdir -p "$LOCAL_STATE_DIR"
  mkdir -p "$(dirname "$LOCAL_ENV_FILE")"
  if [[ ! -f "$LOCAL_ENV_FILE" ]]; then
    cp "$BACKEND_DIR/docker/local.env.example" "$LOCAL_ENV_FILE"
    chmod 600 "$LOCAL_ENV_FILE"
    log "Created $LOCAL_ENV_FILE"
  fi
}

load_local_env() {
  ensure_local_env

  local inherited_postgres_user="${POSTGRES_USER-}"
  local inherited_postgres_password="${POSTGRES_PASSWORD-}"
  local inherited_postgres_db="${POSTGRES_DB-}"
  local inherited_postgres_host_port="${POSTGRES_HOST_PORT-}"
  local inherited_redis_host_port="${REDIS_HOST_PORT-}"
  local inherited_backend_port="${BACKEND_PORT-}"
  local inherited_metrics_port="${METRICS_PORT-}"
  local inherited_frontend_port="${FRONTEND_PORT-}"
  local inherited_project_name="${COMPOSE_PROJECT_NAME-}"

  set -a
  # shellcheck disable=SC1090
  source "$LOCAL_ENV_FILE"
  set +a

  [[ -n "$inherited_postgres_user" ]] && POSTGRES_USER="$inherited_postgres_user"
  [[ -n "$inherited_postgres_password" ]] && POSTGRES_PASSWORD="$inherited_postgres_password"
  [[ -n "$inherited_postgres_db" ]] && POSTGRES_DB="$inherited_postgres_db"
  [[ -n "$inherited_postgres_host_port" ]] && POSTGRES_HOST_PORT="$inherited_postgres_host_port"
  [[ -n "$inherited_redis_host_port" ]] && REDIS_HOST_PORT="$inherited_redis_host_port"
  [[ -n "$inherited_backend_port" ]] && BACKEND_PORT="$inherited_backend_port"
  [[ -n "$inherited_metrics_port" ]] && METRICS_PORT="$inherited_metrics_port"
  [[ -n "$inherited_frontend_port" ]] && FRONTEND_PORT="$inherited_frontend_port"

  POSTGRES_USER="${POSTGRES_USER:-postgres}"
  POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-postgres}"
  POSTGRES_DB="${POSTGRES_DB:-postgres}"
  POSTGRES_HOST_PORT="${POSTGRES_HOST_PORT:-5433}"
  REDIS_HOST_PORT="${REDIS_HOST_PORT:-6379}"
  BACKEND_PORT="${BACKEND_PORT:-8080}"
  METRICS_PORT="${METRICS_PORT:-9000}"
  FRONTEND_PORT="${FRONTEND_PORT:-3000}"
  GRAFANA_PORT="${GRAFANA_PORT:-3001}"
  BACKEND_READY_TIMEOUT="${BACKEND_READY_TIMEOUT:-180}"

  local project_parent
  project_parent="$(basename "$(dirname "$ROOT_DIR")")"
  project_parent="$(printf '%s' "$project_parent" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9_-' '-')"
  COMPOSE_PROJECT_NAME="${inherited_project_name:-dcapal-${project_parent}}"

  export POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB POSTGRES_HOST_PORT
  export REDIS_HOST_PORT BACKEND_PORT METRICS_PORT FRONTEND_PORT GRAFANA_PORT
  export BACKEND_READY_TIMEOUT
  export COMPOSE_PROJECT_NAME
  export LOCAL_STATE_DIR LOCAL_ENV_FILE
}

ensure_wasm_tools() {
  mkdir -p "$TOOL_BIN"
  export PATH="$TOOL_BIN:$PATH"
  export CARGO_HOME="${CARGO_HOME:-$LOCAL_STATE_DIR/cargo-home}"

  if ! wasm-pack --version 2>/dev/null | grep -q "wasm-pack $WASM_PACK_VERSION"; then
    log "Installing wasm-pack $WASM_PACK_VERSION in local tool state"
    cargo install wasm-pack --locked --version "$WASM_PACK_VERSION" --root "$TOOL_ROOT"
  fi

  if ! wasm-bindgen --version 2>/dev/null | grep -q "wasm-bindgen $WASM_BINDGEN_VERSION"; then
    log "Installing wasm-bindgen-cli $WASM_BINDGEN_VERSION in local tool state"
    cargo install wasm-bindgen-cli --locked --version "$WASM_BINDGEN_VERSION" --root "$TOOL_ROOT"
  fi
}

build_optimizer() {
  ensure_wasm_tools
  local package_dir="$ROOT_DIR/dcapal-optimizer-wasm/pkg"
  if [[ -f "$package_dir/package.json" && "${FORCE_OPTIMIZER:-0}" != "1" ]]; then
    log "Using existing optimizer package"
    return
  fi

  log "Building optimizer WASM package"
  local optimizer_target_dir="${CARGO_TARGET_DIR:-$LOCAL_STATE_DIR/optimizer-target}"
  (
    cd "$ROOT_DIR/dcapal-optimizer-wasm/crates/optimizer"
    CARGO_TARGET_DIR="$optimizer_target_dir" \
      wasm-pack build --dev --mode no-install --out-dir ../../pkg
  )
}

install_node_dependencies() {
  local stamp="$LOCAL_STATE_DIR/pnpm-install.stamp"
  if [[ -f "$ROOT_DIR/node_modules/.modules.yaml" \
    && "$stamp" -nt "$ROOT_DIR/pnpm-lock.yaml" \
    && "$stamp" -nt "$ROOT_DIR/package.json" \
    && "$stamp" -nt "$ROOT_DIR/dcapal-frontend/package.json" \
    && "$stamp" -nt "$ROOT_DIR/packages/api-client/package.json" ]]; then
    log "Using existing frontend dependencies"
    return
  fi

  log "Installing frontend dependencies"
  (
    cd "$ROOT_DIR"
    pnpm install --frozen-lockfile
  )
  touch "$stamp"
}

bootstrap() {
  load_local_env
  build_optimizer
  install_node_dependencies
}

run_supabase() {
  (
    cd "$ROOT_DIR"
    XDG_CACHE_HOME="$LOCAL_STATE_DIR/pnpm-cache" \
    SUPABASE_HOME="$LOCAL_STATE_DIR/supabase" \
      SUPABASE_TELEMETRY_DISABLED=1 \
      pnpm dlx --package "supabase@$SUPABASE_VERSION" supabase "$@"
  )
}

capture_supabase_env() {
  mkdir -p "$LOCAL_STATE_DIR"
  run_supabase status --workdir "$SUPABASE_WORKDIR" -o env > "$SUPABASE_ENV_FILE"
  set -a
  # shellcheck disable=SC1090
  source "$SUPABASE_ENV_FILE"
  set +a
  chmod 600 "$SUPABASE_ENV_FILE"

  : "${API_URL:?Supabase did not report API_URL}"
  : "${ANON_KEY:?Supabase did not report ANON_KEY}"
  : "${JWT_SECRET:?Supabase did not report JWT_SECRET}"
  export API_URL ANON_KEY JWT_SECRET
}

start_supabase() {
  log "Starting Supabase"
  if ! run_supabase start --workdir "$SUPABASE_WORKDIR" > "$SUPABASE_LOG_FILE" 2>&1; then
    printf '[local-stack] ERROR: Supabase failed to start; inspect %s\n' "$SUPABASE_LOG_FILE" >&2
    return 1
  fi
  capture_supabase_env
}

compose_core() {
  docker compose -p "$COMPOSE_PROJECT_NAME" \
    -f "$BACKEND_DIR/docker-compose.yml" \
    -f "$BACKEND_DIR/docker/docker-compose.dev.yml" "$@"
}

compose_docker() {
  compose_core -f "$BACKEND_DIR/docker/docker-compose.local.yml" "$@"
}

compose_observability() {
  compose_core -f "$BACKEND_DIR/docker/docker-compose.observability.yml" "$@"
}

start_core() {
  log "Starting Redis and TimescaleDB"
  compose_core up -d --wait --wait-timeout 180 db redis
}

start_dependencies() {
  # Supabase and the application dependencies are independent and can start in
  # parallel. Keeping this orchestration here gives all Make targets one seam.
  start_core &
  local core_pid=$!
  if ! start_supabase; then
    kill "$core_pid" 2>/dev/null || true
    wait "$core_pid" 2>/dev/null || true
    compose_core down --remove-orphans >/dev/null 2>&1 || true
    return 1
  fi
  if ! wait "$core_pid"; then
    stop_supabase
    compose_core down --remove-orphans >/dev/null 2>&1 || true
    return 1
  fi
}

render_config() {
  local mode="${1:-host}"
  local redis_hostname="127.0.0.1"
  local redis_port="$REDIS_HOST_PORT"
  local postgres_hostname="127.0.0.1"
  local postgres_port="$POSTGRES_HOST_PORT"
  local web_hostname="127.0.0.1"
  local metrics_hostname="127.0.0.1"
  local web_port="$BACKEND_PORT"
  local metrics_port="$METRICS_PORT"

  if [[ "$mode" == "container" ]]; then
    redis_hostname="redis"
    redis_port="6379"
    postgres_hostname="db"
    postgres_port="5432"
    web_hostname="0.0.0.0"
    metrics_hostname="0.0.0.0"
    web_port="8080"
    metrics_port="9000"
  elif [[ "$mode" != "host" ]]; then
    die "Unknown configuration mode: $mode"
  fi

  export DCAPAL_JWT_SECRET="$JWT_SECRET"
  export DCAPAL_JWT_JWKS="$(curl --fail --silent --show-error "$API_URL/auth/v1/.well-known/jwks.json")"
  export DCAPAL_WEB_HOSTNAME="$web_hostname"
  export DCAPAL_WEB_PORT="$web_port"
  export DCAPAL_METRICS_HOSTNAME="$metrics_hostname"
  export DCAPAL_METRICS_PORT="$metrics_port"
  export DCAPAL_REDIS_HOSTNAME="$redis_hostname"
  export DCAPAL_REDIS_PORT="$redis_port"
  export DCAPAL_REDIS_USER=dcapal
  export DCAPAL_REDIS_PASSWORD=dcapal
  export DCAPAL_POSTGRES_HOSTNAME="$postgres_hostname"
  export DCAPAL_POSTGRES_PORT="$postgres_port"
  export DCAPAL_POSTGRES_USER="$POSTGRES_USER"
  export DCAPAL_POSTGRES_PASSWORD="$POSTGRES_PASSWORD"
  export DCAPAL_POSTGRES_DATABASE="$POSTGRES_DB"
  export DCAPAL_LOG_FILE=data/dcapal/dcapal.log
  export DCAPAL_LOG_ENABLE_STDOUT=true

  log "Rendering $mode backend configuration"
  (
    cd "$BACKEND_DIR"
    PYTHONDONTWRITEBYTECODE=1 python3 scripts/render-dcapal-config.py --output dcapal.yml
  )
}

run_migrations() {
  local database_url="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:${POSTGRES_HOST_PORT}/${POSTGRES_DB}"
  local migration_log="$LOCAL_STATE_DIR/migration.log"
  log "Applying database migrations"
  if ! (
    cd "$ROOT_DIR"
    DATABASE_URL="$database_url" cargo run -p migration
  ) > "$migration_log" 2>&1; then
    sed -E 's#(postgresql://[^:]+:)[^@]+@#\1<REDACTED>@#g' "$migration_log" >&2 || true
    die "Database migrations failed. If the database password is mismatched, run make local-reset to recreate local volumes."
  fi
  rm -f "$migration_log"
}

wait_for_url() {
  local url="$1"
  local attempts="$BACKEND_READY_TIMEOUT"
  while (( attempts > 0 )); do
    if curl --fail --silent --max-time 2 "$url" >/dev/null; then
      return 0
    fi
    sleep 1
    attempts=$((attempts - 1))
  done
  return 1
}

check_backend() {
  local backend_url="http://127.0.0.1:${BACKEND_PORT}"
  wait_for_url "$backend_url/" || die "Backend did not become ready; inspect $BACKEND_LOG"
  curl --fail --silent "$backend_url/assets/fiat" >/dev/null \
    || die "Backend asset endpoint is unavailable"
  curl --fail --silent "$backend_url/assets/crypto" >/dev/null \
    || die "Backend crypto endpoint is unavailable"
  log "Backend is ready at $backend_url"
}

run_frontend() {
  export VITE_SUPABASE_URL="$API_URL"
  export VITE_SUPABASE_ANON_KEY="$ANON_KEY"
  export WATCHPACK_POLLING="${WATCHPACK_POLLING:-true}"
  export CHOKIDAR_USEPOLLING="${CHOKIDAR_USEPOLLING:-true}"
  export CHOKIDAR_INTERVAL="${CHOKIDAR_INTERVAL:-500}"
  log "Starting frontend at http://127.0.0.1:${FRONTEND_PORT}"
  cd "$ROOT_DIR"
  exec pnpm frontend:dev:ci
}

start_host() {
  bootstrap
  start_dependencies
  render_config host
  # A previous Docker-mode run may still own the published backend port.
  # Remove only this project's backend container before starting the host
  # process; the database, Redis, and Supabase services remain available.
  compose_docker rm --force --stop dcapal >/dev/null 2>&1 || true
  run_migrations

  if [[ "${1:-backend}" == "ui" ]]; then
    : > "$BACKEND_LOG"
    (
      cd "$BACKEND_DIR"
      cargo run --manifest-path crates/backend/Cargo.toml
    ) > "$BACKEND_LOG" 2>&1 &
    local backend_pid=$!
    cleanup_host() {
      kill "$backend_pid" 2>/dev/null || true
      wait "$backend_pid" 2>/dev/null || true
    }
    trap cleanup_host EXIT INT TERM
    check_backend
    run_frontend
  fi

  log "Starting host backend"
  cd "$BACKEND_DIR"
  exec cargo run --manifest-path crates/backend/Cargo.toml
}

start_docker() {
  bootstrap
  start_dependencies
  render_config container
  log "Building and starting the local backend image"
  if ! compose_docker up -d --build --force-recreate --wait --wait-timeout 180 dcapal; then
    compose_docker logs --no-color --tail 40 dcapal >&2 || true
    die "Docker backend failed to start. If the database password is mismatched, run make local-reset to recreate local volumes."
  fi
  check_backend

  if [[ "${1:-backend}" == "ui" ]]; then
    run_frontend
  fi
}

stop_supabase() {
  if [[ -f "$SUPABASE_ENV_FILE" || -d "$LOCAL_STATE_DIR/supabase" ]]; then
    log "Stopping Supabase"
    run_supabase stop --workdir "$SUPABASE_WORKDIR" || true
  fi
}

stop_host() {
  load_local_env
  compose_core down --remove-orphans
  stop_supabase
}

stop_docker() {
  load_local_env
  compose_docker down --remove-orphans
  stop_supabase
}

reset_stack() {
  load_local_env
  compose_docker down --volumes --remove-orphans
  compose_observability --profile observability down --volumes --remove-orphans >/dev/null 2>&1 || true
  stop_supabase
  log "Removed local Compose containers and named volumes for $COMPOSE_PROJECT_NAME"
}

observability_up() {
  load_local_env
  compose_observability --profile observability up -d
}

observability_down() {
  load_local_env
  compose_observability --profile observability rm --force --stop prometheus grafana cadvisor
}

doctor() {
  local failures=0
  local required_node
  required_node="$(tr -d '[:space:]' < "$ROOT_DIR/.nvmrc")"

  for command_name in docker cargo pnpm curl python3; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
      printf '[local-stack] missing command: %s\n' "$command_name" >&2
      failures=$((failures + 1))
    fi
  done

  if command -v docker >/dev/null 2>&1 && ! docker info >/dev/null 2>&1; then
    printf '[local-stack] Docker daemon is not available\n' >&2
    failures=$((failures + 1))
  fi

  if command -v docker >/dev/null 2>&1; then
    docker compose version >/dev/null 2>&1 || {
      printf '[local-stack] Docker Compose plugin is not available\n' >&2
      failures=$((failures + 1))
    }
  fi

  if command -v node >/dev/null 2>&1; then
    local node_major
    node_major="$(node --version | sed 's/^v//' | cut -d. -f1)"
    if [[ "$node_major" != "$required_node" ]]; then
      printf '[local-stack] warning: repository expects Node %s, found Node %s\n' "$required_node" "$node_major" >&2
    fi
  fi

  if command -v wasm-pack >/dev/null 2>&1; then
    wasm-pack --version
  else
    printf '[local-stack] wasm-pack will be bootstrapped by a local workflow\n'
  fi

  if (( failures > 0 )); then
    return 1
  fi
  log "Local environment prerequisites look good"
}

usage() {
  cat <<'EOF'
Usage: local-stack.sh <command>

Commands:
  bootstrap          Prepare local tools, the optimizer package, and pnpm dependencies
  optimizer          Build the optimizer package
  supabase-up        Start Supabase and capture its local environment
  supabase-down      Stop Supabase
  render-host        Render configuration for a host-run backend
  render-container   Render configuration for a container backend
  up-host            Start dependencies and the host-run backend
  up-host-ui         Start dependencies, the host-run backend, and the frontend
  up-docker          Start dependencies and the Docker backend
  up-docker-ui       Start dependencies, the Docker backend, and the frontend
  down-host          Stop host-mode dependencies and Supabase
  down-docker        Stop Docker-mode dependencies and Supabase
  reset              Remove local Compose containers and named volumes
  observability-up   Start the optional observability profile
  observability-down Stop the optional observability profile
  doctor             Check local development prerequisites
EOF
}

command_name="${1:-}"
case "$command_name" in
  bootstrap)
    bootstrap
    ;;
  optimizer)
    load_local_env
    build_optimizer
    ;;
  supabase-up)
    bootstrap
    start_supabase
    ;;
  supabase-down)
    load_local_env
    stop_supabase
    ;;
  render-host)
    load_local_env
    capture_supabase_env
    render_config host
    ;;
  render-container)
    load_local_env
    capture_supabase_env
    render_config container
    ;;
  up-host)
    start_host backend
    ;;
  up-host-ui)
    start_host ui
    ;;
  up-docker)
    start_docker backend
    ;;
  up-docker-ui)
    start_docker ui
    ;;
  down-host)
    stop_host
    ;;
  down-docker)
    stop_docker
    ;;
  reset)
    reset_stack
    ;;
  observability-up)
    observability_up
    ;;
  observability-down)
    observability_down
    ;;
  doctor)
    doctor
    ;;
  *)
    usage
    exit 2
    ;;
esac
