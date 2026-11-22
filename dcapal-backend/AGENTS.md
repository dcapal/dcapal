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
- Enums expose their names through `strum` (`IntoStaticStr`, `EnumString`,
  `EnumIter`, `Display`) and stored codes through `sqlx::Type`; do not repeat
  provider, asset class or fee type names as string literals (legacy v1 alias
  tables and wire-contract tests are the exception).
- All Yahoo traffic goes through the rate-limited `YahooProvider::get`; user
  facing proxies answer 429 when saturated, workers and lookups wait. Custom
  query shapes get an axum extractor (see `rest/extract.rs`), not
  `Query<Vec<(String, String)>>`.
- Migrations that are not merged may be edited in place; applied migrations stay
  immutable.
- REST layout: handlers live under `rest/handlers/`, one module per endpoint
  nested like the URL path; their DTOs are mirrored under `rest/types/`; common
  REST modules (`mod.rs` router wiring, `error.rs`, `extract.rs`, `openapi.rs`)
  stay in `rest/`. See the "REST module layout" section of `CODING_STANDARDS.md`.

## Metrics conventions

Prometheus metrics use the `metrics` macros; names and label vocabularies live
in `crates/backend/src/app/infra/stats.rs` (prefix `dcapal_`). Describe each
family in `init_metrics()` (`src/lib.rs`).

| Family | Labels and values |
| --- | --- |
| `dcapal_requests_total` | `path` (route template), `status` (`2xx`, `4xx`, `5xx`, `other`) |
| `dcapal_provider_requests_total` | `provider` (`yahoo`, `kraken`, `coin_market_cap`), `op` (`lookup`, `price`, `proxy`), `outcome` (`ok`, `not_found`, `rate_limited`, `timeout`, `circuit_open`, `error`) |
| `dcapal_provider_request_seconds` | `provider`, `op` (no `outcome`); seconds, includes limiter wait |
| `dcapal_yahoo_limiter_wait_seconds` | no labels; seconds spent waiting for a local Yahoo request slot |
| `dcapal_yahoo_limiter_rejections_total` | no labels; Yahoo calls rejected before reaching the provider |
| `dcapal_yahoo_upstream_rate_limits_total` | no labels; HTTP 429 responses returned by Yahoo |
| `dcapal_sync_stage_seconds` | `stage` (`resolve`, `onboard`, `load`, `apply`); seconds |
| `dcapal_sync_events_total` | `event` (`stale_retry`, `conflict`, `pull_only`, `new_assets_resolved`, `refresh_attempted`, `too_many_new_assets`) |

- Labels are low cardinality: never a ticker, symbol, user id or raw URL. Use
  `&'static str` values from strum enums, not literals at call sites.
- Emit where the fact is known: HTTP metrics in the stats middleware, provider
  metrics in outbound adapters (`observe_provider`), sync metrics in services
  (`SyncStage::timer`, `record_sync_event`). Never emit the same call twice.
- `*_seconds` families are exported as summaries (the exporter default, no
  explicit buckets): per label set they emit 7 quantile series (`0`, `0.5`,
  `0.9`, `0.95`, `0.99`, `0.999`, `1`) plus `_sum` and `_count`, i.e. 9 series.
  Quantiles are computed per instance and cannot be aggregated across replicas;
  aggregate only `_sum` and `_count`. Keep label sets small. `dcapal_latency_summary`
  is a summary in microseconds.
