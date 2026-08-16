SHELL := /bin/bash

COMPOSE_DEV_ARGS := -f docker-compose.yml -f docker/docker-compose.dev.yml
COMPOSE_TEST_ARGS := -f docker-compose.yml -f docker/docker-compose.test.yml
DCAPAL_BACKEND_DIR := ./dcapal-backend
DCAPAL_OPTIMIZER_DIR := ./dcapal-optimizer-wasm/crates/optimizer
DCAPAL_FRONTEND_DIR := ./dcapal-frontend
SUPABASE_WORKDIR := ./config
POSTGRES_PASSWORD ?= postgres
TIMESCALE_DATABASE_URL ?= postgresql://postgres:$(POSTGRES_PASSWORD)@127.0.0.1:5433/postgres
DATABASE_URL ?= $(TIMESCALE_DATABASE_URL)

.PHONY: help supabase-up supabase-down render-local-config local-up local-down backend-db-up backend-db-down backend-db-check backend-migrate test-backend docker-dev-up docker-dev-down dev-up dev-down export-openapi

## Show this help message
help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*##"}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## Format codebase
fmt:  ## Format codebase
	cargo +nightly fmt --all -- --config-path rustfmt.nightly.toml
	pnpm frontend:format

## Run Rust linters
lint-rust: ## Run Rust linters
	cargo +nightly fmt --all -- --config-path rustfmt.nightly.toml --check
	cargo +nightly clippy -- -D warnings

## Run JS linters
lint-js: ## Run JS linters
	pnpm frontend:lint

## Run linters on the codebase
lint: lint-rust lint-js  ## Run linters on the codebase

## Build backend
build-backend: ## Build backend
	cd $(DCAPAL_BACKEND_DIR) && cargo build --manifest-path crates/backend/Cargo.toml

## Export backend OpenAPI spec
export-openapi: ## Export backend OpenAPI spec
	cargo run -p openapi-generator --bin openapi-generator -- dcapal-backend/docs/openapi.json

## Build optimizer-wasm
build-optimizer: ## Build optimizer-wasm
	cd $(DCAPAL_OPTIMIZER_DIR) && wasm-pack build --dev --out-dir ../../pkg

## Build frontend
build-frontend: ## Build frontend
	pnpm frontend:build:dev

## Build all
build: build-backend build-optimizer build-frontend  ## Build all

## Test frontend (unit-tests)
test-frontend-unit: ## Run frontend tests
	pnpm frontend:test

## Test frontend (e2e)
test-frontend-e2e: ## Run frontend tests
	pnpm frontend:test:e2e

## Test frontend
test-frontend: test-frontend-unit test-frontend-e2e ## Run all frontend tests

## Run backend (dev)
run-backend-dev: ## Run backend (dev)
	cd $(DCAPAL_BACKEND_DIR) && cargo run --manifest-path crates/backend/Cargo.toml

## Run frontend (dev)
run-frontend-dev: ## Run frontend (dev)
	cd $(DCAPAL_OPTIMIZER_DIR) && wasm-pack build --dev --out-dir ../../pkg
	pnpm frontend:dev

## Start Supabase with config
supabase-up:  ## Start Supabase with config
	cd $(DCAPAL_BACKEND_DIR) && npx supabase start --workdir $(SUPABASE_WORKDIR)

## Stop Supabase
supabase-down:  ## Stop Supabase
	cd $(DCAPAL_BACKEND_DIR) && npx supabase stop --workdir $(SUPABASE_WORKDIR)

## Start the TimescaleDB database used by backend development and tests
backend-db-up:  ## Start the backend database
	cd $(DCAPAL_BACKEND_DIR) && env POSTGRES_PASSWORD="$(POSTGRES_PASSWORD)" docker compose $(COMPOSE_TEST_ARGS) up -d --wait --wait-timeout 180 db

## Stop the TimescaleDB database used by backend development and tests
backend-db-down:  ## Stop the backend database
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_TEST_ARGS) stop db
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_TEST_ARGS) rm --force db

## Check that the backend database is already running
backend-db-check:  ## Check backend database connectivity
	@psql "$(DATABASE_URL)" -c "SELECT 1" >/dev/null

## Apply pending SQLx migrations to the backend database
backend-migrate: backend-db-check  ## Apply backend database migrations
	@DATABASE_URL="$(DATABASE_URL)" cargo run -p migration

## Run the full backend test suite against an already-running database
test-backend: backend-db-check  ## Run backend tests (requires backend-db-up)
	@DATABASE_URL="$(DATABASE_URL)" RUST_LOG=dcapal_backend=debug cargo test -p dcapal-backend -p migration -- --nocapture

## Start development Docker containers
docker-dev-up:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_DEV_ARGS) up -d

## Stop development Docker containers
docker-dev-down:  ## Stop development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_DEV_ARGS) down

## Start development Docker containers with Dcapal image
docker-local-build:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_DEV_ARGS) -f docker/docker-compose.local.yml build

## Start development Docker containers with Dcapal image
docker-local-up:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_DEV_ARGS) -f docker/docker-compose.local.yml up -d

## Start development Docker containers with Dcapal image
docker-local-down:  ## Stop development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_DEV_ARGS) -f docker/docker-compose.local.yml down

## Start full dev environment (Supabase + Docker)
dev-up: supabase-up docker-dev-up  ## Start full dev environment (Supabase + Docker)

## Stop full dev environment
dev-down: docker-dev-down supabase-down  ## Stop full dev environment
## Start full dev+local environment (Supabase + Docker)

render-local-config: supabase-up  ## Render the local backend config from Supabase's signing keys
	cd $(DCAPAL_BACKEND_DIR) && \
		eval "$$(npx supabase status --workdir ./config -o env)" && \
		DCAPAL_JWT_SECRET="$$JWT_SECRET" \
		DCAPAL_JWT_JWKS="$$(curl --fail --silent --show-error "$$API_URL/auth/v1/.well-known/jwks.json")" \
		python3 scripts/render-dcapal-config.py --output dcapal.yml

local-up: render-local-config  ## Start full dev environment (Supabase + Docker)
	$(MAKE) docker-local-up

## Stop full dev+local environment
local-down: docker-local-down supabase-down  ## Stop full dev environment

## Merge all dependabot PRs
chore-merge-dependabot:  ## Merge all dependabot PRs
	script/merge-dependabot.sh dcapal/dcapal master
