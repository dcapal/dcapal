# ADR 0004: Unify Portfolio Asset allocation storage

- Status: accepted
- Date: 2026-09-12

## Context

Simple allocation stored its absolute target on `portfolio_asset`, while a
Strategic design would add another Portfolio Asset target table. Keeping those
paths separate makes every Portfolio read and write branch on several child
tables and leaves the source of truth unclear when a Portfolio changes mode.

## Decision

Store one Portfolio Asset allocation row for each
`(portfolio_id, portfolio_asset_id)` relationship in
`portfolio_asset_allocation`. The Portfolio owns one `allocation_mode`
(`simple` or `strategic`); the allocation row's `weight` is an absolute target
in Simple mode and a relative target within the effective Asset Class in
Strategic mode.

Store Strategic Asset Class targets in
`strategic_asset_class_targets`, keyed by `(portfolio_id, asset_class)`. A
class target belongs to the Portfolio and applies to every Strategic Portfolio
Asset with that effective class, so it does not use a nullable Portfolio Asset
foreign key. Derived Strategic absolute targets are calculated at read or
compute time and are never stored as another value.

Keep an explicit `strategic_allocation_present` marker on the Portfolio. A
`null` Strategic Allocation clears its rows; an object, including an empty
object, remains present even while Simple mode is active. This preserves the
full-document synchronization distinction between `null` and an empty draft.

The migration copies existing `portfolio_asset.target_weight` values into
`portfolio_asset_allocation` before removing the legacy column. The old v1
projection continues to expose absolute targets and resets a Portfolio to
Simple mode when it receives a legacy write.

## Consequences

Portfolio ownership is enforced by composite keys and foreign keys. Hard
deletes cascade through Portfolio Asset relationships and allocation rows, but
normal deletion remains a terminal soft-delete tombstone. Joined reads use one
allocation source for both modes, while Strategic class targets remain a
small, naturally scoped child table.
