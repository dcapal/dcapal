# Findings: TimescaleDB production, local, and CI parity

Research date: 2026-08-05

Implementation update (2026-08-13): issue #794 adopts the TimescaleDB
PostgreSQL 18 application-database contract and keeps Supabase as a separate
authentication stack. The durable decision is recorded in
[backend ADR 0002](../../dcapal-backend/docs/adr/0002-timescaledb-application-database.md).

The numbered findings below describe the pre-implementation baseline where
they refer to the former PostgreSQL 17 image or Supabase-backed test setup.

## Context

- Ticket: [Research: Verify TimescaleDB local and CI parity](https://github.com/dcapal/dcapal/issues/757)
- Wayfinder map: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/742)

This is a research finding, not a product-code change. It uses the current
repository source and first-party TimescaleDB, SQLx, and Supabase documentation.
Redis is explicitly outside this epic.

## Historical findings (pre-issue #794 baseline)

The findings below preserve the research evidence that led to issue #794.
They describe the repository before the implementation and are not current
acceptance criteria where the implementation update above says otherwise.

1. **The repository already has a suitable local TimescaleDB service.** The base
   Compose file uses `timescale/timescaledb-ha:pg17`, exposes PostgreSQL only to
   the Compose network, supplies `POSTGRES_PASSWORD`, and waits for both
   `pg_isready` and `SELECT 1`. This is the same official image family and
   PostgreSQL major version recommended by Timescale's Docker installation guide,
   which documents `timescale/timescaledb-ha:pg17` and says the image pre-creates
   TimescaleDB in the default database and adds it to new databases. [Repository
   Compose service](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/docker-compose.yml#L20-L37)
   and [Timescale Docker installation](https://docs.timescale.com/self-hosted/latest/install/installation-docker/)

2. **The production contract must be explicit about the extension.** A
   Timescale image makes the extension available, but a database migration still
   needs to verify or create it in the target database with
   `CREATE EXTENSION IF NOT EXISTS timescaledb`. Timescale describes TimescaleDB
   as a PostgreSQL extension and gives this command for self-hosted instances.
   Production is therefore Timescale-compatible only when its PostgreSQL target
   has the extension installed and enabled; a plain Supabase PostgreSQL target
   is not equivalent merely because local Compose is Timescale-based. [Timescale
   self-hosted installation](https://docs.timescale.com/self-hosted/latest/install/)
   and [Timescale troubleshooting: create the extension](https://docs.timescale.com/self-hosted/latest/troubleshooting/)

3. **Hypertables fit time-series observations, not the current portfolio tables.**
   Timescale defines a hypertable as a PostgreSQL table partitioned by time. A
   candidate observation table should have a non-null `timestamptz` partition
   column, a stable series identity, and query indexes such as
   `(series_id, observed_at)`. Any primary key or unique index on a hypertable
   must include every partitioning column, normally the time column. [Timescale
   `create_hypertable` guidance](https://docs.timescale.com/api/latest/hypertable/create_hypertable/)
   and [hypertable unique-index rules](https://docs.timescale.com/use-timescale/latest/hypertables/hypertables-and-unique-indexes/)

4. **Keep application job state in ordinary application tables.** Timescale's
   `timescaledb_information.jobs` and `job_history` describe the extension's
   internal policy and background-worker jobs, including schedule, retries,
   next start, success, and error fields. They are useful for observing retention
   or aggregate policies, but they are not a substitute for a DcaPal fetch-job
   table with an application idempotency key, ownership/claim state, provider
   identity, requested range, and durable error details. [Timescale jobs view](https://docs.timescale.com/api/latest/informational-views/jobs/)
   and [Timescale job history view](https://docs.timescale.com/api/latest/informational-views/job_history/)

5. **SQLx can run the same migrations and fixtures against TimescaleDB.** The
   repository embeds migrations with `sqlx::migrate!("../migrations")`; the
   migration runner connects using `DATABASE_URL` and runs the embedded
   `Migrator`. SQLx documents that `migrate!` embeds migrations in the binary and
   that migration files are resolved relative to the Cargo project root. The
   Timescale-specific risk is therefore SQL, not the Rust pool boundary: the
   migration must be tested against a database where the extension is available,
   and any hypertable unique constraint must include its time partition column.
   [Repository migration crate](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/migration/src/lib.rs#L1-L2),
   [repository migration runner](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/migration/src/main.rs#L7-L18),
   and [SQLx `migrate!` documentation](https://docs.rs/sqlx/0.9.0/sqlx/macro.migrate.html)

6. **The current SQLx compatibility boundary is valuable and should remain.**
   The repository has a migration-compatibility test that starts from a fixture
   representing the existing SeaORM schema, runs the SQLx migrator, checks
   `_sqlx_migrations`, preserves `seaql_migrations`, and verifies the numeric
   column type. A Timescale migration should add the same kind of adoption test:
   verify extension availability, hypertable status, partition column, indexes,
   and application metadata without assuming an empty database. [Migration
   compatibility test](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/tests/migration_compatibility.rs#L1-L30)
   and [migration ADR](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/docs/adr/0001-sqlx-persistence-and-migrations.md#decision)

7. **Ordinary CI should use the Timescale Compose database for backend tests.**
   The pre-issue backend test job (now `build-test-backend`) installed the
   Supabase CLI, started Supabase,
   and runs `make test-backend`; the Makefile defaults `DATABASE_URL` to
   Supabase's `127.0.0.1:54322` endpoint. This makes ordinary SQLx tests depend
   on the wrong database implementation for a Timescale contract. The ordinary
   job should instead start only the Timescale `db` service with the local Compose
   port mapping, wait for its health check, set `DATABASE_URL` to that mapped
   endpoint, and run the same migration and SQLx test commands. [Current backend
   CI job](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/.github/workflows/build-test.yml#L54-L92),
   [Makefile database defaults and tests](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/Makefile#L1-L7),
   [Makefile Compose targets](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/Makefile#L99-L117),
   and [SQLx test attribute documentation](https://docs.rs/sqlx/latest/sqlx/attr.test.html)

8. **Supabase should be isolated to a smoke-test boundary.** The existing
   The pre-issue smoke job (now `test-e2e-smoke`) already started Supabase
   separately, configured the application to use the Compose `db` service, and
   started the Timescale/Redis
   application stack with `docker compose ... up --wait`. That job is the right
   place to retain Supabase checks that exercise Supabase-specific auth or local
   service wiring. It should not be the database oracle for ordinary repository
   migrations or SQLx tests. Supabase's own documentation describes the CLI as a
   local configuration and development environment; it does not make a Supabase
   database and a self-hosted Timescale image the same production contract.
   [Current backend smoke job](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/.github/workflows/build-test.yml#L330-L481)
   and [Supabase local configuration documentation](https://supabase.com/docs/guides/local-development/managing-config)

9. **Redis is a separate runtime dependency and should not be changed here.**
   The Compose file defines Redis with its own image and health check, and the
   backend constructs Redis-backed market, statistics, imported, and miscellaneous
   repositories at application startup. The current database integration tests,
   however, use only `PgPool` and SQLx fixtures. The Timescale parity epic can
   therefore exclude Redis while keeping the existing Redis service in the smoke
   stack and leaving Redis-specific behavior for a separate issue. [Redis Compose
   service](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/docker-compose.yml#L2-L18),
   [backend runtime wiring](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/lib.rs#L125-L145),
   and [current PostgreSQL-only integration tests](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/tests/portfolio_repository.rs#L58-L129)

## Future Timescale work outside issue #794

Issue #794 deliberately does not add a Timescale-specific migration, create
hypertables, or change the application schema. Timescale provisioning owns
extension availability. The following ideas remain separate future work and
must not be read as requirements for the current database upgrade.

If a future schema introduces time-series observations, its design should:

- Keep extension provisioning outside application migrations, and verify the
  operational prerequisite in the deployment process rather than adding
  `CREATE EXTENSION` to issue #794.
- Create the observation table with a non-null `timestamptz` partition column and
  an application-level series key. Choose the unique key with the time column
  included, for example `(series_id, observed_at)`.
- Keep provider metadata, fetch timestamps, and application job state in ordinary
  columns/tables. Inspect Timescale policy jobs through informational views, but
  do not use those views as the application's job ledger.
- Add a compatibility test for the existing schema and a Timescale-specific test
  that asserts `pg_extension`, hypertable metadata, indexes, and idempotent
  re-running of the migration.
- Keep repository tests on `#[sqlx::test(migrations = "./migrations", fixtures(...))]`.
  SQLx applies migrations and fixtures in isolated test databases; fixture order
  must continue to satisfy foreign keys. [SQLx test and fixture guidance](https://docs.rs/sqlx/latest/sqlx/attr.test.html)
- Preserve the issue #794 boundary in any future schema work: ordinary backend
  CI uses the Timescale Compose `db` service, while Supabase remains an
  explicitly named authentication-smoke dependency.

## Future-work validation ideas

These checks apply only if the separate hypertable/schema work is taken up:

A future implementation is ready when a clean checkout can demonstrate all of
the following:

1. The Compose database reports healthy only after PostgreSQL accepts both
   readiness and a real query, and `SELECT extname, extversion FROM pg_extension`
   reports `timescaledb`.
2. The migration runner succeeds against the Timescale Compose database and a
   second run reports no pending migration.
3. The migration-compatibility test preserves the existing SeaORM metadata and
   the new Timescale test proves the intended hypertable and index metadata.
4. `cargo test -p dcapal-backend -p migration -- --nocapture` passes with
   `DATABASE_URL` pointing at TimescaleDB and without starting Supabase or Redis.
5. The named Supabase smoke job still starts its own Supabase services and the
   existing Compose application stack, while failure logs identify which service
   failed.

For issue #794, the production image contract and PostgreSQL upgrade procedure
are recorded in ADR 0002 and the upgrade runbook. Any future decision about
Timescale-specific schema objects should be made separately from that accepted
application-database boundary.
