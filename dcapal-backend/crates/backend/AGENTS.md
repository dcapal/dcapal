# Backend Rust guidance

Before changing Rust code, tests, or module documentation under this crate,
read:

- [../../CODING_STANDARDS.md](../../CODING_STANDARDS.md)
- [../../CONTEXT.md](../../CONTEXT.md)

Keep REST DTOs and HTTP conversion in `ports/inbound/rest`. Pass
transport-neutral application or domain types through commands, services, and
outbound ports. Keep SQLx and provider details in adapters.

Commands own semantic validation and read-only ownership/context lookups;
services own workflows; adapters own SQL and transactions. Prefer `thiserror`
for errors, `derive_more` for simple trait implementations, `mockall` for
test-only outbound-port mocks, and RAII guards for scoped validation paths.
Use bound SQL parameters for values and enum discriminants. Applied migration
files are immutable; evolve schemas with new migrations and keep compatibility
fixtures focused on supported production states.

Module layout: use real module directories (`portfolio/`, `sync_v2/`), not
`#[path]` attributes. Put shared REST error bodies in helpers rather than
repeating `V2ErrorResponse` literals, and put domain decision logic (planners,
validators) in `app/` where it is testable without a database. Follow
`../../CODING_STANDARDS.md` sections 2 to 4 for plan-then-apply writes,
validation placement and provider access.
