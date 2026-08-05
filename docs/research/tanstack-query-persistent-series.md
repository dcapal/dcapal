# Findings: TanStack Query persistence for daily asset and FX series

Research date: 2026-08-05

## Context

- Ticket: [Research: Verify TanStack Query persistent series caching](https://github.com/dcapal/dcapal/issues/758)
- Parent product direction: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/742)

This is a research finding, not a product-code change. It uses the current repository source and official TanStack Query documentation/source only.

## Executive findings

1. **Use TanStack Query persistence for the browser cache, with IndexedDB behind the official `Persister` interface.** The smallest supported design is `@tanstack/react-query-persist-client` plus a custom IndexedDB persister following TanStack's documented `idb-keyval` example. `createAsyncStoragePersister` is also official, but it is a generic `getItem`/`setItem`/`removeItem` adapter and its default JSON serialization is not the best fit for large series. There is no dedicated official TanStack IndexedDB adapter in the official package set inspected here.

2. **Persist only successful asset/FX series queries.** Use `dehydrateOptions.shouldDehydrateQuery` to select the canonical series query family. Keep Zustand for client state and keep current prices out of this persisted series family. The persister stores a dehydrated QueryClient, not a separate time-series database.

3. **Set `gcTime` at least as high as `maxAge`, and choose the two separately from `staleTime`.** `staleTime` controls when a series becomes eligible for refetch; `gcTime` controls removal of inactive in-memory queries; `maxAge` controls whether the persisted snapshot is accepted on restore. TanStack warns that the default hydration `gcTime` is five minutes, while persisted `maxAge` defaults to 24 hours, so leaving `gcTime` at its default can discard a valid persisted snapshot early.

4. **Make the query key asset-series based, never Portfolio based.** A shared key should include every variable that changes the returned series—provider, canonical symbol or FX pair, source/quote currency, frequency, and the requested window—but not `portfolioId`. Every Portfolio that needs the same series then shares one QueryClient entry. The current repository already has one app-wide QueryClient and provider-price keys that are independent of Portfolio identity.

5. **Use direct asset-series requests and a bounded daily tail refresh.** The historical contract should be an asset-level request, not a request that embeds a whole Portfolio. On a daily view, refetch only the newest one-month tail, merge observations by canonical observation date into the long-range cached series, and update the canonical query with `setQueryData`. This is an application reconciliation policy; TanStack persistence does not merge ranges or deduplicate daily points for the application.

6. **A stale-cache refetch failure must preserve the last successful data and expose the failure.** TanStack Query reports the last successful `data` separately from `isRefetchError`/`error` and marks the result stale. The UI should show the series as stale or unavailable at its tail, retry later, and never replace a usable old series with an empty or invented series. A first-load failure with no cached data is a different state and should render as unavailable.

## Current repository source

### Query client and provider cache

- The frontend creates one shared `QueryClient` for the application. Its current defaults disable query retries and window-focus refetching. [Frontend QueryClient](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/queryClient.ts#L1-L23)
- Current price reads use a five-minute `staleTime`; Yahoo keys include provider, symbol, quote, and the sorted supported-currency list. DcaPal price keys come from the generated API client and include the asset and quote. These keys are already independent of Portfolio identity. [Provider query keys and options](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/priceProviders.ts#L113-L160)
- The current Yahoo price adapter calls a four-day chart window and selects the newest valid close; this is a live-price fallback path, not a historical daily-series cache. [Yahoo current-price adapter](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/priceProviders.ts#L42-L110)

### Existing historical boundary

- The backend exposes an asset-level `GET /assets/chart/{symbol}` route with `startPeriod` and `endPeriod`, and forwards it to the Yahoo adapter. The route has no Portfolio identifier or persistent-series contract. [Chart route](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/src/ports/inbound/rest/mod.rs#L171-L234)
- The generated client creates a query key from the asset path and the request-parameter object: `['/assets/chart/${symbol}', params]`. This is a useful generated transport key, but the future canonical series key should be a deliberate domain key shared by all Portfolio consumers and should include the normalized series identity and frequency. [Generated chart query key](https://github.com/dcapal/dcapal/blob/master/packages/api-client/src/gen/index.ts#L243-L275)
- The current chart response keeps only Yahoo `meta.currency` and close arrays. It does not preserve observation timestamps in the application-facing type, so a durable daily-series contract must add dated observations before reconciliation can be reliable. [Yahoo chart proxy types](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/src/ports/inbound/rest/proxy_types.rs#L17-L49)
- The frontend domain explicitly separates the historical Portfolio model series from current Portfolio value. A cloned Portfolio recomputes historical performance from asset time series rather than copying history. [Frontend domain context](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/CONTEXT.md#L1-L20) and [historical performance ADR](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/docs/adr/002-use-normalized-model-history.md#L1-L20)

## Official TanStack Query findings

### Smallest persistence design

TanStack's v5 persistence plugin is `@tanstack/react-query-persist-client`. It defines a `Persister` with three operations: `persistClient`, `restoreClient`, and `removeClient`. `persistQueryClient` restores the snapshot and subscribes to later cache changes; `PersistQueryClientProvider` additionally prevents query fetches racing with asynchronous restore and exposes restore lifecycle callbacks. [Official persistence guide](https://tanstack.com/query/v5/docs/framework/react/plugins/persistQueryClient#persisters) and [provider guidance](https://tanstack.com/query/v5/docs/framework/react/plugins/persistQueryClient#persistqueryclientprovider)

For this repository, the smallest production shape is:

```ts
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { get, set, del } from "idb-keyval";

const persister = {
  persistClient: (client) => set("dcapal-series", client),
  restoreClient: () => get("dcapal-series"),
  removeClient: () => del("dcapal-series"),
};
```

The snippet is an application sketch, not an implementation. TanStack's official example uses the same `idb-keyval` operations and says IndexedDB is preferable to Web Storage for larger data because it stores more than 5 MB and does not require serialization. [Official IndexedDB persister example](https://tanstack.com/query/v5/docs/framework/react/plugins/persistQueryClient#building-a-persister)

`createAsyncStoragePersister` is a supported alternative when a storage object implements `getItem`, `setItem`, and `removeItem`; the official package defaults to JSON serialization and one-second write throttling. An IndexedDB wrapper can implement that interface, but a direct `Persister` is smaller and avoids treating IndexedDB as string storage. [Official async-storage adapter](https://tanstack.com/query/v5/docs/framework/react/plugins/createAsyncStoragePersister#options)

The official package tree contains the persistence core and sync/async storage persister packages, while the IndexedDB example is documented as a custom persister. This supports using `idb-keyval` as a small direct dependency, not adding a speculative adapter abstraction. [TanStack persistence package](https://github.com/TanStack/query/tree/main/packages/react-query-persist-client) and [TanStack async-storage package](https://github.com/TanStack/query/tree/main/packages/query-async-storage-persister)

### Persist a subset, not the whole application cache

`dehydrate` persists successful queries by default and accepts `shouldDehydrateQuery` to decide which queries are included. The series persistence subscription should select only the canonical asset/FX series family. This keeps asset metadata, search results, authentication-adjacent data, and live current prices outside the large persisted payload. [Official dehydration reference](https://tanstack.com/query/v5/docs/framework/react/reference/hydration#dehydrate)

### `staleTime`, `gcTime`, and persisted `maxAge`

These values have different meanings:

| Setting | Meaning for this design | Starting policy |
| --- | --- | --- |
| `staleTime` | How long a successfully fetched series is considered fresh. A stale query may refetch on mount, focus, or reconnect according to the other refetch settings. | Daily series: short enough to allow the one-month-tail policy after a day boundary; the exact duration is a product decision. Do not use `Infinity` for mutable daily data. |
| `gcTime` | How long an inactive query remains in memory before garbage collection. It does not mean “how long the IndexedDB record is valid.” | At least the persisted `maxAge`; for a daily series, a multi-day or longer value is reasonable if the browser timer limit is handled. |
| `maxAge` | Maximum age of the persisted snapshot at restore. An older snapshot is silently discarded and `removeClient()` is called for expired/busted/error/empty persisted data. | Match the offline continuity requirement, such as several days, and let the daily tail reconciliation revalidate the end. |

TanStack's current defaults are `staleTime: 0` and inactive-query `gcTime: 5 minutes`; it documents `staleTime` as freshness and `gcTime` as inactive-cache lifetime. [Official defaults](https://tanstack.com/query/v5/docs/framework/react/guides/important-defaults) and [useQuery option reference](https://tanstack.com/query/v5/docs/framework/react/reference/useQuery#options)

For persistence, TanStack specifically says that hydration uses a five-minute default `gcTime` when it is not overridden, and recommends `gcTime >= maxAge`; otherwise garbage collection can remove data earlier than the persisted max age. It also documents a roughly 24-day JavaScript timer limit unless the timeout provider is replaced. [Official persistence lifetime guidance](https://tanstack.com/query/v5/docs/framework/react/plugins/persistQueryClient#how-it-works)

The persisted `maxAge` check is snapshot-wide. It is not a per-observation or per-query freshness check. Therefore a snapshot can be accepted while an individual daily series is stale; `staleTime`, refetch policy, and the tail reconciliation must handle that case.

### Canonical keys and Portfolio reuse

TanStack requires serializable array query keys and says every variable used by the query function that changes the result belongs in the key. Object properties are hashed deterministically, but array order matters. [Official query-key rules](https://tanstack.com/query/v5/docs/framework/react/guides/query-keys)

Recommended domain shape:

```ts
["asset-series", {
  provider: "yahoo",
  symbol: "VWCE.DE",
  quote: "EUR",
  frequency: "daily",
  from: "2025-08-05",
  to: "2026-08-05",
}]
```

The key must not contain `portfolioId`, target weights, quantity, or other Zustand state. Those values affect the derived Portfolio model series, not the underlying asset/FX observations. If two Portfolios ask for the same normalized identity and window, they share the cached and persisted query. If the request window changes, it is a different query and must be reconciled or fetched explicitly.

### Direct-depth requests and daily one-month-tail reconciliation

The current repository already has a direct asset chart boundary, so the smallest compatible design is an asset-series query function that calls that boundary directly. A Portfolio view composes the same asset-level query options for each holding and separately queries any required FX series. It does not issue a Portfolio-shaped historical request or include the Portfolio in the series key.

For a daily series with a long requested range:

1. Restore the canonical long-range query from IndexedDB.
2. If its daily data is stale or the day boundary has passed, issue one direct request for `[today - 1 month, today]` for that same series identity.
3. Merge returned observations by canonical UTC/business observation date. The tail replaces rows for dates it covers; older rows remain unchanged. Sort and deduplicate before storing.
4. Write the merged result to the same long-range query key with `queryClient.setQueryData`, preserving the last successful data when the tail request fails.
5. Persist the changed cache through the persistence subscription. Do not create one persisted query per Portfolio.

This tail policy is an application rule inferred from the repository's asset-level chart boundary and the product's daily-series requirement. TanStack provides cache identity, hydration, refetch, and `setQueryData`; it does not know that two responses are date ranges of one logical series and will not merge them automatically. The merge must use a stable observation date and must not use fetch time as the row key.

### Stale-cache failure behavior

TanStack's `useQuery` reference defines `data` as the last successfully resolved data, `isRefetchError` as a failure during refetch, and `isStale` as true when data is invalidated or older than `staleTime`. Thus a failed tail refresh does not require discarding the old series. The UI can render the old series with an explicit stale/error state and retry later. [Official `useQuery` result reference](https://tanstack.com/query/v5/docs/framework/react/reference/useQuery#returns)

The persistence layer has a different failure rule: if restoring data finds an expired snapshot, a buster mismatch, an error, or an empty result, TanStack calls `removeClient()` and discards that persisted cache. That is correct for a corrupt or too-old snapshot, but it is not a reason to delete a valid in-memory series after a provider tail failure. [Official persistence removal rules](https://tanstack.com/query/v5/docs/framework/react/plugins/persistQueryClient#removal)

The repository currently sets `retry: false`, so a future series query must either opt into bounded retries for transient provider failures or keep the current no-retry behavior and expose the stale state immediately. In either case, the result contract should distinguish: no cache and failed initial request; cached but stale and failed refresh; and cached, refreshed, and current. [Current repository retry default](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/queryClient.ts#L9-L20)

## Recommended smallest design

| Concern | Recommendation |
| --- | --- |
| State ownership | Zustand for Portfolio/client state; TanStack Query for asset/FX server state; live current prices remain separate. |
| Persistence packages | `@tanstack/react-query-persist-client` plus a small custom IndexedDB `Persister` using the official `idb-keyval` pattern. Add `@tanstack/query-async-storage-persister` only if its generic storage interface is useful elsewhere. |
| Provider | `PersistQueryClientProvider`, so async IndexedDB restore cannot race query mounts. |
| Dehydration | Include only successful canonical asset/FX series queries with `shouldDehydrateQuery`. |
| Key | `asset-series` + provider + canonical symbol/pair + source/quote + frequency + requested range; never Portfolio id. |
| Daily refresh | Refetch the latest one-month tail, date-merge it into the long-range query, and keep the old series on failure. |
| Freshness | Set daily `staleTime` to permit daily reconciliation; set `gcTime >= maxAge`; choose `maxAge` for offline continuity. |
| Failure | Preserve last successful data on refetch failure and expose stale/error metadata; discard only invalid/expired persisted snapshots. |

## Remaining implementation decisions

- Canonical symbol and FX-pair normalization, including provider and exchange identity.
- The exact UTC/business-date rule and whether the newest daily point is allowed before the provider's daily candle is final.
- The desired offline continuity window, which determines `maxAge` and the matching `gcTime`.
- Whether a long-range query uses one bounded response or pages/chunks the series; large payloads should be measured before choosing one giant query.
- A provider retry policy, because the current repository-wide `retry: false` is not enough for reliable daily-tail maintenance.
