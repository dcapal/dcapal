COMPOSE_BASE_ARGS := -f docker-compose.yml -f docker/docker-compose.dev.yml
DCAPAL_BACKEND_DIR := ./dcapal-backend
SUPABASE_WORKDIR := ./config

.PHONY: help supabase-up supabase-down docker-dev-up docker-dev-down dev-up dev-down

## Show this help message
help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*##"}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

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
