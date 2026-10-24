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
- Unknown providers are validation errors at their exact field path; they fail
  the request rather than being dropped.
- `SyncPortfoliosV2Command::try_new` validates, checks Portfolio ownership,
  and resolves every distinct reference through `AssetService::resolve`
  (shared directory first, then the provider; see "Metadata provenance").
  Unknown assets become
  validation errors at every offending `portfolios[i].assets[j].asset`;
  provider outages become `502 provider_unavailable`. The command performs
  reads only.
- The service onboards the resolved assets (`AssetService::onboard`:
  `upsert_resolved` and `confirm`, idempotent) before the retried serializable
  transaction, so no provider or network I/O happens while rows are locked.
- The repository looks up the already-stored `assets_data` IDs inside the
  transaction, updates relationships matching by shared asset, inserts new
  ones with fresh UUIDv7 IDs, and deletes the rest. Switching an asset is a
  delete plus an insert. No schema change is needed; allocation rows remain
  keyed by the internal relationship ID.
- `aclassOverride` is stored and returned verbatim. It is no longer reduced to
  `null` when it equals the shared class, because the response no longer
  carries the shared class to compare against.

## Metadata provenance

`assets_data.metadata_source` (`SMALLINT NOT NULL DEFAULT 0`, checked to be 0
or 1) records where a row's name, currency and class came from: `0` is
client-supplied (the v1 synchronization path, or any row that predates the
column) and `1` is confirmed by the provider. The default keeps v1 code and
older backend binaries, whose INSERTs omit the column, working. The v1
`upsert_asset_data` still creates rows with source `0` and never overwrites.

Without provenance a first-writer-wins directory let a v1 client choose the
metadata that v2 then served to everyone forever. `AssetService::resolve`
therefore treats rows by source:

- Source `1`: used as is, no provider call. The first provider resolution
  wins and is never overwritten.
- Source `0` ("unverified"): refreshed on use. The provider is asked and an
  answer replaces the cached metadata (`upsert_resolved`, which overwrites only
  rows still at source `0` and marks them `1`). If the provider answers
  "unknown" or fails, the cached metadata is used, the sync does not fail and
  nothing is marked, so a later sync retries.
- Kraken exception: its catalog only yields the symbol as name, which is worse
  than a v1 name. An unverified Kraken row keeps its cached metadata and is
  only marked confirmed (`confirm`) when the catalog knows the symbol.
- No row: provider lookup, as before; unknown is a validation error and an
  outage is `502`.

Refresh lookups share the 50-lookup per-request budget but are best effort.
References with no row always come first and are never starved; only the
first `50 - unknown` unverified references in request order are refreshed in
one request, and the rest keep cached metadata until later syncs. The
`GET /markets/v2/assets/lookup` endpoint stays read-only and returns cached
metadata whatever its source; the source is not exposed.

## Consequences

- Client metadata can no longer poison the shared directory: v2 never trusts
  it without provider confirmation, and confirmed metadata is never replaced.
- Placeholder parking, explicit-ID/identity-fallback matching, asset-ID
  ownership checks and the unsupported-provider filtering were removed.
- A request referencing an asset for the first time may call a provider, so
  sync latency and availability depend on providers for new assets.
- One synchronization may send at most 50 distinct references unknown to the
  directory to providers (`MAX_NEW_ASSETS_PER_RESOLUTION`); beyond that it
  fails with a `400` validation error at `portfolios` before any provider
  call. References with a directory row do not count towards that limit, but
  unverified rows consume whatever budget the unknown ones leave (see
  "Metadata provenance"). Only newly resolved or refreshed assets are written
  to the directory.
- Yahoo lookups time out after 10 seconds (reported as `502`); HTTP 400, 404
  and 422 mean the symbol is unknown, while 429 and 5xx mean the provider is
  unavailable. The currency is kept exactly as Yahoo returns it so pence codes
  such as `GBp` are not turned into `GBP`.
- Kraken has no display name for cryptocurrencies, so the symbol itself is
  stored as the placeholder name and trading currency. An empty Kraken
  catalog (cold start) is reported as `502` rather than as an unknown symbol.
- Clients must use `GET /markets/v2/assets/lookup` to read asset metadata.
- Assets in Portfolios skipped as stale or tombstoned are still resolved.
- This supersedes the full-asset-data and client-generated relationship ID
  parts of #745 and #771. The v1 contract and repository paths are unchanged.
