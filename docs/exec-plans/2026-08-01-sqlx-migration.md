# Replace SeaORM with SQLx

This ExecPlan is a living document. It follows `/Users/leonardoarcari/.codex/PLANS.md` and must be updated as implementation progresses. The related deferred uniqueness work is tracked in GitHub issue #714.

## Purpose / Big Picture

DcaPal Backend will use plain SQLx 0.9.0 instead of SeaORM for PostgreSQL access and schema migrations. After this change, the backend will keep its current HTTP behavior, run against the existing production tables without rebuilding them, and test repository behavior against real PostgreSQL databases created by `sqlx::test`.

The migration must be observable in two ways. A fresh Supabase PostgreSQL database must receive the complete schema from the SQL migration files, and a database that already has the SeaORM-created tables must accept the same migration run without losing data. The full backend test suite will be runnable through Makefile targets after Supabase PostgreSQL is started.

## Progress

- [x] (2026-08-01) Read the repository instructions, domain map, backend glossary, migration code, deployment scripts, and upstream `asktobi/services/api` layout.
- [x] (2026-08-01) Confirmed the migration, repository, test, Makefile, CI, and documentation decisions through the grilling session.
- [x] (2026-08-01) Published the deferred portfolio-asset uniqueness specification as GitHub issue #714.
- [x] Add SQLx 0.9.0 dependencies and remove SeaORM dependencies.
- [x] Replace the SeaORM migration crate with an embedded SQLx migration runner.
- [x] Port the existing schema history to idempotent SQLx migrations.
- [x] Replace SeaORM entity models and repository calls with typed SQLx row types and queries.
- [x] Enforce saved-portfolio ownership during SQLx writes.
- [x] Add SQLx integration tests, fixtures, and migration compatibility coverage.
- [x] Add Supabase-backed Makefile targets and update backend CI.
- [x] Update the ADR, migration documentation, and backend testing documentation.
- [x] Run formatting, linting, tests, and migration checks; Docker validation remains environment-blocked.

## Surprises & Discoveries

- Observation: Production already has SeaORM-created tables and a SeaORM migration-history table, but SQLx uses its own migration-history table.
  Evidence: The existing deployment runs the `migration` binary, while the current Rust migration crate is based on `sea-orm-migration`; SQLx migrations will therefore create `_sqlx_migrations` without reading `seaql_migrations`.

- Observation: The current schema does not enforce uniqueness for a portfolio asset symbol within a saved portfolio.
  Evidence: The portfolio-asset migration has no unique constraint on `(portfolio_id, symbol)`, and the repository searches existing assets by symbol in application code. This is intentionally deferred to issue #714.

- Observation: The domain glossary promises owner-only saved-portfolio changes, but the current repository can update or soft-delete by portfolio ID without checking the owner.
  Evidence: The current `upsert` and `soft_delete` paths do not include the user ID in their ownership checks. The SQLx rewrite will enforce the glossary invariant.

- Observation: The upstream reference uses service-root `.up.sql` and `.down.sql` files, PostgreSQL-specific repositories, separate row types, runtime SQLx queries, and SQLx fixtures.
  Evidence: `asktobi/services/api` at commit `7bb8f376` follows this layout. DcaPal will adapt it under its existing ports-and-adapters boundary.

- Observation: Rust 1.97.1 is the latest stable toolchain available for this task, and SQLx 0.9.0 is the current SQLx release.
  Evidence: `rustc 1.97.1` was installed and used for workspace checks; `cargo info sqlx@0.9.0` resolved SQLx 0.9.0.

- Observation: Docker is not running in the local validation environment.
  Evidence: `make backend-db-up` reached the existing Supabase CLI target but failed because the Docker daemon socket was unavailable. A temporary local PostgreSQL instance was used for equivalent SQLx integration and migration checks.

## Decision Log

- Decision: Use SQLx 0.9.0 with PostgreSQL, Tokio, Rustls, migrations, derive macros, UUID, chrono, and rust_decimal support.
  Rationale: This is the current SQLx release and supplies the pool, typed row decoding, migration runner, and `sqlx::test` features required by the backend.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Use the stable Rust toolchain for the workspace, CI, and backend build image.
  Rationale: The root `rust-toolchain.toml` is the single source of truth for the selected toolchain. `Cargo.toml` keeps `rust-version = "1.97.1"` as the minimum supported compiler version.
  Date/Author: 2026-08-02 / Codex and user.

- Decision: Preserve all four historical schema steps as SQLx migrations with numeric timestamp versions and reversible `.up.sql` / `.down.sql` files.
  Rationale: Fresh databases retain the existing schema history, while idempotent up migrations can safely run against production tables that were created by SeaORM.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Leave `seaql_migrations` untouched and let SQLx create and maintain `_sqlx_migrations`.
  Rationale: The two migration systems use different histories; changing or importing the old history is riskier than applying the idempotent historical SQL and recording the new SQLx history.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Keep a separate migration binary, backed by `sqlx::migrate!`, that accepts both a zero-argument `DATABASE_URL` form and the existing `up -u <url>` form.
  Rationale: This preserves the current deployment shape while removing the SeaORM CLI dependency.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Put SQLx row types and PostgreSQL repository implementations below the outbound repository boundary, with repository traits at the existing application-facing boundary.
  Rationale: This follows the reference layout without moving DcaPal’s existing ports-and-adapters structure.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Use runtime typed SQLx queries rather than compile-time query macros and `.sqlx` metadata.
  Rationale: The migration does not need a build-time database connection or a new metadata preparation workflow.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Preserve current duplicate-asset and timestamp behavior, except enforce the existing saved-portfolio ownership invariant.
  Rationale: The migration should avoid unrelated schema and behavior changes; uniqueness enforcement is separately specified in issue #714.
  Date/Author: 2026-08-01 / Codex and user.

- Decision: Use real PostgreSQL integration tests with `sqlx::test`, deterministic SQL fixtures, and a migration compatibility test.
  Rationale: Repository behavior and migration safety depend on PostgreSQL constraints and transaction behavior that mocks cannot prove.
  Date/Author: 2026-08-01 / Codex and user.

## Outcomes & Retrospective

The implementation is complete. Fresh-schema migrations and the production-shaped
compatibility migration both passed against PostgreSQL. The compatibility test
confirmed that SQLx creates four `_sqlx_migrations` rows and leaves
`seaql_migrations` in place. Repository tests covered user writes, portfolio
reads and writes, asset removal, and ownership checks. The Makefile migration
target was run twice successfully, and `make test-backend` passed.

Formatting, workspace Clippy with warnings denied, workspace checks, and Rust
test compilation all passed with Rust 1.97.1. The Supabase CLI target itself
could not start because Docker was unavailable; CI now starts and stops
Supabase around `make test-backend`.

## Context and Orientation

The workspace contains the backend package, the optimizer package, and the current SeaORM-based migration package. The backend creates one PostgreSQL pool in `src/lib.rs`, passes that pool to concrete repositories under `src/ports/outbound/repository`, and maps persistence models into REST responses. The current `src/app/domain/db` modules are generated SeaORM entities and will be removed after equivalent SQLx row types exist.

The database schema has three tables used by the backend: `users`, `portfolios`, and `portfolio_asset`. Users own saved portfolios. A saved portfolio has portfolio assets and a soft-deletion marker. The existing schema also stores fee fields, timestamps, and an optional average buy price. The migration must preserve these names, nullability rules, defaults, foreign keys, and existing numeric definitions.

The current production startup waits for PostgreSQL, runs the standalone migration binary, and then starts the backend. The replacement must preserve that ordering. Local Supabase PostgreSQL is configured on port 54322, and the Makefile already has `supabase-up` and `supabase-down` targets.

## Plan of Work

First, update the workspace dependency graph. Replace the SeaORM dependency with SQLx 0.9.0 and add the required runtime, migration, derive, and PostgreSQL type features. Remove the SeaORM migration dependencies and replace the migration package implementation with a Tokio-based SQLx runner. Keep the migration package as a workspace member so Docker and deployment can continue to build a separate binary.

Next, create service-root SQL migration files. Translate the four existing SeaORM migrations into numeric SQLx versions in the same order. The creation statements must be idempotent for the production handoff, and the average-buy-price alteration must use an idempotent column addition. Down files must reverse dependencies and be documented as development-only destructive operations.

Then, replace generated SeaORM entities with persistence-only SQLx row types. Define application-facing repository traits, PostgreSQL implementations that own the pool, and explicit-column SQL queries. Keep request, response, and domain types stable. Use transactions for portfolio and portfolio-asset writes, return database rows with `RETURNING`, and preserve the existing asset deletion and fee-field behavior.

During the repository rewrite, enforce saved-portfolio ownership. An upsert for an existing portfolio owned by another user returns a generic bad request. A delete for a portfolio not owned by the caller is a no-op. Do not add the deferred portfolio-asset uniqueness constraint in this change.

Add SQLx integration tests under the backend test boundary. Use `sqlx::test` with the service-root migrations and named fixture SQL files. Cover user creation and update, portfolio reads, portfolio upserts, asset removal, soft deletion, ownership violations, and transaction behavior. Add a compatibility test that creates the current production-shaped schema, including legacy migration metadata, then runs the SQLx migrator.

Update the Makefile with Supabase-backed database lifecycle and test targets. The database check must run `psql` with the same `DATABASE_URL` used by SQLx. The backend test target must require the database to be running rather than starting it implicitly. Update CI to install the Supabase CLI, start Supabase, run the Makefile backend test target, and stop Supabase on cleanup.

Finally, add the backend ADR and update migration and backend README instructions. The backend glossary remains unchanged because this work adds no domain terms. Run all validation commands and update this plan with evidence.

## Concrete Steps

Run all commands from the repository root unless stated otherwise.

1. Establish the baseline.

       cargo test -p dcapal-backend -- --nocapture
       cargo test -p migration -- --nocapture

   The baseline should pass or expose an environment-only failure before source changes begin.

2. Update dependencies and migration code. Use `cargo check --workspace` after the dependency change to expose SQLx feature or Rust-version issues early.

3. Create the SQL migration files and run the migration compatibility test against the Supabase database.

       make supabase-up
       make backend-db-check
       make backend-migrate

4. Run the backend integration suite.

       make test-backend

5. Run repository quality checks.

       cargo +nightly fmt --all -- --config-path rustfmt.nightly.toml --check
       cargo clippy --workspace --all-targets -- -D warnings
       cargo test -p dcapal-backend -p migration -- --nocapture

6. If a Docker build is available, build the backend image and verify that its startup script can run the migration binary before the application.

       docker build -f dcapal-backend/docker/Dockerfile -t dcapal-backend:sqlx-migration .

7. Stop local Supabase after validation when it was started only for this task.

       make supabase-down

## Validation and Acceptance

The change is accepted when all of the following are true:

- `cargo check --workspace` succeeds without SeaORM or sea-orm-migration dependencies.
- A fresh Supabase PostgreSQL database receives `users`, `portfolios`, and `portfolio_asset` with the existing columns, defaults, foreign keys, and average-buy-price column.
- Running the migration against an existing production-shaped schema succeeds without dropping or rewriting data.
- The old `seaql_migrations` table remains untouched and SQLx records its own applied migrations.
- The migration binary works with both `DATABASE_URL` and `up -u <url>` invocation forms.
- The backend starts after migrations complete in the existing Docker startup flow.
- Repository tests use PostgreSQL through `sqlx::test` and deterministic fixtures rather than SeaORM mocks.
- Portfolio synchronization still returns the same API shapes and preserves current fee, asset-deletion, and timestamp behavior.
- Cross-user portfolio updates are rejected and cross-user deletes do not modify data.
- `make test-backend` fails early when Supabase PostgreSQL is unavailable and passes when it is available.
- CI starts PostgreSQL before the backend test target and tears it down afterward.
- The ADR and README instructions describe the new migration and test workflow.
- Issue #714 remains separate and is not implemented by this change.

## Idempotence and Recovery

The historical SQLx up migrations must be safe to run against tables already created by SeaORM. SQLx will record successful runs in `_sqlx_migrations`; rerunning the migration binary will then do nothing. The old SeaORM metadata table must not be deleted or rewritten.

Down migrations are for local development and test reset only. They are destructive because they remove schema objects and data. Production recovery must use a database backup and a reviewed forward migration; do not run a down or reset command against production.

If the migration compatibility test fails, inspect the exact SQL error and compare the SQL file with the existing SeaORM migration definition. Do not make the production migration destructive to make a test pass. If a production-shaped schema differs from the repository definition, stop and document that difference before changing the migration.

If the backend test target cannot connect, start Supabase with `make supabase-up` and verify the configured PostgreSQL URL with `make backend-db-check`. Do not replace SQLx integration tests with mocks to bypass a missing database.

## Artifacts and Notes

The important implementation artifacts will be:

- the workspace and package manifests with SQLx 0.9.0;
- service-root reversible SQL migrations;
- the standalone SQLx migration binary;
- PostgreSQL repository traits, implementations, and row types;
- SQLx fixture and compatibility tests;
- Makefile and CI database lifecycle changes;
- the backend ADR and updated migration instructions.

The deferred uniqueness work is tracked at https://github.com/dcapal/dcapal/issues/714.

## Interfaces and Dependencies

The backend must expose a PostgreSQL pool based on `sqlx::PgPool` from the application context. Repositories must receive a cloned pool and expose application-facing async methods through object-safe traits.

The user repository must provide an operation that creates a user from authentication claims when no row exists and updates the stored username, email, role, and update timestamp when the row exists.

The portfolio repository must provide operations to load a user’s saved portfolios with their portfolio assets, upsert a portfolio and its assets in one transaction, and soft-delete a saved portfolio for its owner. The portfolio upsert must preserve the current fee-field mapping and remove database assets absent from the incoming portfolio.

Persistence rows must derive SQLx row decoding and use `rust_decimal::Decimal`, `chrono` timestamps, and `uuid::Uuid` for the existing PostgreSQL types. SQL must use explicit column lists and PostgreSQL bind parameters.

The migration package must export one embedded SQLx migrator over the service-root migration directory. Its binary must connect using `DATABASE_URL`, run pending migrations, and support the compatibility invocation required by the current deployment script.

## Implementation Notes

This plan was created after the grilling and domain-modeling session. No source files were changed during that design session. The implementation must update this file after each milestone, including timestamps, evidence, surprises, and final outcomes.
