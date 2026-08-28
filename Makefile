SHELL := /bin/bash

REPO_ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
DCAPAL_BACKEND_DIR := ./dcapal-backend
DCAPAL_OPTIMIZER_DIR := ./dcapal-optimizer-wasm/crates/optimizer
DCAPAL_FRONTEND_DIR := ./dcapal-frontend
LOCAL_STACK_SCRIPT := $(DCAPAL_BACKEND_DIR)/scripts/local-stack.sh
COMPOSE_PROJECT_NAME ?= dcapal-$(notdir $(patsubst %/,%,$(dir $(REPO_ROOT))))
COMPOSE_PROJECT_ARGS := -p $(COMPOSE_PROJECT_NAME)
COMPOSE_CORE_ARGS := $(COMPOSE_PROJECT_ARGS) -f docker-compose.yml -f docker/docker-compose.dev.yml
COMPOSE_TEST_ARGS := $(COMPOSE_PROJECT_ARGS) -f docker-compose.yml -f docker/docker-compose.test.yml
COMPOSE_LOCAL_ARGS := $(COMPOSE_CORE_ARGS) -f docker/docker-compose.local.yml
COMPOSE_OBSERVABILITY_ARGS := $(COMPOSE_CORE_ARGS) -f docker/docker-compose.observability.yml
SUPABASE_WORKDIR := ./config
LOCAL_ENV_FILE ?= $(REPO_ROOT)/dcapal-backend/docker/local.env
POSTGRES_PASSWORD ?= postgres
POSTGRES_USER ?= postgres
POSTGRES_DB ?= postgres
POSTGRES_HOST_PORT ?= 5433
REDIS_HOST_PORT ?= 6379
BACKEND_PORT ?= 8080
METRICS_PORT ?= 9000
FRONTEND_PORT ?= 3000
GRAFANA_PORT ?= 3001

# Local stack defaults belong to local.env. Export only values explicitly
# supplied by the caller so a checked-out worktree can keep its own ignored
# ports and database credentials.
ifneq ($(filter command line environment,$(origin POSTGRES_USER)),)
export POSTGRES_USER
endif
ifneq ($(filter command line environment,$(origin POSTGRES_PASSWORD)),)
export POSTGRES_PASSWORD
endif
ifneq ($(filter command line environment,$(origin POSTGRES_DB)),)
export POSTGRES_DB
endif
ifneq ($(filter command line environment,$(origin POSTGRES_HOST_PORT)),)
export POSTGRES_HOST_PORT
endif
ifneq ($(filter command line environment,$(origin REDIS_HOST_PORT)),)
export REDIS_HOST_PORT
endif
ifneq ($(filter command line environment,$(origin BACKEND_PORT)),)
export BACKEND_PORT
endif
ifneq ($(filter command line environment,$(origin METRICS_PORT)),)
export METRICS_PORT
endif
ifneq ($(filter command line environment,$(origin FRONTEND_PORT)),)
export FRONTEND_PORT
endif
ifneq ($(filter command line environment,$(origin GRAFANA_PORT)),)
export GRAFANA_PORT
endif
TIMESCALE_DATABASE_URL ?= postgresql://$(POSTGRES_USER):$(POSTGRES_PASSWORD)@127.0.0.1:$(POSTGRES_HOST_PORT)/$(POSTGRES_DB)
DATABASE_URL ?= $(TIMESCALE_DATABASE_URL)

.PHONY: help supabase-up supabase-down render-local-config render-container-config bootstrap-local local-up local-up-ui local-down local-docker-up local-docker-up-ui local-docker-down local-reset local-doctor local-observability-up local-observability-down backend-db-up backend-db-down backend-db-check backend-migrate test-backend docker-dev-up docker-dev-down docker-local-build docker-local-up docker-local-down dev-up dev-down export-openapi

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
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" "$(LOCAL_STACK_SCRIPT)" optimizer

## Prepare local development dependencies
bootstrap-local: ## Prepare local development dependencies
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" "$(LOCAL_STACK_SCRIPT)" bootstrap

## Build frontend
build-frontend: ## Build frontend
	pnpm frontend:build:dev

## Build all
build: build-backend bootstrap-local build-frontend  ## Build all

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
	$(MAKE) bootstrap-local
	pnpm frontend:dev

## Start Supabase with config
supabase-up:  ## Start Supabase with config
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" "$(LOCAL_STACK_SCRIPT)" supabase-up

## Stop Supabase
supabase-down:  ## Stop Supabase
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" "$(LOCAL_STACK_SCRIPT)" supabase-down

## Start the TimescaleDB database used by backend development and tests
backend-db-up:  ## Start the backend database
	cd $(DCAPAL_BACKEND_DIR) && env POSTGRES_USER="$(POSTGRES_USER)" POSTGRES_PASSWORD="$(POSTGRES_PASSWORD)" POSTGRES_DB="$(POSTGRES_DB)" docker compose $(COMPOSE_TEST_ARGS) up -d --wait --wait-timeout 180 db

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
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_OBSERVABILITY_ARGS) --profile observability up -d --wait

## Stop development Docker containers
docker-dev-down:  ## Stop development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_OBSERVABILITY_ARGS) --profile observability down --remove-orphans

## Start development Docker containers with Dcapal image
docker-local-build:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_LOCAL_ARGS) build

## Start development Docker containers with Dcapal image
docker-local-up:  ## Start development Docker containers
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" up-docker

## Start development Docker containers with Dcapal image
docker-local-down:  ## Stop development Docker containers
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" down-docker

## Start full dev environment (Supabase + Docker)
dev-up: supabase-up docker-dev-up  ## Start full dev environment (Supabase + Docker)

## Stop full dev environment
dev-down: docker-dev-down supabase-down  ## Stop full dev environment
## Render host-mode backend configuration
render-local-config: supabase-up  ## Render host-mode backend configuration
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" render-host

## Render container-mode backend configuration
render-container-config: supabase-up  ## Render container-mode backend configuration
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" render-container

## Start the local backend with the host Rust process
local-up:  ## Start Supabase, core dependencies, and the host backend
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" up-host

## Start the host backend and frontend development server
local-up-ui:  ## Start the host backend and frontend development server
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" up-host-ui

## Stop the host backend environment
local-down:  ## Stop the host backend environment
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" down-host

## Start the local backend as a Docker image
local-docker-up:  ## Start Supabase, core dependencies, and the Docker backend
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" up-docker

## Start the Docker backend and frontend development server
local-docker-up-ui:  ## Start the Docker backend and frontend development server
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" up-docker-ui

## Stop the Docker backend environment
local-docker-down:  ## Stop the Docker backend environment
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" down-docker

## Remove local Compose volumes and containers
local-reset:  ## Remove local Compose volumes and containers
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" reset

## Check local development prerequisites
local-doctor:  ## Check local development prerequisites
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" doctor

## Start optional observability services
local-observability-up:  ## Start optional observability services
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" observability-up

## Stop optional observability services
local-observability-down:  ## Stop optional observability services
	LOCAL_ENV_FILE="$(LOCAL_ENV_FILE)" COMPOSE_PROJECT_NAME="$(COMPOSE_PROJECT_NAME)" "$(LOCAL_STACK_SCRIPT)" observability-down

## Merge all dependabot PRs
chore-merge-dependabot:  ## Merge all dependabot PRs
	script/merge-dependabot.sh dcapal/dcapal master
