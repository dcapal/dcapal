# ADR 0003: Separate shared asset metadata from Portfolio Asset relationships

- Status: accepted
- Date: 2026-08-26

## Context

The previous Portfolio Asset migration treated `(portfolio_id, provider,
symbol)` as the complete identity. That preserved separate rows in different
Portfolios, but it also repeated immutable asset metadata. The product needs
to reuse the same provider-bound asset in several Portfolios without merging
their quantities, targets, prices, basis costs, or fee policies.

## Decision

Store shared asset metadata in `assets_data`. Its identity is the numeric
provider and upper-case symbol, enforced by `UNIQUE (provider, symbol)`.
`assets_data` owns the UUIDv7 identifier, provider, symbol, immutable name and
currency, shared default Asset Class, and timestamps.

Keep `portfolio_asset` as the Portfolio Asset relationship table. A row links
one Portfolio to one `assets_data` row and owns the quantity, target weight,
manual price, average buy price, fee fields, timestamps, and nullable
`asset_class_override`. Enforce `UNIQUE (portfolio_id, assets_data_id)`.

An absent Asset Class override means that the Portfolio inherits the shared
default. A present override is Portfolio-specific. Portfolio synchronization
may create a missing shared record, but it does not replace an existing
record's name, currency, or shared default class. Later writes return the
canonical stored metadata and update only the relationship row.

The forward-only migration after `20260814000000` backfills shared records
from the oldest surviving row, reports duplicate and metadata-conflict
counts with `RAISE NOTICE`, preserves relationship manual prices, converts
class differences into overrides, removes only duplicate relationships inside
one Portfolio, and rewrites both table identities with PostgreSQL 18
`uuidv7()`. The down migration is for development and test reset only.

The current v1 synchronization payload remains unchanged: it is a legacy flat
projection that returns the effective class and existing provider/class
aliases. The v2 synchronization contract is out of scope for this decision
and remains owned by issue #773.

## Consequences

Cloning, importing, and synchronizing a Portfolio must get or create the
shared asset by `(provider, upper-case symbol)` and create a new relationship
for each Portfolio. Joined reads are required to return both shared metadata
and relationship values.

PostgreSQL 18 is a deployment requirement for this migration because native
UUIDv7 is used for new and rewritten identities. Production recovery is
forward-only from the pre-migration backup; the development down migration is
not a production rollback strategy.
