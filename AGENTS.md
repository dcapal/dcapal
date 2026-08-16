## Repository structure

- `dcapal-backend/` — Rust backend, migrations, provider adapters, and backend tests.
- `dcapal-frontend/` — frontend application, routes, UI, browser journeys, and frontend tests.
- `dcapal-optimizer-wasm/` — Rust/WASM allocation and rebalancing calculations.
- `packages/api-client/` — generated OpenAPI client models, query operations, and MSW handlers.
- `docs/` — shared ADRs, research, plans, and agent guidance.
- `CONTEXT-MAP.md` and each app's `CONTEXT.md` — context boundaries, glossary, and domain relationships.

## Domain docs

Multi-context layout: root `CONTEXT-MAP.md` with per-app `CONTEXT.md` files and ADRs. See `docs/agents/domain.md`.

## Project management

### Issue tracker

Issues and PRDs for this repo live as GitHub issues; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

## Code authoring

The following rules apply to all code in this repo. Nested `AGENTS.md` files extend the following with app-specific rules.

### Comments

- Always document public types, functions, constants. Document private helpers and code blocks when longer than 10 lines and in general when the intent is not immediately obvious.
- Comments must state the "What", not the "How". The "How" is stated in code. The "What" informs an unfamiliar reader about the intent of a piece of code, so they don't have to dive into the implementation details.

### Tests

- Prefer integration tests over unit tests except for pure business logic and calculations. Integration test interfaces are more stable and need less maintenance.
- Tests should always have documentation explaining the scenario in plain English and the expecations. Ideally, a reader shouldn't need to dive into the implementation to understand the scenario covered. They should follow the GIVEN, WHEN, THEN format.