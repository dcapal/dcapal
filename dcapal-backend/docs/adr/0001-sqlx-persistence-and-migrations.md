# ADR 0001: Use SQLx for PostgreSQL persistence and migrations

- Status: accepted
- Date: 2026-08-01

## Context

The backend used SeaORM for PostgreSQL repositories and for its standalone
migration crate. Production already contains the tables created by those
migrations, so the replacement must not rebuild or delete them. Repository
tests also need to exercise PostgreSQL constraints and transactions.

## Decision

Use SQLx 0.9.0 with the stable Rust toolchain. Keep a separate migration crate
and embed the service-root SQL migrations with `sqlx::migrate!`. Preserve the four existing
numeric migration versions as reversible `.up.sql` and `.down.sql` files. The
up files are idempotent so they can run against the existing tables; SQLx uses
`_sqlx_migrations` and leaves `seaql_migrations` untouched.

Keep repository contracts in the existing outbound ports. Put PostgreSQL
implementations and SQLx `FromRow` persistence types below
`ports/outbound/repository/postgres`. Use explicit-column runtime queries and
transactions for portfolio writes. Use `sqlx::test` with deterministic SQL
fixtures for repository integration tests.

## Consequences

The backend no longer depends on SeaORM or its migration CLI. Deployments keep
their current startup order and command form, while local tests require a
running Supabase PostgreSQL instance. SQLx migrations have a separate history,
so old SeaORM migration metadata remains visible for compatibility. Portfolio
asset uniqueness is intentionally not added here; it is tracked separately in
[issue #714](https://github.com/dcapal/dcapal/issues/714).
