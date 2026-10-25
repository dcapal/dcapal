# DcaPal backend guidance

For any work under `dcapal-backend/`, read these documents before editing:

- [CODING_STANDARDS.md](CODING_STANDARDS.md) for the REST → command → service
  → outbound-port boundary and error flow.
- [CONTEXT.md](CONTEXT.md) for backend domain terms and context boundaries.
- Relevant records under `docs/adr/` and backend documentation when the change
  affects a recorded architectural decision.

These instructions apply recursively. Keep transport types at the REST
boundary, application and domain types in application code and ports, and
storage/provider details in adapters. Preserve the existing test and comment
standards from the repository root `AGENTS.md`.

For Rust backend changes, prefer typed application/domain values at command,
service, and outbound-port seams; use fallible `Command::try_new` for semantic
validation and context lookups. Use `thiserror` for errors, `derive_more` for
small mechanical trait implementations, and `mockall` for test-only port
mocks. Use RAII guards for scoped stack/path state, bind SQL values (including
enum discriminants) instead of embedding literals, and never rewrite applied
migrations to satisfy a new test; add forward migrations or test fixtures.

## Conventions for new endpoints and services

Apply these when adding or changing a use case; `CODING_STANDARDS.md` has the
reasoning and examples.

- Layers: REST handler -> `Command::try_new` -> service -> outbound port ->
  adapter. The command validates and resolves context with read-only access.
  The service owns the workflow and every business decision. The adapter runs
  SQL and transactions and makes no business decisions.
- For multi-aggregate writes, the service reads a snapshot, computes a pure,
  unit-tested plan, and the adapter applies the plan in one short transaction
  with set-based statements and an optimistic version check (see ADR 0006).
  Do not put last-write-wins, response building or other decisions in a
  repository.
- All value validation (ranges, enums, formats) lives in the application. Do
  not add database `CHECK` constraints for value ranges; keep structural
  constraints (keys, uniqueness, foreign keys, `NOT NULL`). When a migration
  must cope with legacy data, drop the malformed records instead of keeping a
  constraint.
- Every outbound network call has an explicit timeout, and any fan-out driven
  by a request has a documented cap. Route to a provider with an explicit
  `match` on the provider name rather than looping over implementations.
- Data cached from a provider records where it came from, and provider-confirmed
  data is never overwritten by client-supplied data.
- Migrations that are not merged may be edited in place; applied migrations stay
  immutable.
