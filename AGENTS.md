## Repository structure

- `dcapal-backend/` — Rust backend, migrations, provider adapters, and backend tests.
- `dcapal-frontend/` — frontend application, routes, UI, browser journeys, and frontend tests.
- `dcapal-optimizer-wasm/` — Rust/WASM allocation and rebalancing calculations.
- `packages/api-client/` — generated OpenAPI client models, query operations, and MSW handlers.
- `docs/` — shared ADRs, research, plans, and agent guidance.
- `CONTEXT-MAP.md` and each app's `CONTEXT.md` — context boundaries, glossary, and domain relationships.


## Agent skills

### Issue tracker

Issues and PRDs for this repo live as GitHub issues; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context layout: root `CONTEXT-MAP.md` with per-app `CONTEXT.md` files and ADRs. See `docs/agents/domain.md`.
