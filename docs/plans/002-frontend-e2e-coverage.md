# Add real frontend user-journey coverage and GitHub changed-line reporting

This ExecPlan is living. Keep its `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` sections current as work advances. Follow `/Users/leonardoarcari/.codex/PLANS.md` and the repository instructions in `AGENTS.md` when implementing it.

## Purpose / Big Picture

After this change, the frontend API migration in this branch will be protected by real browser journeys. A contributor will be able to run the DcaPal application in a real Chromium browser, use the same React components, Redux state, TanStack Query hooks, generated Orval client, Supabase client, and routing that a user uses, and have MSW intercept only the backend boundary with deterministic data.

The journeys will cover the changed frontend behavior for portfolio creation, import, asset search, prices, fees, weights, persistence, and authenticated synchronization. Focused MSW-backed package tests will cover API transport branches that have no honest user journey. GitHub Actions will publish one combined coverage report containing HTML, LCOV, JSON, a text summary, and a changed-line report. The report will not fail the job for a coverage percentage, but every changed in-scope executable line must be covered before handoff.

The demo portfolio route is explicitly out of scope. The optimizer implementation is also out of scope because it was not changed in this branch; the first-time allocation journey stops when the user reaches the investment step.

## Progress

- [x] (2026-08-01) Inspected the branch, existing frontend Playwright setup, MSW handlers, API-client transport, CI workflow, and changed frontend files.
- [x] (2026-08-01) Completed the batch design interview and confirmed the shared understanding with the user.
- [x] (2026-08-01) Accepted ADR 004, which records the real-browser/MSW boundary and changed-line coverage policy.
- [x] (2026-08-01) Agreed the user-journey catalog, selector policy, deterministic clock policy, transport test matrix, and GitHub report shape.
- [x] (2026-08-02) Added deterministic MSW scenario helpers, authentication fixtures, persisted-state helpers, and stable selectors.
- [x] (2026-08-02) Rewrote the existing smoke, currency, and synchronization tests as real user journeys.
- [x] (2026-08-02) Added success, loading, empty, malformed, HTTP-error, fee, weight, persistence, import, search, and synchronization journeys.
- [x] (2026-08-02) Expanded handwritten API transport tests with MSW-backed edge cases.
- [x] (2026-08-02) Collected and merged browser V8 coverage with API-client Vitest coverage.
- [x] (2026-08-02) Added changed-line reporting and the non-blocking GitHub coverage job.
- [x] (2026-08-02) Ran the complete local verification matrix and inspected every changed in-scope executable line.
- [x] (2026-08-02) Reviewed the PR feedback and recorded the requirement that new frontend runtime source files use TypeScript.
- [x] (2026-08-02) Applied the PR feedback by converting the new frontend serializer to TypeScript, hardened both retried E2E journeys, and made the combined CI matrix green locally.

## Surprises & Discoveries

- Observation: The frontend already starts an MSW browser worker when `REACT_APP_E2E_MSW=1`, and the existing Playwright server command enables that flag.
  Evidence: `dcapal-frontend/src/index.js`, `dcapal-frontend/src/mocks/browser.js`, and `dcapal-frontend/playwright.config.js`.
- Observation: Existing browser tests include a direct `page.evaluate(fetch)` sync test instead of a user journey.
  Evidence: `dcapal-frontend/tests/syncPortfolios.spec.js` posts directly to `/api/v1/sync/portfolios`; this plan replaces it with visible portfolio actions and boundary assertions.
- Observation: The branch uses Webpack with inline source maps for the Playwright development server.
  Evidence: `dcapal-frontend/webpack.dev.js` sets `devtool: "inline-source-map"`.
- Observation: Playwright's native JavaScript coverage API is Chromium-only, which matches the agreed coverage job. It returns V8 ranges that can be converted to Istanbul coverage; no Babel instrumentation is needed.
  Evidence: the installed Playwright version exposes `page.coverage.startJSCoverage()` and `stopJSCoverage()`, and the official API describes the V8-to-Istanbul conversion path.
- Observation: Playwright's browser clock can advance timers without waiting in wall-clock time.
  Evidence: the installed Playwright version exposes `page.clock`; the price-refresh and synchronization timers therefore do not require five-minute or five-second waits.
- Observation: The branch contains large generated Orval production and MSW trees.
  Evidence: `packages/api-client/src/gen` and `packages/api-client/src/gen-mocks`. They remain exercised through application requests but are excluded from the handwritten changed-line denominator.
- Observation: The existing frontend context already defines the domain terms needed by the journeys, including portfolio import and portfolio synchronization.
  Evidence: `dcapal-frontend/CONTEXT.md`. No general testing terms were added to the domain glossary.
- Observation: Importing a portfolio under React StrictMode could start the same import twice and briefly clear the selected file, which made a real browser journey flaky.
  Evidence: the import flow now guards the request with a ref, waits for the currency catalog, and clears the file only after a terminal result or explicit back action.
- Observation: Firefox could race the development server's vendor chunk while reloading persisted state in the same context.
  Evidence: the persistence journey reopens the app in a fresh context seeded from the previous storage state, and MSW E2E mode disables Webpack HMR/live reload.
- Observation: The local default sandbox could not reliably launch repeated Chromium coverage processes after the manual browser smoke check.
  Evidence: the unprivileged attempt produced macOS `MachPortRendezvous` permission failures and `EMFILE` watcher errors; the clean 76-test coverage run passed with elevated process access.
- Observation: The first CI coverage run passed all 76 browser tests and all 15 API-client tests, then failed while finding `HEAD^` in the shallow PR merge checkout. The fallback Istanbul HTML report also contained CSS source-map filenames with `?` characters, which GitHub artifact upload rejects.
  Evidence: Actions run `30723868878`, job `91432090385`; the coverage report failed at `git diff --unified=0 HEAD^`, and artifact upload rejected `spinner.css?...html`.
- Observation: The compatibility job reported two retried tests: successful portfolio import in Chromium and quantity/price/weight editing in WebKit.
  Evidence: Actions run `30723868878`, job `91432090373`; both passed on retry, but the first assertion observed the transient import route and the second observed the previous weight warning state.
- Observation: The PR review requires new frontend runtime source files to be TypeScript as the migration continues.
  Evidence: unresolved review thread `PRRT_kwDOIeNHNM6VpwiF` on `dcapal-frontend/src/api/portfolioSync.js`.
- Observation: The first local coverage report reused a stale webpack dev server and therefore mapped the serializer to the deleted JavaScript path.
  Evidence: the existing process on port 3000 served `portfolioSync.js`; after restarting it, fresh V8 fragments mapped the runtime to `portfolioSync.ts`.

## Decision Log

- Decision: Use real Playwright browser journeys for frontend behavior and MSW's browser worker as the only backend boundary.
  Rationale: The user wants realistic frontend tests. This exercises the real React tree, router, Redux store, TanStack Query cache, generated client, and request lifecycle without a live backend.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Keep focused MSW-backed tests for handwritten API transport edge cases.
  Rationale: Raw response parsing, 204/205/304 handling, abort propagation, and guarded authentication retries do not all have honest user journeys. Testing the transport with real `fetch` and MSW covers those branches without mocking frontend modules.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Require coverage of changed in-scope executable lines/statements, while publishing function and branch totals as diagnostics.
  Rationale: The user wants every new line checked but does not want a global GitHub threshold. Some environment and defensive branches are not meaningful user journeys; reporting them is useful, pretending they are a merge gate is not.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Use native Playwright V8 coverage and merge it with Vitest V8 coverage instead of Babel/Istanbul browser instrumentation.
  Rationale: Chromium already exposes the coverage data needed for the agreed coverage job, and native collection leaves normal Webpack bundles unchanged.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Use one two-lane E2E matrix job: a Chrome lane runs desktop/mobile coverage and the other-browsers lane runs Firefox/WebKit compatibility checks.
  Rationale: The coverage desktop project is the same Chromium user journey as the compatibility project, so running it once avoids duplicate Chrome work. Firefox and WebKit still run sequentially under the CI worker limit, while the two lanes can run in parallel.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Select MSW data with an `x-e2e-scenario` request header set by a Playwright fixture.
  Rationale: Each test can choose a deterministic backend scenario without adding a production test route, exposing a frontend test API, or intercepting requests in Playwright.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Organize tests around user journeys and start every journey spec with a clear GIVEN/WHEN/THEN comment.
  Rationale: The suite should explain the user outcome before exposing implementation details, and separate failure outcomes should remain readable and independently diagnosable.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Do not include demo portfolio journeys.
  Rationale: The user explicitly excluded them from this coverage effort.
  Date/Author: 2026-08-01 / user and Codex.
- Decision: Do not add a global coverage gate, PR-comment bot, or external coverage service.
  Rationale: GitHub Actions job summaries and downloadable artifacts are enough to inspect changed-line coverage without adding noisy or costly external infrastructure.
  Date/Author: 2026-08-01 / user and Codex.

## Outcomes & Retrospective

Implemented the plan with 38 user-journey tests. The two-lane CI matrix runs the journeys in desktop/mobile Chromium for coverage and in Firefox/WebKit for compatibility; the demo portfolio scenario remains excluded.

The API-client transport suite has 15 MSW-backed tests. The combined report includes browser and API-client V8 coverage in `coverage/frontend/`, including HTML, LCOV, JSON, text, and `changed-lines.md`. The final report shows 745/745 changed in-scope executable lines covered. It reports global branch/function totals for diagnostics but enforces no global percentage threshold.

GitHub Actions publishes the combined report as `frontend-coverage-report` and appends the changed-line summary to the job summary. The E2E job uses `chrome` and `other-browsers` matrix lanes; only the Chrome lane runs API coverage and report generation.

Small runtime fixes were needed for safe, deterministic user-visible behavior: the import flow now avoids duplicate StrictMode requests and keeps its file state until the result is known; unresolved import prices show an actionable error and Go back; search and portfolio screens expose stable state selectors; and E2E MSW mode disables development-server HMR/live reload to keep persisted-state journeys stable.

Follow-up hardening converts the portfolio sync serializer to `portfolioSync.ts`, removes the successful-import assertion on a transient loading route in favour of the import response and final editor state, and uses the enabled Confirm weights action as the final readiness check in the editing journey.

## Context and Orientation

The repository is a root pnpm workspace. The frontend package is `dcapal-frontend`; the generated API package is `packages/api-client`; CI is defined in `.github/workflows/build-test.yml`. The backend remains the source of the OpenAPI contract, but this plan tests the frontend branch against MSW responses rather than a live backend.

A user journey is one readable path through the application that starts with a user goal and ends with an observable result. MSW (Mock Service Worker) is the existing request-interception library: its browser worker sees `fetch` requests and returns scenario data, while the React application remains unaware that the backend is simulated. V8 coverage is the JavaScript execution information produced by Chromium. Istanbul is the common report format used to combine coverage from the browser and Vitest. Changed-line coverage compares the report with the PR base (`origin/master`) and lists whether each changed source line was executed.

The branch's changed handwritten runtime includes `dcapal-frontend/src/api/portfolioSync.ts`, `priceProviders.js`, and `queryClient.js`; application startup in `src/index.js`; `src/app/index.js`; allocation state in `src/components/allocationFlow/portfolioSlice.js`; import and portfolio screens; `src/components/allocationFlow/steps/portfolio/searchBar.js`; the import and synchronization hooks; and the router's `SyncCoordinator` integration. The handwritten API transport is `packages/api-client/src/mutator/api-fetch.ts`.

The coverage denominator includes those changed runtime files and the transport file, plus any handwritten helper directly added for the runtime behavior. It excludes generated files under `packages/api-client/src/gen` and `packages/api-client/src/gen-mocks`, test files, MSW fixtures and handler plumbing, build/configuration files, coverage helpers, deleted code, and the test-only MSW bootstrap branch. The API-client public exports and transport behavior remain covered through real imports and focused tests. The existing demo route is not a required journey.

The current browser setup runs `webpack-dev-server` at `http://127.0.0.1:3000`, uses `dcapal-frontend/playwright.config.js`, and has desktop Chromium, Firefox, and WebKit projects. The coverage run will use Chromium desktop and one mobile Chromium device. The existing compatibility run remains unchanged except for migrated tests and any stable selectors.

## Plan of Work

### Milestone 1: Make scenarios and state deterministic

Add a small test-support layer under `dcapal-frontend/tests/support`. Define named scenario profiles for normal asset catalogs, successful and failing import, search empty/error/malformed responses, price conversion, authenticated sessions, token refresh, sign-out, and sync conflict/deletion. The Playwright fixture will set `x-e2e-scenario` before navigation. The application handlers in `dcapal-frontend/src/mocks/handlers.js` will read that header and return scenario-specific data; generated Orval handlers remain the fallback for operations not overridden by a DcaPal scenario.

Keep scenario data readable and stable. Use a fresh browser context for creation and search journeys. Use persisted storage only to arrange legitimate starting state for imported portfolios, existing portfolio-card operations, and authentication. Do not dispatch Redux actions, call internal hooks, or issue direct application API requests from a journey.

Replace the broad Supabase catch-all handler with scenario-aware HTTP responses sufficient for the real Supabase client to perform `getSession`, auth-state changes, refresh, and sign-out. Add a helper for a valid-looking browser session and make the authenticated scenario return deterministic access tokens. Keep live Supabase out of every test.

Add targeted stable selectors only where roles, labels, and visible text are not enough. Prefer accessible locators. Use stable IDs such as `route-import`, `route-allocate`, `portfolio-search`, `portfolio-card`, `asset-card`, `asset-result`, `transaction-fees`, and `import-error`; use `data-symbol` or `data-portfolio-id` attributes for identity rather than putting random IDs into the test ID itself.

At the top of every journey spec, write a short comment in this shape:

    /*
     * GIVEN an investor has opened the allocation flow
     * WHEN they search for and add a market asset
     * THEN the asset appears with a quote-currency price
     */

Use Playwright's clock in tests that need the five-minute price-refresh timer, the five-second synchronization interval, or the existing one-second import delay. Advance the clock immediately and wait for a request or visible result; never wait real minutes.

### Milestone 2: Migrate existing tests and add user journeys

Rewrite `dcapal-frontend/tests/routes.smoke.spec.js` and `pfolioCurrencies.spec.js` with journey comments and accessible selectors. Keep route coverage for `/`, `/allocate`, `/import`, `/login`, and missing-data redirects. Remove the demo route assertion. The goal is to verify the router and shared `SyncCoordinator` through real navigation without expanding unrelated route behavior.

Replace `dcapal-frontend/tests/syncPortfolios.spec.js` with visible portfolio actions. A test should create or load a portfolio through the UI, edit or duplicate/delete it, observe the real synchronization request at the MSW boundary, and assert the visible result. Do not call `/api/v1/sync/portfolios` directly from the page.

Add focused journey specs, keeping one goal per test and separate tests for materially different outcomes:

- Portfolio creation: load the fiat catalog, choose a name and quote currency, proceed to the portfolio editor, search for a DcaPal asset, add it after its price arrives, enter quantity and target weight, and reach the investment step.
- Asset search: cover the less-than-two-character guard, loading state, cash and crypto results, Yahoo results, empty results, malformed data, HTTP errors, DcaPal price failures, Yahoo bad price removal, a Yahoo price already in the quote currency, and a Yahoo price that needs conversion.
- Portfolio import: show the loading screen, import a successful fixture, verify names/assets/quantities/weights/fees and decimal-string conversion through visible values, handle a missing import ID, and handle an imported asset whose price cannot be resolved by showing the error and using “Go back”.
- Portfolio editing: change quantity, average buy price, and target weights; show under-allocation, exact allocation, and over-allocation; show positive, negative, and zero gain fixtures; remove an asset and return to the portfolio list.
- Fee policies: exercise zero, fixed, and variable portfolio fees; maximum fee impact; fixed amount; percentage; minimum and maximum values; invalid minimum/maximum bounds and recovery; asset-specific overrides; and return to the portfolio default.
- Portfolio management: create, rename and cancel, save a rename, duplicate, delete, reload persisted state, and verify the visible portfolio list. Where authenticated, assert the matching synchronization request.
- Synchronization: cover unauthenticated no-op behavior, initial authenticated sync, local-wins update, server-wins conflict, deleted portfolios, a successful expired-token refresh, and refresh/sign-out failure.
- Responsive coverage: repeat the changed portfolio/asset journeys in the mobile Chromium viewport so mobile-only render branches are executed.

Do not add a demo journey or continue the first-time creation journey through the optimizer recommendation. The optimizer's own existing tests remain responsible for its unchanged calculation code.

If a failure path currently leaves the UI in an unsafe or permanently loading state, make the smallest user-visible error-state fix before asserting it. The test should assert the safe outcome, not preserve an ambiguous implementation accident.

### Milestone 3: Cover the handwritten API transport

Expand `packages/api-client/src/mutator/api-fetch.test.ts` to use MSW-backed responses and real `fetch` behavior. Keep the tests focused on the transport boundary, not React internals. Cover configured base URL resolution, bearer-token injection, preservation of an explicit authorization header, no-token requests, JSON/text/empty response parsing, 204/205/304 responses, non-success normalized errors, one successful 401 refresh, refresh throwing, refresh returning no token, a second 401 causing the failure callback, and abort propagation.

Add the Vitest V8 coverage provider to the API-client package and configure coverage to include handwritten transport files while excluding generated output and tests. Keep the existing generated-client smoke test and make sure it runs in the coverage command.

### Milestone 4: Collect and merge coverage

Add a coverage-aware Playwright fixture or reporter under `dcapal-frontend/tests/support`. For Chromium coverage runs, start `page.coverage.startJSCoverage()` before navigation and stop it after the journey. Convert each returned V8 script using `v8-to-istanbul` and its Webpack source map, then write one Istanbul JSON fragment per test or worker. Do not collect coverage in the other-browsers compatibility lane.

Add a root coverage reporting script, for example under `scripts/coverage`, that merges browser fragments and the API-client Vitest `coverage-final.json` with `istanbul-lib-coverage`, then writes HTML, LCOV, JSON, and text reports. Preserve source paths so a reader can open a file in the combined report and distinguish `dcapal-frontend` from `packages/api-client`.

Add a changed-line reporter that compares `git diff --unified=0 origin/master...HEAD` with the merged coverage map. It should list every changed in-scope executable line as covered or uncovered and write a Markdown summary. It should return success even when a line is uncovered; the report is evidence for review, not a GitHub coverage gate. Test failures, failed conversion, failed merge, and missing report files must still fail the command.

### Milestone 5: Publish the report in GitHub Actions

Use one matrix `test:e2e:frontend` job in `.github/workflows/build-test.yml` with a Chrome lane and an other-browsers lane. The Chrome lane reuses the optimizer artifact and frozen pnpm install, installs Playwright browsers, runs Chromium desktop/mobile coverage projects, runs API-client Vitest coverage, merges reports, and writes the changed-line Markdown to `$GITHUB_STEP_SUMMARY`. The other-browsers lane runs Firefox and WebKit without coverage. The checkout must include enough history for the changed-line base comparison.

Upload the combined HTML, LCOV, JSON, text, and changed-line files as a named artifact with the same practical retention period as the existing Playwright report. Do not add a PR-comment action, third-party coverage service, or a coverage threshold. The job must still fail if tests or report generation fail.

Add package and root scripts with clear names, such as `api-client:test:coverage`, `frontend:test:e2e:coverage`, and `frontend:coverage:report`, or equivalent names that match the existing script style. The final plan revision must record the actual chosen names and commands.

## Concrete Steps

Run commands from the repository root unless the command uses a package filter.

1. Add the agreed coverage and reporting dependencies through the pnpm catalog and workspace lockfile. Expected tools are `@vitest/coverage-v8`, `v8-to-istanbul`, `istanbul-lib-coverage`, `istanbul-lib-report`, and `istanbul-reports`. Use the repository's current compatible versions rather than installing duplicate versions in each package.

2. Add the test-support modules and scenario-aware handlers. Run the existing Playwright suite before adding new assertions so the support changes have a clean baseline.

3. Add stable selectors, then migrate the route, currency, and sync tests. Run:

   pnpm frontend:test:e2e --project=chromium

   Expect the migrated tests to pass without a live backend. An unhandled `/api/` request must fail loudly through the MSW bootstrap.

4. Add the journey tests one vertical slice at a time. After each slice run:

   pnpm frontend:typecheck
   pnpm frontend:test
   pnpm frontend:test:e2e --project=chromium

5. Add the transport edge tests and package coverage command. Run:

   pnpm --filter @dcapal/api-client typecheck
   pnpm --filter @dcapal/api-client test
   pnpm --filter @dcapal/api-client test:coverage

   Expect successful generated-client and transport tests and a machine-readable coverage file for the handwritten mutator.

6. Add native browser coverage collection and the merge/changed-line script. Run the coverage command locally with the Chromium desktop and mobile projects. Expect HTML, LCOV, JSON, text, and changed-line Markdown files. Inspect the Markdown and confirm no changed in-scope executable line is uncovered.

7. Add the CI matrix and run the same commands locally. Verify that the Chrome lane runs desktop/mobile coverage plus package coverage, while the other-browsers lane runs Firefox and WebKit without coverage. The implemented scripts are `pnpm frontend:test:e2e:coverage`, `pnpm --filter @dcapal/api-client test:coverage`, and `pnpm frontend:coverage:report`.

8. Run the final repository checks:

   pnpm install --frozen-lockfile
   pnpm frontend:lint
   pnpm frontend:typecheck
   pnpm frontend:test
   pnpm frontend:build:dev
   pnpm frontend:test:e2e:coverage
   pnpm frontend:test:e2e --project=firefox --project=webkit
   pnpm --filter @dcapal/api-client typecheck
   pnpm --filter @dcapal/api-client test
   pnpm frontend:coverage:report
   git diff --check

   Run the repository's existing backend/OpenAPI checks if the workspace validation requires them. This testing task must not change the generated OpenAPI contract.

## Validation and Acceptance

The frontend test suite passes in a real browser with MSW enabled and no live backend. Each new journey file begins with a readable GIVEN/WHEN/THEN comment, uses semantic locators before test IDs, and leaves no direct application endpoint calls or frontend module mocks.

The portfolio creation journey reaches the investment step. Import journeys visibly distinguish loading, successful imported data, missing imports, and unresolved prices. Search journeys visibly distinguish loading, results, empty/error behavior, DcaPal prices, Yahoo prices, and currency conversion. Portfolio editing visibly covers the agreed fee, weight, gain, persistence, and removal states. Authenticated synchronization visibly covers local/server conflict outcomes and request-level auth refresh/failure behavior.

The API-client transport tests pass with MSW-backed real fetches and cover every agreed transport branch. Generated Orval production and mock output is not directly counted in the handwritten coverage denominator.

The combined report contains HTML, LCOV, JSON, and text output. The changed-line Markdown report lists all changed in-scope executable lines relative to `origin/master` and shows every one as covered. GitHub Actions publishes the report in the coverage job summary and uploads it as an artifact. CI does not fail because a percentage is below a threshold; it fails only for test, collection, conversion, merge, or report-generation errors.

The matrix retains desktop Chromium coverage, Firefox, and WebKit compatibility, and adds mobile Chromium coverage. The demo portfolio route is not part of the acceptance suite. The final local matrix shows both lanes green, 15/15 API-client coverage tests, 745/745 changed executable lines, frontend lint, frontend typecheck, frontend unit tests, API-client typecheck/tests, and the development build.

## Idempotence and Recovery

Scenario handlers and coverage fragments must be reset between tests and written to ignored, disposable directories. Running a test twice must not reuse a previous test's sync store, authentication state, or coverage fragment.

Coverage collection is additive to the ordinary test command. If native V8 conversion fails because a source map cannot be resolved, keep the raw coverage fragment, inspect its script URL and source map, correct the conversion path, and rerun the reporter. Do not add application instrumentation just to hide a source-map problem.

If a journey fails because the current UI has no safe error outcome, fix the smallest visible behavior and record it in `Surprises & Discoveries` and `Decision Log`. Do not use a broad coverage ignore comment. Narrow exclusions are limited to generated files, test/MSW plumbing, coverage helpers, and environment-only bootstrap code described in Context and Orientation.

Do not delete or overwrite the user's unrelated untracked `MIGRATION.md` or `agent_docs/` files. Do not reset the worktree to recover from a failed test. Generated coverage artifacts may be removed only from their explicit coverage output directory after preserving any needed report evidence.

## Artifacts and Notes

The durable architecture decision is [ADR 004](../adr/004-frontend-user-journey-coverage.md). The frontend domain glossary is [dcapal-frontend/CONTEXT.md](../../dcapal-frontend/CONTEXT.md); no new testing terms belong there.

The expected test-support layout is:

    dcapal-frontend/tests/
      support/
        auth.js
        coverage.js
        scenarios.js
        state.js
      journeys/
        create-portfolio.spec.js
        import-portfolio.spec.js
        search-assets.spec.js
        edit-portfolio.spec.js
        manage-portfolios.spec.js
        sync-portfolios.spec.js

The exact helper names may change if the final design remains equivalent. Keep the journey files small enough that a reviewer can understand the user goal and expected result without reading the fixture implementation.

The expected GitHub artifacts are `playwright-report-other-browsers` and `frontend-coverage-report`. The latter contains the HTML report, `lcov.info`, JSON, text summary, and changed-line Markdown.

## Interfaces and Dependencies

The Playwright scenario fixture must set the scenario header before navigation and expose only test setup operations such as selecting a scenario, seeding legitimate storage, and advancing the browser clock. It must not expose application Redux actions or generated API functions to tests.

The application MSW handlers must accept the scenario header and return valid OpenAPI-shaped responses for asset catalogs, search, chart, price, import, sync, and Supabase auth requests. Scenario responses must use decimal strings wherever the REST contract uses decimal strings. Generated Orval mock handlers remain the fallback for operations without application-specific fixture data.

The coverage converter must turn each browser V8 entry into an Istanbul coverage object keyed by the original source path. The merger must accept both browser fragments and the API-client Vitest `coverage-final.json`, merge hit counts without losing either source, and write the standard report formats.

The changed-line reporter must take the PR base ref and coverage map as inputs, restrict its output to the agreed in-scope paths, and produce deterministic Markdown. Its output must include the file, line number, covered/uncovered status, and a final count.

The E2E matrix job must depend on the same optimizer artifact as the existing frontend jobs, use `pnpm install --frozen-lockfile`, fetch the base history required for changed-line comparison, publish the report through `actions/upload-artifact`, and append the changed-line summary to `$GITHUB_STEP_SUMMARY`. It must not add a global threshold or external service.

Revision note (2026-08-01): Created after the confirmed batch-grilling session. The plan records the real-browser/MSW boundary, the explicit demo exclusion, the changed-line-only acceptance rule, native Chromium coverage, the transport test seam, deterministic browser time, user-journey comments, and GitHub artifact/report behavior.
Revision note (2026-08-02): Implemented and locally verified. The report covers all 697 changed in-scope executable lines, and the workflow publishes the combined report without a percentage gate.
Revision note (2026-08-02): Follow-up after PR review and CI inspection. New frontend runtime source is TypeScript, E2E/coverage runs in a two-lane matrix, coverage checkout uses full history, non-code source-map entries are excluded from artifact reports, and the two retried journeys use stable final-state assertions.
