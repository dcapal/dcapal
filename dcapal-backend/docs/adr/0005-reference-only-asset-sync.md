# ADR 0005: Reference-only assets in v2 Portfolio synchronization

- Status: accepted
- Date: 2026-10-24

## Context

The first v2 synchronization design (#745, #771) had clients send full shared
asset data (name, currency, class) and client-generated UUIDv7 relationship
IDs. That forced the server to trust client metadata, upsert `assets_data` for
every request, track relationship identity by ID with fallback matching by
asset, and "park" rows behind placeholder assets to swap identities without
violating `UNIQUE (portfolio_id, assets_data_id)`. Unsupported providers were
silently dropped, which required retained-index bookkeeping in validation.
The revision recorded in the "Contract revision" comment of issue #771
replaces this with reference-only assets.

## Decision

- A v2 asset is `{provider, symbol}` (provider `yf` or `kraken`). Requests and
  responses carry only that reference plus Portfolio-specific values; Strategic
  asset targets reference assets the same way. Relationship identity is
  `(Portfolio, provider, symbol)`; relationship IDs stay internal.
- Unknown providers are validation errors at their exact field path.
- `SyncPortfoliosV2Command::try_new` validates, checks Portfolio ownership,
  and resolves every distinct reference through `AssetService::resolve`
  (shared directory first, then the provider). Unknown assets become
  validation errors at every offending `portfolios[i].assets[j].asset`;
  provider outages become `502 provider_unavailable`. The command performs
  reads only.
- The service onboards the resolved assets (`AssetService::onboard`,
  idempotent, first writer wins) before the retried serializable transaction,
  so no provider or network I/O happens while rows are locked.
- The repository looks up the already-stored `assets_data` IDs inside the
  transaction, updates relationships matching by shared asset, inserts new
  ones with fresh UUIDv7 IDs, and deletes the rest. Switching an asset is a
  delete plus an insert. No schema change is needed; allocation rows remain
  keyed by the internal relationship ID.
- `aclassOverride` is stored and returned verbatim. It is no longer reduced to
  `null` when it equals the shared class, because the response no longer
  carries the shared class to compare against.

## Consequences

- Client metadata can no longer poison the shared directory.
- Placeholder parking, explicit-ID/identity-fallback matching, asset-ID
  ownership checks and the unsupported-provider filtering were removed.
- A request referencing an asset for the first time may call a provider, so
  sync latency and availability depend on providers for new assets.
- Clients must use `GET /markets/v2/assets/lookup` to read asset metadata.
- Assets in Portfolios skipped as stale or tombstoned are still resolved.
- This supersedes the full-asset-data and client-generated relationship ID
  parts of #745 and #771. The v1 contract and repository paths are unchanged.
