# ADR 0002: Use TimescaleDB PostgreSQL as the application database

- Status: accepted
- Date: 2026-08-13

## Context

DcaPal uses SQLx migrations and PostgreSQL repositories for users, saved
portfolios, and portfolio assets. The repository also runs Supabase for
authentication. These are two different database boundaries: Supabase's
internal PostgreSQL instance supports the authentication stack, while DcaPal's
Compose `db` service stores application data.

Using different database implementations for local development, backend tests,
and the application runtime would allow SQL that works in one environment to
fail in another. The application database therefore needs one explicit
TimescaleDB PostgreSQL 18 contract.

## Decision

Use the official image tag
`timescale/timescaledb-ha:pg18.4-ts2.28.3-all-oss` for the Compose-managed
DcaPal application database. The tag is intentionally used without a digest;
the image tag is the repository's version contract.

Run SQLx migrations and ordinary backend tests against this TimescaleDB
service. Keep Supabase separate and start it only for workflows that exercise
Supabase-specific authentication, including the full-stack browser smoke test.

The backend keeps the configured shared JWT secret for HS256 tokens and also
accepts the public ES256 signing keys exposed by current local Supabase Auth
through the optional `jwtJwks` setting. The smoke setup supplies those public
keys; no private key or production secret is committed.

TimescaleDB extension availability is owned by database provisioning. The
backend does not add a startup capability check, a warning query, or a runtime
fallback to plain PostgreSQL. This issue also does not introduce hypertables,
UUIDv7 identifiers, or new application tables.

## Consequences

The local `make backend-db-up` target starts TimescaleDB on port 5433, and
`make test-backend` uses that database by default. The ordinary backend CI job
does the same without starting Supabase.

The full-stack smoke job starts both stacks. The backend points at the Compose
TimescaleDB service, while the frontend points at Supabase Auth. A browser
journey proves that a Supabase-issued token reaches the unchanged portfolio
synchronization route, and a query executed inside the Compose `db` container
proves that the resulting rows were written to TimescaleDB rather than to
Supabase's internal database.

PostgreSQL major-version upgrades use a custom-format logical backup and
restore. The source database remains available until the restored PostgreSQL
18 database has passed migration and data checks.
