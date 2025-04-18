COMPOSE_BASE_ARGS := -f docker-compose.yml -f docker/docker-compose.dev.yml
DCAPAL_BACKEND_DIR := ./dcapal-backend
DCAPAL_OPTIMIZER_DIR := ./dcapal-optimizer-wasm
DCAPAL_FRONTEND_DIR := ./dcapal-frontend
SUPABASE_WORKDIR := ./config

.PHONY: help supabase-up supabase-down docker-dev-up docker-dev-down dev-up dev-down

## Show this help message
help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*##"}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## Format codebase
fmt:  ## Format codebase
	cargo +nightly fmt --all -- --config-path rustfmt.nightly.toml
	cd $(DCAPAL_FRONTEND_DIR) && npm run format

## Run Rust linters
lint-rust: ## Run Rust linters
	cargo +nightly fmt --all -- --config-path rustfmt.nightly.toml --check
	cargo clippy -- -D warnings

## Run JS linters
lint-js: ## Run JS linters
	cd $(DCAPAL_FRONTEND_DIR) && npm run check

## Run linters on the codebase
lint: lint-rust lint-js  ## Run linters on the codebase

## Build backend
build-backend: ## Build backend
	cd $(DCAPAL_BACKEND_DIR) && cargo build

## Build optimizer-wasm
build-optimizer: ## Build optimizer-wasm
	cd $(DCAPAL_OPTIMIZER_DIR) && wasm-pack build --dev

## Build frontend
build-frontend: ## Build frontend
	cd $(DCAPAL_FRONTEND_DIR) && npm i && npm run build-dev

## Build all
build: build-backend build-optimizer build-frontend  ## Build all

## Run backend (dev)
run-backend-dev: ## Run backend (dev)
	cd $(DCAPAL_BACKEND_DIR) && cargo run

## Run frontend (dev)
run-frontend-dev: ## Run frontend (dev)
	cd $(DCAPAL_OPTIMIZER_DIR) && wasm-pack build --dev
	cd $(DCAPAL_FRONTEND_DIR) && npm i && npm run start

## Start Supabase with config
supabase-up:  ## Start Supabase with config
	cd $(DCAPAL_BACKEND_DIR) && npx supabase start --workdir $(SUPABASE_WORKDIR)

## Stop Supabase
supabase-down:  ## Stop Supabase
	cd $(DCAPAL_BACKEND_DIR) && npx supabase stop --workdir $(SUPABASE_WORKDIR)

## Start development Docker containers
docker-dev-up:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_BASE_ARGS) up -d

## Stop development Docker containers
docker-dev-down:  ## Stop development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_BASE_ARGS) down

## Start development Docker containers with Dcapal image
docker-local-build:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_BASE_ARGS) -f docker/docker-compose.local.yml build

## Start development Docker containers with Dcapal image
docker-local-up:  ## Start development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_BASE_ARGS) -f docker/docker-compose.local.yml up -d

## Start development Docker containers with Dcapal image
docker-local-down:  ## Stop development Docker containers
	cd $(DCAPAL_BACKEND_DIR) && docker compose $(COMPOSE_BASE_ARGS) -f docker/docker-compose.local.yml down

## Start full dev environment (Supabase + Docker)
dev-up: supabase-up docker-dev-up  ## Start full dev environment (Supabase + Docker)

## Stop full dev environment
dev-down: docker-dev-down supabase-down  ## Stop full dev environment

## Start full dev+local environment (Supabase + Docker)
local-up: supabase-up docker-local-up  ## Start full dev environment (Supabase + Docker)

## Stop full dev+local environment
local-down: docker-local-down supabase-down  ## Stop full dev environment
