# ADR 0001: Use SQLx for PostgreSQL persistence and migrations

- Status: accepted
- Date: 2026-08-01

## Context

The backend uses SQLx for PostgreSQL repositories and its standalone migration
crate. Production already contains the tables created by earlier migrations,
so later changes must not rebuild or delete them. Repository tests also need to
exercise PostgreSQL constraints and transactions.

## Decision

Use SQLx 0.9.0 with the stable Rust toolchain. Keep a separate migration crate
and embed the service-root SQL migrations with `sqlx::migrate!`. Preserve all
applied numeric migration versions and checksums; add new forward migrations
for schema changes. SQLx records migration history in `_sqlx_migrations`.

Keep repository contracts in the existing outbound ports. Put PostgreSQL
implementations and SQLx `FromRow` persistence types below
`ports/outbound/repository/postgres`. Use explicit-column runtime queries and
transactions for portfolio writes. Use `sqlx::test` with deterministic SQL
fixtures for repository integration tests.

## Consequences

Deployments keep their current startup order and command form, while local
tests require a running application PostgreSQL instance. Production recovery
is forward-only from the pre-deployment backup; development down migrations
are not a production rollback strategy.
Portfolio asset uniqueness is intentionally not added here; it is tracked
separately in [issue #714](https://github.com/dcapal/dcapal/issues/714).
