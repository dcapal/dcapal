# Convert new and heavily changed frontend source to TypeScript

This ExecPlan is living. Keep `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current while implementing it. Follow `/Users/leonardoarcari/.codex/PLANS.md`, `AGENTS.md`, and the repository's frontend instructions.

## Purpose / Big Picture

The frontend migration should not leave newly written application or browser-test modules in JavaScript while the surrounding code moves to TypeScript. After this change, every new frontend source module in this branch will use a TypeScript extension, and any existing JavaScript module whose branch diff replaces more than 80% of its original content will also use TypeScript. The application and its real Playwright/MSW journeys must behave exactly as they do now.

The user-visible proof is unchanged behavior: the development build still starts, the allocation/import/search/synchronization journeys still pass in Chromium, Firefox, and WebKit, and frontend typecheck succeeds with the converted runtime modules included.

## Progress

- [x] (2026-08-02) Audited `origin/master...HEAD` and identified added frontend JavaScript files and existing JavaScript files whose changed-line ratio exceeds 80% of the original file.
- [x] (2026-08-02) Chosen scope: convert frontend runtime, MSW, Playwright journey, and Playwright support modules; retain Node-only coverage scripts and build configuration as JavaScript/ESM tooling.
- [x] (2026-08-02) Renamed the selected files, using `.tsx` for the two converted JSX modules, and preserved extensionless imports.
- [x] (2026-08-02) Added TypeScript types for the generated API boundaries, Redux state, MSW fixtures, and Playwright helpers; added React declarations and included the test tree in frontend typecheck.
- [x] (2026-08-02) Validated formatting, typecheck, unit tests, build, coverage browser tests, compatibility browser tests, and changed-line coverage.
- [x] Commit, push, and verify the GitHub pipeline.

## Surprises & Discoveries

- Observation: The branch contains two newly added production JavaScript modules, `dcapal-frontend/src/api/priceProviders.js` and `dcapal-frontend/src/api/queryClient.js`.
  Evidence: both are added files in `git diff --name-status origin/master...HEAD`.
- Observation: The existing files above the 80% threshold are `searchBar.js`, `useFetchImportedPortfolio.js`, `useSyncPortfolios.js`, and `mocks/handlers.js`; the existing journey files `pfolioCurrencies.spec.js`, `routes.smoke.spec.js`, and `syncPortfolios.spec.js` also exceed it.
  Evidence: the audit compares `(added lines + deleted lines) / original line count`; the ratios are 136.9%, 122.0%, 198.0%, 191.9%, 141.7%, 131.0%, and 232.6% respectively.
- Observation: `dcapal-frontend/tsconfig.json` typechecks only `src/**/*`, while Playwright transpiles files under `tests/` directly.
  Evidence: the current `include` contains only `src/**/*`, and `playwright.config.js` runs the `tests` directory.
- Observation: Webpack already accepts `.ts`, `.tsx`, `.js`, and `.jsx`, and Babel already includes `@babel/preset-typescript`.
  Evidence: `dcapal-frontend/webpack.common.js` defines those extensions and the TypeScript Babel preset.
- Observation: Tailwind scanned only JavaScript and JSX files, so utilities used by the newly renamed `.tsx` search component were missing from the compiled bundle.
  Evidence: the fresh browser run showed the dropdown utility resolving to an incorrect computed position until `tailwind.config.js` included `ts` and `tsx` content globs.
- Observation: a local Playwright run reused a stale webpack server after source edits, which made the browser test appear unchanged until the server was explicitly stopped and a fresh CI-mode server was started.
  Evidence: the old process was listening on port 3000; fresh-server runs loaded the corrected TSX bundle and passed.

## Decision Log

- Decision: Convert every added frontend `.js` module under `tests/` as well as under `src/`, because browser journeys and their support modules are frontend source written in this branch.
  Rationale: the review requirement is broad, and keeping newly authored test modules in JavaScript would leave the migration visibly inconsistent.
  Date/Author: 2026-08-02 / user and Codex.
- Decision: Use `.tsx` for `searchBar` because it contains JSX; use `.ts` for non-JSX modules and test files.
  Rationale: TypeScript uses `.tsx` for files containing JSX and `.ts` for ordinary modules.
  Date/Author: 2026-08-02 / Codex.
- Decision: Do not rename `dcapal-frontend/scripts/coverage-report.mjs`, `playwright.config.js`, or Webpack configuration files.
  Rationale: they are Node/build/test tooling rather than frontend application source, and renaming the coverage script would require adding a TypeScript runtime loader without improving the browser code. The user’s 80% rule also does not select the existing configuration files.
  Date/Author: 2026-08-02 / Codex.
- Decision: Preserve runtime behavior and API boundaries; add types around existing generated Orval models, Redux state, MSW request handlers, and Playwright fixtures without redesigning them.
  Rationale: this is a language migration, so behavior changes would make failures harder to attribute.
  Date/Author: 2026-08-02 / Codex.
- Decision: Add `@types/react` and `@types/react-dom`, include `tests/**/*` in the frontend TypeScript project, and map the generated API-client `mocks` subpath in `tsconfig.json`.
  Rationale: the existing JavaScript build did not need React declarations or TypeScript resolution for the Playwright/MSW tree, but the converted source must compile without ambient `any` module errors.
  Date/Author: 2026-08-02 / Codex.
- Decision: Expand Tailwind content globs to include `.ts` and `.tsx`, and use an explicit `top-[3rem]` utility for the search dropdown.
  Rationale: the migrated TSX component must retain its utility CSS, and the real browser journey exposed an overlap when the class was not generated.
  Date/Author: 2026-08-02 / Codex.

## Outcomes & Retrospective

The selected new and heavily changed frontend modules are now TypeScript/TSX. Tooling remains JavaScript/ESM by decision: `scripts/coverage-report.mjs`, `playwright.config.js`, and Webpack configuration files were not selected by the 80% rule. Frontend unit tests, the development build, frontend and API-client typechecks, API-client tests, both 76-test browser lanes, and changed-line coverage pass. The coverage report records 320/320 changed executable lines covered and does not enforce a global percentage threshold. The durable language policy is recorded in [ADR 005](../adr/005-frontend-source-uses-typescript.md).

## Context and Orientation

The repository root is a pnpm workspace. The browser application is `dcapal-frontend`. Webpack builds its `src/` tree, Babel strips TypeScript syntax, and the generated package `packages/api-client` provides typed Orval hooks and REST models. MSW handlers in `dcapal-frontend/src/mocks/handlers` provide deterministic backend responses to browser journeys. Playwright discovers the browser tests under `dcapal-frontend/tests`.

The selected production files are:

- `dcapal-frontend/src/api/priceProviders.js` to `priceProviders.ts`.
- `dcapal-frontend/src/api/queryClient.js` to `queryClient.ts`.
- `dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.js` to `searchBar.tsx`.
- `dcapal-frontend/src/hooks/useFetchImportedPortfolio.js` to `useFetchImportedPortfolio.ts`.
- `dcapal-frontend/src/hooks/useSyncPortfolios.js` to `useSyncPortfolios.tsx`.
- `dcapal-frontend/src/mocks/handlers.js` to `handlers.ts`.

The selected new or heavily changed browser-test files are:

- `tests/journeys/create-portfolio.spec.js`, `edit-portfolio.spec.js`, `import-portfolio.spec.js`, `manage-portfolios.spec.js`, `search-assets.spec.js`, and `transaction-fees.spec.js` to `.spec.ts`.
- `tests/support/auth.js`, `coverage.js`, `fixtures.js`, `scenarios.js`, and `state.js` to `.ts`.
- `tests/pfolioCurrencies.spec.js`, `routes.smoke.spec.js`, and `syncPortfolios.spec.js` to `.spec.ts`.

`dcapal-frontend/src/api/portfolioSync.ts` is already TypeScript from the preceding review follow-up. Existing extensionless imports should continue to resolve after the rename because Webpack and TypeScript search the configured extensions.

## Plan of Work

First rename the selected files with `git mv`, choosing `.tsx` only for `searchBar`. Update no import strings unless a path explicitly includes an extension; the application imports these modules without extensions. Update any package script or Playwright glob that names a `.js` file explicitly.

Next type the two API modules. `priceProviders.ts` will use generated API response/model types and TanStack Query callback types, while preserving the existing provider and error values. `queryClient.ts` will retain its constants and `QueryClient` options. Type `searchBar.tsx` around its props, search result shapes, Redux selectors, generated hook data, and callback events. Type the two synchronization hooks around Redux state, React context, and generated mutation/query results. Type `handlers.ts` around MSW request parameters and fixture data, using narrow local types where generated response types do not describe malformed/error fixture variants.

Finally convert the Playwright support and journey modules. Use Playwright’s existing fixture types for `page`, `testInfo`, and the extended fixture options. Type helper inputs and returned fixture state, but keep the GIVEN/WHEN/THEN comments and user-journey behavior unchanged. Do not add direct application fetches or frontend module mocks.

## Concrete Steps

Run from the repository root:

    git diff --name-status --find-renames=80% origin/master...HEAD
    git diff --check

Rename the selected files, then run formatting and checks from the root:

    pnpm install --frozen-lockfile
    pnpm frontend:lint
    pnpm frontend:typecheck
    pnpm frontend:test
    pnpm frontend:build:dev
    pnpm frontend:test:e2e:coverage
    pnpm frontend:test:e2e --project=firefox --project=webkit
    pnpm --filter @dcapal/api-client typecheck
    pnpm --filter @dcapal/api-client test
    git diff --check

The expected browser outcomes are 76 passed coverage journeys across desktop/mobile Chromium and 76 passed compatibility journeys across Firefox/WebKit. The generated coverage report shows 320/320 changed executable lines covered when run against the branch diff, without a global percentage threshold.

## Validation and Acceptance

Acceptance requires that no added frontend application or browser-test JavaScript file remains in the selected scope, and no existing JavaScript file with a branch diff ratio above 80% remains in that scope. `portfolioSync.ts` and all converted modules must be included in the frontend typecheck or Playwright run that executes them.

The development build must compile the renamed `.ts` and `.tsx` modules. The unit tests, API-client tests, 76 Chromium coverage journeys, and 76 Firefox/WebKit journeys must pass. The final GitHub run must report green Chrome and other-browser matrix jobs, and the existing unrelated untracked files `MIGRATION.md` and `agent_docs/` must remain untouched.

## Idempotence and Recovery

Renames are explicit and reversible with `git mv` before commit. If a type error appears, fix the type at the module boundary rather than weakening the repository’s strict TypeScript settings or changing runtime behavior. Do not reset the worktree, delete unrelated files, or stage `MIGRATION.md` or `agent_docs/`. Generated coverage and Playwright output may be removed only from their explicit ignored directories when rerunning checks.

## Artifacts and Notes

The final diff should show `.js` to `.ts`/`.tsx` renames for the selected files rather than unrelated rewrites. The living plan should end with a short summary such as:

    Converted selected new and >80%-changed frontend modules to TypeScript; tooling configuration remains JavaScript/ESM by decision. Frontend typecheck, build, unit tests, browser lanes, and GitHub checks pass.

## Interfaces and Dependencies

The converted modules continue to use the existing interfaces:

- `priceProviders.ts` exports `Provider`, `FetchError`, `fetchDcaPalPrice`, `fetchYahooPrice`, `getYahooPriceQueryKey`, `useYahooPrice`, `getDcaPalPrice`, `getYahooPrice`, and `getPriceForProvider`.
- `queryClient.ts` exports `SESSION_STALE_TIME`, `SEARCH_STALE_TIME`, `PRICE_STALE_TIME`, and `queryClient`.
- `searchBar.tsx` exports `SearchBar` with the existing `text`, `setText`, and `addAsset` behavior.
- `useFetchImportedPortfolio.ts` exports `useFetchImportedPortfolio` with the existing `{ portfolio, isLoading, isError }` result.
- `useSyncPortfolios.tsx` exports `SyncCoordinator` and `useSyncPortfolios` with the existing context value and generated mutation boundary.
- `handlers.ts` exports the MSW `handlers` array and preserves all existing scenario routes.

The only new dependencies are development-time React declaration packages (`@types/react` and `@types/react-dom`). The existing TypeScript compiler, Babel TypeScript preset, Webpack extensions, Playwright runner, generated API models, and pnpm scripts remain the runtime dependency surface.

Revision note (2026-08-02): Created after auditing the full branch diff in response to the request to convert all new frontend source and any existing JavaScript changed by more than 80%.
Revision note (2026-08-02): Updated after implementation and validation; recorded the TSX Tailwind scan fix, fresh-server browser validation, final coverage result, and converted `.tsx` hook path.
Revision note (2026-08-02): Recorded the durable TypeScript source policy in ADR 005 and completed the final commit, push, and pipeline verification item.
