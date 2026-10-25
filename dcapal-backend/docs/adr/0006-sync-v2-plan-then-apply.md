# ADR 0006: Plan, then apply, for v2 Portfolio synchronization

- Status: accepted
- Date: 2026-10-25

## Context

The v2 synchronization repository method read and locked every Portfolio of
the user in a serializable transaction, decided last-write-wins, chose which
server-side Portfolios to echo, derived allocation weights per mode, wrote
Portfolio after Portfolio with several statements each, and built the response
by reading the written rows back. The business rules therefore lived in the
persistence adapter and could only be tested with PostgreSQL, the service was a
thin pass-through, and the number of statements grew with the number of
Portfolios and assets.

## Decision

The workflow is split by responsibility (ADR 0005 still describes the command
and asset onboarding):

- **Command** (`SyncPortfoliosV2Command::try_new`) validates, checks ownership
  facts and resolves asset references, performing reads only.
- **Service** (`PortfolioService::sync_portfolios_v2`) onboards the resolved
  assets once, then retries the whole cycle
  `load_sync_snapshot -> plan_sync -> apply_sync_plan`. After the retries are
  exhausted a conflict is reported as `409`, as before.
- **Planner** (`plan_sync` in `app/domain/portfolio_sync.rs`) is a pure
  function from a `SyncSnapshot` (stored Portfolios with their versions) and
  the request to a `SyncPlan`. It owns last-write-wins (a stored copy that is
  not older wins and is not written), tombstone semantics (terminal; unknown
  ids create a tombstone, known ids are marked), the echo of server-only
  Portfolios and deleted ids, the derived allocation rows per mode (Simple
  uses the asset target, Strategic the relative target keyed by asset
  reference, both defaulting to zero), the class targets to store, and the
  complete response (assets sorted by provider identifier and symbol, class
  targets by class, Strategic Allocation presence rules, `aclassOverride`
  verbatim, timestamps reduced to the store's microsecond precision). The
  response of a written Portfolio is built from the plan, not read back; the
  database keeps `NUMERIC` without scale, so storing and echoing the bound
  decimal are equivalent.
- **Adapter** (`SqlxPortfolioRepository`) only maps rows to the domain
  snapshot (strictly, reporting `CorruptData`) and executes a plan. It takes no
  synchronization decisions.

`apply_sync_plan` runs in one short transaction at the default read-committed
isolation, replacing the serializable transaction:

1. It locks, in identifier order, every row named by `plan.expected` (all
   snapshot rows and every submitted or deleted identifier) with
   `SELECT ... FOR UPDATE` and compares each row with the version the plan was
   derived from. A row of another user is an `OwnershipConflict`. A changed,
   vanished or unexpectedly present row of the user is
   `PortfolioRepositoryError::StaleSnapshot`, which is retryable, so the
   service re-reads and re-plans.
2. For identifiers expected to be absent there is nothing to lock, so a plain
   `INSERT` (no `ON CONFLICT`) is used and a unique violation of the Portfolio
   primary key is mapped to `StaleSnapshot` as well. Other unique violations
   stay permanent failures.
3. It then runs constant-size, set-based statements over all Portfolios of the
   plan at once, all values bound as `UNNEST` arrays: insert and update of
   Portfolios, one resolution of the shared asset identifiers for the union of
   references (`AssetNotOnboarded` if missing), one delete of relationships
   not retained, one upsert of relationships (existing identifiers kept, new
   UUIDv7 for new rows), one upsert of allocation rows, replacement of class
   targets, and tombstone update/insert. At most 11 statements run
   (empty ones are skipped), independent of the number of Portfolios or assets.

The version token (`PortfolioVersion`) is `(deleted, last_updated_at,
updated_at)`. No single column is reliable: the legacy v1 path never refreshes
`updated_at` and only flips `deleted` when deleting, while every v2 write
refreshes `updated_at` and moves `last_updated_at` forward. The snapshot is
read in one repeatable-read, read-only transaction so a Portfolio and its
class targets come from the same state.

## Consequences

- LWW, tombstones, echo rules and response shape are covered by pure unit tests
  without a database; integration tests cover the adapter (rollback on a stale
  version, concurrent creation, ownership, multi-Portfolio set-based writes,
  and equality between the planned response and a later read).
- Locks are held only for the apply transaction; the read happens before. The
  price is optimistic concurrency: a concurrent change to any Portfolio of the
  user between read and apply (including ones the request only echoes) makes
  the attempt stale and it is re-planned, up to the existing three attempts
  before `409`. This is the same exclusion the serializable transaction gave,
  now decided in application code.
- The planner is the single place that derives allocation weights and the
  response; the adapter's only knowledge of them is the rows it is told to
  write. The v1 synchronization and its repository paths are unchanged, and no
  migration is needed.
