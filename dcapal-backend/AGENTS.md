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
