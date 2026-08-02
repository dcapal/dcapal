# Migrate the DcaPal frontend API to a generated TanStack Query client

This ExecPlan is living. Keep its `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` sections current as work advances. Follow `/Users/leonardoarcari/.codex/PLANS.md` and the repository instructions in `AGENTS.md` when editing or executing this plan.

## Purpose / Big Picture

After this change, `dcapal-frontend` will use one workspace package, `@dcapal/api-client`, for backend REST operations. The package will be generated from the backend's OpenAPI document with Orval, expose TanStack Query v5 hooks and query options, and expose generated MSW handlers for tests. Frontend feature code will use TanStack Query for backend server state, while Redux will continue to hold local portfolio state and receive the market-price values that existing allocation code needs.

A contributor will be able to regenerate the client from the backend contract, start the frontend with the existing `/api` proxy, run the existing deterministic MSW-backed browser tests, and observe the same search, import, synchronization, and allocation flows. The migration must not change routes, WASM workers, allocation rules, or unrelated UI behavior.

## Progress

- [x] (2026-08-01) Inspected the workspace, frontend API call sites, backend OpenAPI generation, existing MSW fixtures, and the `asktobi` reference.
- [x] (2026-08-01) Completed the dependency-aware design interview and received confirmation of the shared understanding.
- [x] (2026-08-01) Recorded the generated-client architecture in ADR 002 and the decimal wire contract in ADR 003.
- [x] (2026-08-01) Marked ADR 001's temporary no-Orval decision as superseded without rewriting its historical context.
- [x] (2026-08-01) Corrected the backend REST serialization/schema mismatches and regenerated `dcapal-backend/docs/openapi.json`.
- [x] (2026-08-01) Added the `@dcapal/api-client` workspace package, Orval configuration, mutator, generated production output, generated MSW output, and package README.
- [x] (2026-08-01) Added TanStack Query to the frontend root and configured the query client and API transport at application startup.
- [x] (2026-08-01) Added the frontend price-provider adapter and migrated every backend REST consumer.
- [x] (2026-08-01) Composed generated MSW handlers with existing DcaPal fixtures and removed Axios-specific API tests.
- [x] (2026-08-01) Removed the obsolete Axios REST client and service wrappers after all consumers moved.
- [x] (2026-08-01) Ran contract, backend, package, frontend, build, and Chromium E2E verification; recorded the evidence below.

## Surprises & Discoveries

- Observation: The existing root ADR says not to add an Orval client, but the current request explicitly changes that direction.
  Evidence: `docs/adr/001-root-pnpm-workspace.md` contains the temporary no-Orval decision; ADR 002 now supersedes it.
- Observation: The `asktobi` reference uses one workspace package with two generated outputs, not two workspace packages.
  Evidence: `packages/api-client` exports `.`, `./mocks`, and `./model`, with `src/gen` and `src/gen-mocks` outputs.
- Observation: The backend OpenAPI document has no `servers` entry and the frontend's effective base path is `/api`.
  Evidence: `dcapal-frontend/src/app/config.js` defines `DCAPAL_API = "/api"`, and the development proxy removes that prefix before forwarding to the backend.
- Observation: The frontend currently combines Axios services, a direct `fetch` sync call, effects, timers, and request-id/cancel-token logic.
  Evidence: `dcapal-frontend/src/api/services`, `useFetchImportedPortfolio.js`, `useSyncPortfolios.js`, and the portfolio search/price components.
- Observation: Rust `Decimal` fields are currently described as strings by parts of OpenAPI but several response serializers force JSON floating-point numbers; the price timestamp is serialized as epoch seconds while its generated schema says date-time.
  Evidence: the `rust_decimal::serde::float*` attributes in `dcapal-backend/src/ports/inbound/rest`, `#[serde(with = "chrono::serde::ts_seconds")]` on `Price.ts`, and the generated `Price` schema.
- Observation: `MIGRATION.md` and `agent_docs/migration_inventory_report.md` are untracked stale work describing a different stack.
  Evidence: the files mention Zodios, Zustand, and Next.js. They are intentionally left untouched.
- Observation: Orval 8.23 expects MSW generation through `mock.generators`, and the imported portfolio schema's local `$defs` cannot remain rooted at the OpenAPI document.
  Evidence: generation failed with the older Orval mock syntax and backend validation exposed unresolved `#/$defs/...` references; the configuration and backend exporter now handle both current constraints.
- Observation: The browser test environment can block Chromium bootstrap inside the sandbox, while the same suite passes with the required elevated browser launch permission.
  Evidence: the Chromium Playwright project passed all seven tests on the escalated retry; the full Firefox/WebKit matrix was not required to validate this migration.

## Decision Log

- Decision: Use one `@dcapal/api-client` workspace package with `./mocks` and `./model` subpaths.
  Rationale: It matches the reference layout, keeps one public import boundary, and avoids a second runtime client package.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Generate both the production React Query client and MSW handlers from the backend OpenAPI artifact.
  Rationale: One contract drives production typing and test interception; generated output is committed and regeneration is explicit.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Use a native `fetch` mutator with `/api` base URL resolution, frontend-provided access-token callbacks, one optional 401 refresh retry, and normalized thrown errors.
  Rationale: It matches the reference, supports `AbortSignal`, centralizes transport behavior, and keeps Supabase out of the shared package.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Use TanStack Query only for server state; leave local UI and portfolio state in its current Redux model.
  Rationale: The migration changes request lifecycle and caching without expanding into a state-management rewrite.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Keep Rust `Decimal` REST values as strings and convert them only at the existing JavaScript calculation boundary.
  Rationale: JSON numbers cannot preserve arbitrary decimal precision in JavaScript, while a full decimal arithmetic migration is outside scope.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Keep feature-specific price-provider composition in the frontend as a price-provider adapter.
  Rationale: The shared package remains a generated contract and transport boundary; Yahoo parsing and DcaPal provider shapes are application behavior.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Preserve current search, import, synchronization, and allocation behavior while moving request state to TanStack Query.
  Rationale: The request is an API/client architecture migration, not a product-flow redesign.
  Date/Author: 2026-08-01 / user and Codex.

## Outcomes & Retrospective

Completed on 2026-08-01. The frontend now uses `@dcapal/api-client` for all backend REST operations. Orval generates both the React Query client and MSW handlers from the checked-in backend OpenAPI artifact. The native-fetch mutator centralizes `/api` resolution, Supabase token callbacks, one guarded refresh retry, cancellation, and normalized errors. Redux remains the local portfolio-state store; TanStack Query owns server state and request lifecycle.

The existing fixture-backed browser flows passed without route or allocation changes. Decimal-backed REST values are strings at the wire boundary and become JavaScript numbers only where the existing calculation model requires them. The remaining operational requirement is to rerun the explicit generation command whenever the backend contract changes; generated files are committed so contract changes are reviewable.

## Context and Orientation

The repository is a root pnpm workspace. The root package is `dcapal`; its workspace members are `dcapal-frontend` (`@dcapal/frontend`) and `packages/api-client` (`@dcapal/api-client`). The backend lives in `dcapal-backend` and generates `dcapal-backend/docs/openapi.json` from Rust `utoipa` annotations. The backend exposes the document at `/api-doc/openapi.json` and CI already verifies the checked-in generated artifact.

The current frontend REST boundary is `dcapal-frontend/src/api`. `httpClient.ts` creates an Axios instance. `services/providers.ts` reads DcaPal asset lists, prices, and Yahoo chart data; `services/assetsSearch.ts` reads Yahoo search data; `services/importPortfolio.ts` reads temporary imported portfolios; and `services/syncPortfolios.ts` posts authenticated portfolio synchronization data. Existing consumers are in the allocation app, portfolio search, import route, price refresh flow, portfolio Redux thunk, and synchronization hook.

TanStack Query is the library that stores and refreshes asynchronous data whose source of truth is outside the browser. Orval is the generator that turns OpenAPI operations into TypeScript request functions and TanStack Query hooks. MSW is the request-interception library already used by DcaPal's browser tests. A mutator is the shared function generated operations call to perform HTTP requests; it will resolve `/api`, attach the current access token, support cancellation, normalize errors, and perform one optional authentication retry.

The backend contract includes asset catalog operations, Yahoo search and chart pass-through operations, conversion prices, imported portfolios, and authenticated portfolio synchronization. All operations in the document will be generated, even if the current frontend has no caller for some of them. Supabase authentication calls are not part of this package; the frontend supplies token and refresh callbacks to the package.

## Plan of Work

First correct the backend contract representation. Keep Rust `Decimal` values exact and serialize them as strings in REST responses; keep request schemas as strings so generated request types require explicit conversion from the frontend's existing numbers. Replace the misleading float serializers in fee and portfolio response fields with string serializers. Annotate `Price.ts` as an epoch-seconds integer in OpenAPI while preserving its existing `ts_seconds` wire encoding. Regenerate the checked-in OpenAPI file and add focused backend tests for the serialized values and schema shape.

Next create `packages/api-client` as a private TypeScript workspace package named `@dcapal/api-client`. Its Orval configuration will read `../../dcapal-backend/docs/openapi.json`, generate React Query v5 operations under `src/gen`, generate MSW handlers under `src/gen-mocks`, and expose the real client, mock output, and models through `package.json` exports. Its `src/mutator/api-fetch.ts` will provide base-URL configuration, access-token and refresh callbacks, one guarded 401 retry, response parsing, and a normalized error. Generated output will be committed. The package README will document the generation command, public imports, frontend transport setup, and MSW composition.

Add the package and `@tanstack/react-query` to the workspace catalog and frontend dependencies. Add one `QueryClient` and `QueryClientProvider` at the frontend root with retries and focus refetching disabled. Configure the API mutator from frontend startup with the `/api` base path and callbacks backed by Supabase session APIs; do not import Supabase into the shared package.

Migrate consumers to generated operations. Use generated hooks or query options directly for all spec-covered operations. Add a frontend price-provider adapter for the Yahoo chart plus conversion-price composition, asset-shape mapping, response validation, and imperative `queryClient.fetchQuery` use in import and price-refresh flows. Use five-minute price freshness, session-fresh asset catalogs, and short-lived search caching. Keep 300 ms search debounce, Fuse filtering, current-result protection, and existing fallback values. Use a single synchronization coordinator near `Router`, expose manual `syncNow` through a small context, and use a generated sync mutation whose success data continues to update Redux.

Update the existing MSW setup to register application-specific fixture handlers before generated handlers. Keep current deterministic fixture behavior and add generated handlers as defaults for operations not covered by fixtures. Replace Axios-specific unit mocks with package/adapter tests that exercise generated query options and the shared transport through MSW. Remove the obsolete Axios client, direct fetch sync implementation, old service wrappers, and compatibility exports only after all feature consumers have moved.

Finally update the root and package scripts so one explicit command regenerates the backend OpenAPI document first and then the Orval outputs. Run the backend contract tests, package tests, frontend type check, unit tests, production build, and existing E2E suite. Record exact evidence and any changes from this plan in the living sections below.

## Concrete Steps

Run all commands from the repository root unless a step says otherwise.

1. Inspect and update the Rust REST serializers and OpenAPI annotations in `dcapal-backend/src/ports/inbound/rest/mod.rs`, `request.rs`, `response.rs`, and any related schema tests. Run the backend formatter and tests. Use the repository's existing OpenAPI export command and confirm only the intended generated contract changes appear.

2. Create the `packages/api-client` package and add its workspace entry. Add the package-local `generate` script and a root script that exports the backend OpenAPI document before running Orval. Install or add the pinned workspace versions of `@tanstack/react-query`, `orval`, and any MSW mock-generation development dependencies through the pnpm catalog; update the single root lockfile.

3. Run the package generation command twice. The second run must be idempotent: it should not produce a new diff in `dcapal-backend/docs/openapi.json`, `packages/api-client/src/gen`, or `packages/api-client/src/gen-mocks`. This was verified after the final contract was generated.

4. Add the root query provider and API mutator configuration. Verify that a request made through a generated query option resolves `/api`, carries the current bearer token when available, and aborts when TanStack Query cancels it.

5. Migrate one vertical slice at a time: asset catalog and currency loading; imported portfolio route; DcaPal and Yahoo search; per-result and portfolio price refresh; then authenticated portfolio synchronization. After each slice, run the relevant frontend unit tests and inspect the diff for direct backend network calls outside the package or price-provider adapter.

6. Compose generated MSW handlers with the existing fixture handlers. Run the browser tests with `REACT_APP_E2E_MSW=1` through the repository's existing Playwright command and ensure unhandled `/api/` requests still fail loudly.

7. Remove obsolete API files and Axios-specific dependencies after `rg` confirms no frontend feature code imports them. Update the package README, ADRs, and this plan with final paths and evidence.

## Validation and Acceptance

The backend OpenAPI export must be reproducible. Running the export command followed by `git diff --exit-code dcapal-backend/docs/openapi.json` must succeed after the committed contract is current. The generated production and mock directories must also be clean after a second generation run.

The API package must have one production import surface, `@dcapal/api-client`, plus `@dcapal/api-client/mocks` and `@dcapal/api-client/model`. A package smoke test must fetch at least one generated query through a `QueryClient` and MSW, and a mutator test must cover base URL resolution, bearer-token injection, a successful response, an API error, cancellation, and one guarded 401 retry.

The frontend must pass its type check, unit tests, production build, and existing Playwright E2E tests. The E2E suite must continue to cover the existing route smoke flows, including allocation, import, login, and demo routes. Search must still debounce input, display the same asset classes, and load provider prices. Import must still navigate to allocation on a valid fixture and back to the portfolios step for missing data. Authenticated synchronization must still run immediately and every five seconds through one coordinator, while manual portfolio edits still trigger synchronization. Existing allocation and WASM worker behavior must remain unchanged.

Use repository-appropriate commands, expected to include:

    pnpm install --frozen-lockfile
    pnpm --filter @dcapal/api-client generate
    pnpm frontend:typecheck
    pnpm frontend:test
    pnpm frontend:build
    pnpm frontend:test:e2e

Run the backend formatter/tests and OpenAPI verification using the existing Rust workspace commands documented by the repository. If a command name differs from the current checkout, update this plan with the exact command before continuing; do not hide a missing validation step.

Verification evidence (2026-08-01):

    cargo fmt --all -- --check
    cargo test -p dcapal-backend -- --nocapture       # 9 passed
    pnpm install --frozen-lockfile
    pnpm --filter @dcapal/api-client typecheck
    pnpm --filter @dcapal/api-client test                # 5 passed
    pnpm --filter @dcapal/frontend typecheck
    pnpm --filter @dcapal/frontend test                  # 6 passed
    pnpm --filter @dcapal/frontend lint
    pnpm --filter @dcapal/frontend build
    pnpm --filter @dcapal/frontend exec playwright test --project=chromium  # 7 passed

The final `make export-openapi && pnpm --filter @dcapal/api-client generate` run completed successfully. A before/after checksum over both generated trees remained unchanged on the second run. `rg` found no Axios imports, obsolete API-service imports, or direct frontend backend `fetch` calls outside the intended MSW/test boundaries. The browser suite was verified with Chromium; Firefox and WebKit were not run because they were outside the required migration check.

## Idempotence and Recovery

OpenAPI and Orval generation are safe to repeat. If generated output changes unexpectedly, inspect the OpenAPI diff first and do not hand-edit generated files. Correct the Rust annotations or Orval configuration, regenerate, and rerun the contract checks.

Do not delete the stale untracked `MIGRATION.md` or `agent_docs/migration_inventory_report.md`. They are outside this migration's documentation scope. Before removing the old frontend API files, confirm all imports with `rg` and preserve any unrelated user changes. If a slice cannot be migrated without changing product behavior, keep the smallest feature adapter needed and record the reason in `Surprises & Discoveries` rather than weakening the shared contract.

## Artifacts and Notes

The durable architecture decisions are in `docs/adr/002-openapi-tanstack-api-client.md` and `docs/adr/003-decimal-strings-on-rest-wire.md`. The backend contract artifact is `dcapal-backend/docs/openapi.json`. The intended generated package layout is:

    packages/api-client/
      package.json
      orval.config.ts
      README.md
      src/index.ts
      src/mutator/api-fetch.ts
      src/gen/                 # committed Orval production output
      src/gen-mocks/           # committed Orval MSW output

Keep short command transcripts and focused diffs here as implementation produces them. Do not paste large generated files into this plan.

## Interfaces and Dependencies

The API package must export the generated React Query v5 operations and models, plus the following transport configuration surface from `src/index.ts` or its mutator module:

    configureApiClientBaseUrl(baseUrl: string | undefined): void
    configureApiClientAuth(hooks: ApiClientAuthHooks | null): void
    resetApiClientConfiguration(): void

`ApiClientAuthHooks` must allow asynchronous access-token retrieval, optional asynchronous refresh, and an optional authentication-failure callback. The mutator must accept the `AbortSignal` provided by generated query functions and throw a normalized error rather than returning an empty successful result.

The frontend price-provider adapter must keep the existing provider distinction: DcaPal assets use the generated conversion-price operation; Yahoo assets use the generated chart operation and, when needed, the generated conversion-price operation. It must expose a React query path for visible search results and an imperative path for portfolio/import refresh actions. It may return the existing `BAD_DATA`, cancellation, and nullable outcomes at the feature boundary.

The frontend must depend directly on `@tanstack/react-query` and `@dcapal/api-client`. The API package may declare TanStack Query as a peer dependency and Orval/MSW as development dependencies. The backend remains the source of the OpenAPI artifact; the frontend package must not maintain an independently edited contract copy.

Revision note (2026-08-01): Created after the confirmed batch-grilling session. The plan records the one-package `asktobi` layout, generated MSW output, price-provider adapter boundary, decimal-string wire contract, preserved frontend behavior, and the deliberate decision to ignore stale untracked migration notes.
