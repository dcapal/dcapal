# Research findings: historical price series, daily maintenance, and rate-fetch feasibility

Status: resolved research for [ticket #746](https://github.com/dcapal/dcapal/issues/746).

- Map context: [Wayfinder map #742](https://github.com/dcapal/dcapal/issues/742)
- Proposal context: [Discussion #441](https://github.com/dcapal/dcapal/discussions/441)
- Repository snapshot: [`f652260de11204971f95b6235ffc40cf812911a1`](https://github.com/dcapal/dcapal/tree/f652260de11204971f95b6235ffc40cf812911a1)
- Date: 2026-08-03
- Scope: research only. No product code was changed.

The repository had no established research-note directory. This report is therefore stored in `docs/research/` as the closest shared documentation location.

## Conclusion

The feature is feasible, but the proposal does not fit the current backend as written. The current service can fetch a latest price and run periodic loops. It cannot yet retain a historical series, track a durable fetch job, expose a normalized stored series, or explain partial coverage to a client.

Recommended MVP shape:

1. Keep PostgreSQL as the durable store for series observations and job state. Keep Redis for the existing market catalog and a fast latest-price cache.
2. Run a fetch worker and a maintenance worker in the backend process, but make their work durable and retryable. A PostgreSQL job table is the simplest source of truth. Redis Streams are also feasible because the repository already runs Redis Stack; if used, keep observation and job outcome metadata in PostgreSQL.
3. Treat Kraken as a viable crypto source for bounded daily and weekly backfills, subject to its 720-row OHLC limit and rate limits. Do not treat the existing Yahoo chart proxy as a stable long-term provider contract until its data rights, availability, and supported range are verified.
4. Ingest Fed H.10 and ECB EXR rates as separate daily-reference series. They are not live market prices and need observation-date, publication/release, and fetch timestamps.
5. Return normalized, timestamped samples from a storage-backed endpoint. Return actual observations only; let the client forward-fill after the first observation and show a pending, partial, or stale state when coverage is incomplete.

The map already settles the MVP sampling windows: daily for `1Y`, weekly for `3Y`, `5Y`, and `Max`. Monthly samples are out of scope even though the older proposal mentions monthly aggregation.

## Context and identity

The backend glossary distinguishes a market price from a general conversion rate. A market price belongs to a base/quote market and has a timestamp; a conversion rate may be derived through intermediary markets. The relevant terms and boundaries are in the [backend context](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/CONTEXT.md).

The proposal says that a series should be keyed by a ticker and the follow-up discussion adds an important constraint: a ticker is tied to a provider and, for listed instruments, an exchange. The discussion records Kraken for crypto and Yahoo Finance for other assets. A durable series key must therefore include at least `provider`, provider instrument/ticker, and exchange or market where the provider exposes one; a ticker string alone is not enough. See [the proposal](https://github.com/dcapal/dcapal/discussions/441), [the exchange/ticker comment](https://github.com/dcapal/dcapal/discussions/441#discussioncomment-10685927), and [the provider-bound identity clarification](https://github.com/dcapal/dcapal/discussions/441#discussioncomment-10718951).

## What the repository does today

| Area | Evidence | Exact current behavior |
| --- | --- | --- |
| Current price refresh | [`PriceUpdaterWorker`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/workers/price_updater.rs#L18-L85) and [`DcaServer::start`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/lib.rs#L225-L250) | Loads every known market, fetches one latest value, stores one `Market` snapshot in Redis, and sleeps five minutes between sweeps. It waits 100 ms between markets. |
| Market discovery | [`MarketDiscoveryWorker`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/workers/market_discovery.rs#L21-L148) | Polls an in-process loop, checks a Redis date marker, and attempts discovery roughly daily. There is no durable maintenance queue. The marker is updated after the attempt even when discovery returned an error. |
| Storage | [`MarketDataRepository`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/repository/market_data/mod.rs#L10-L79) and [`RedisMarket`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/repository/market_data/redis_market.rs#L12-L44) | Assets and markets are Redis hashes. A market stores an optional latest `Price`; there is no observation table, time-range query, job table, or historical retention policy. |
| Price timestamp | [`Price` and `Market`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/domain/entity.rs#L80-L228) and [`fetch_market_price`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/domain/market_data_utils.rs#L7-L35) | The domain has a timestamp and a five-minute freshness rule, but the fetched value is wrapped with `Utc::now()` after the provider call. The source candle timestamp is not retained. |
| Existing historical route | [`/assets/chart/{symbol}`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/inbound/rest/mod.rs#L168-L230) and [`YahooProvider::chart`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L120-L157) | Accepts `startPeriod` and `endPeriod`, then forwards a raw Yahoo request with a fixed `interval=5m`. It is not backed by stored data and has no `timeframe`, job status, or normalized sample contract. |
| Existing current-price route | [`/price/{asset}`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/inbound/rest/mod.rs#L233-L288) | Returns a conversion rate from the current market cache. It is not a historical-series endpoint. |
| PostgreSQL | [SQLx persistence ADR](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/docs/adr/0001-sqlx-persistence-and-migrations.md) and [current migrations](https://github.com/dcapal/dcapal/tree/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/migrations) | PostgreSQL is the durable store for users, portfolios, and portfolio assets. The migrations contain no market-data or job tables. |
| Observability | [`infra::stats`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/infra/stats.rs#L21-L66) and [`init_metrics`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/lib.rs#L264-L300) | Prometheus metrics cover visitors, HTTP requests, endpoint latency, and imported portfolios. Worker failures are log messages; there are no provider, job, retry, backlog, coverage, or freshness metrics. |

## Provider and rate feasibility

### Kraken: viable for crypto, with a bounded backfill

Kraken’s official `GET /0/public/OHLC` endpoint accepts a pair, an interval in minutes, and a `since` timestamp. Its documented intervals include `1440` (daily) and `10080` (weekly). The endpoint returns at most 720 recent entries, regardless of `since`, and the last entry is the current, not-yet-committed timeframe. See the [official OHLC documentation](https://docs.kraken.com/api-reference/market-data/get-ohlc-data).

This is enough for the MVP’s recent daily window and a substantial weekly window, but it is not an unlimited historical source. A daily 1Y request fits under 720 rows. A weekly request fits roughly 13.8 years under the same limit; anything older needs another source or an explicit “Max available” boundary. The ingestion code must drop the uncommitted candle for historical samples and must page or split requests only within the provider’s documented limit.

The current adapter requests only `5` and `1440`, asks for a recent window, selects one close, and discards the remaining candles. See [`KrakenProvider::fetch_price`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/kraken.rs#L86-L169) and [`get_kraken_api_periods`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/kraken.rs#L531-L535). Weekly fetching, bulk persistence, confirmed-candle filtering, and gap detection are therefore missing.

For a current crypto price, Kraken also documents a separate [Ticker endpoint](https://docs.kraken.com/api-reference/market-data/get-ticker-information). The current code uses a recent OHLC close for both current and daily fallback prices. A future current-price adapter should choose the ticker or another explicit current-price source and document what `observed_at` means when the provider does not return a point timestamp.

Kraken’s official [Spot REST rate-limit guide](https://docs.kraken.com/exchange/guides/rest/ratelimits) documents a tiered call counter and returns `EAPI:Rate limit exceeded` or `EService: Throttled: [UNIX timestamp]` when limits are exceeded. The current 100 ms inter-market sleep is not a provider-aware shared limiter. The fetch worker needs one limiter per provider, handling concurrency, the provider’s throttle timestamp, and backoff across all jobs rather than per request only.

### Yahoo Finance: technically callable, contract not established

The existing adapter calls `query1.finance.yahoo.com/v8/finance/chart/{symbol}` with `period1`, `period2`, and either `5m` or `1d`, and sends a browser-like user agent. Its public chart route fixes the interval to `5m`. See [`YahooProvider::fetch_price`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L25-L118) and [`YahooProvider::chart`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/yahoo.rs#L120-L157).

This proves that the current process can make a chart request; it does not establish a first-party, durable API contract for stored historical data. The response model used by the route exposes only `currency` and close arrays, not timestamps, in [`proxy_types.rs`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/inbound/rest/proxy_types.rs#L19-L49). Before using Yahoo as the canonical long-term equity source, the product must confirm supported ranges, rate limits, redistribution rights, adjusted-versus-unadjusted close semantics, exchange calendars, and an owned provider contract. Until then, Yahoo is a best-effort adapter, not a reliable guarantee for `3Y`, `5Y`, or `Max`.

### CryptoWatch: legacy adapter, not a new storage contract

The repository also has a CryptoWatch adapter with `after`, `before`, and `periods` parameters. It fetches 5-minute or daily OHLC and has one special second request with an API key after a 429. It does not persist a batch or implement delayed retries. See [`cw.rs`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/cw.rs#L104-L178).

This source duplicates Kraken for the configured crypto exchange and its current official contract was not verified in this pass. Do not build the new historical-storage design around it without revalidating availability, limits, and terms.

### Federal Reserve H.10: daily observations, weekly publication

The Federal Reserve Board’s [H.10 release page](https://www.federalreserve.gov/releases/h10/default.htm) says that it releases daily bilateral exchange rates for the previous business week on Mondays at 4:15 p.m. (or the next business day when Monday is a holiday). The [historical country-data page](https://www.federalreserve.gov/Releases/H10/hist/) describes the values as noon New York buying rates and lists the available currencies. The page exposes RSS, XML, and data-download links; the current XML download is [`FRB_h10_xml.zip`](https://www.federalreserve.gov/releases/h10/data/FRB_h10_xml.zip).

H.10 is feasible for a daily business-day series, but it is not a live or seven-day-a-week feed. The maintenance job should poll after the expected Monday release, backfill the previous business week, treat `ND` as missing rather than zero, and retain both the observation date and the release/fetch time. If the product needs a live Fed policy rate rather than an FX reference rate, that is a separate source decision and is outside this market-price series contract.

### European Central Bank: strong fit for daily reference FX

The ECB Data Portal provides an official [SDMX 2.1 REST API](https://data.ecb.europa.eu/help/api/overview). Its [data API documentation](https://data.ecb.europa.eu/help/api/data) supports `startPeriod`, `endPeriod`, `updatedAfter`, `lastNObservations`, `includeHistory`, and JSON or CSV output. The official [examples](https://data.ecb.europa.eu/help/api/data-examples) use `EXR/D.USD.EUR.SP00.A` for a bounded daily USD/EUR series.

This is a good fit for daily ECB reference rates and for retryable, bounded maintenance. The data is an observation series, not an intraday quote. The API’s `updatedAfter` and `includeHistory` options also mean that revisions are possible; the storage contract must decide whether to keep only the latest value per observation date or retain source revisions.

The live endpoint was checked during this research: the bounded ECB CSV request returned HTTP 200 and daily rows for the requested dates. A single Kraken daily OHLC request and the Fed H.10 page also returned HTTP 200. These checks show reachability, not a substitute for provider limits or a production reliability test.

## Storage decision and TimescaleDB fit

The proposal’s TimescaleDB idea is technically sound for raw observations and rollups: Timescale describes a hypertable as a PostgreSQL table partitioned by time and describes continuous aggregates as incrementally refreshed materialized data. Its [continuous aggregate guide](https://docs.timescale.com/use-timescale/latest/continuous-aggregates/about-continuous-aggregates/) supports layered aggregates, and the [creation guide](https://docs.timescale.com/use-timescale/latest/continuous-aggregates/create-a-continuous-aggregate/) requires a `time_bucket` and an explicit refresh policy.

However, it is not a safe default for the repository’s managed PostgreSQL target. The local development compose file uses [`timescale/timescaledb-ha:pg17`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/docker-compose.yml#L20-L37), but that image does not prove that the deployed Supabase project exposes the same extension. Supabase’s official [TimescaleDB migration guide](https://supabase.com/docs/guides/database/migrating-to-pg-partman) says that starting with PostgreSQL 17, Supabase projects do not have `timescaledb` available and recommends native PostgreSQL partitioning or `pg_partman` as a migration path. Supabase’s [extension list](https://supabase.com/docs/guides/database/extensions) also marks TimescaleDB as deprecated.

Recommendation: use standard PostgreSQL tables and SQLx migrations for the first implementation, with a time column, a unique series/time key, and indexes for `(series_id, observed_at)` and range reads. Native range partitioning is enough for the MVP’s daily and weekly volume. Add TimescaleDB only if deployment is explicitly pinned to a supported self-hosted or managed version. Weekly rows may be derived from daily observations in SQL or in application code; a Timescale continuous aggregate is an optional optimization, not a requirement.

Redis remains useful for the existing hot market catalog and latest-price cache. Redis Streams are a possible queue: the official [Streams documentation](https://redis.io/docs/latest/develop/data-types/streams/) describes consumer groups, acknowledgements, pending entries, and claiming work after a consumer fails. The current Redis Stack image is compatible with that basic model. A stream alone does not give the product a durable observation schema or a clear job outcome history, so it should not replace PostgreSQL for those records.

Supabase also supports [Cron](https://supabase.com/docs/guides/cron), which records scheduled job runs and can execute SQL, database functions, or HTTP requests. It can schedule a daily maintenance trigger, but it does not remove the need for a provider-aware Rust fetcher, rate limiter, retry policy, and idempotent observation writes.

## Exact gaps and the contract needed to close them

### 1. Background jobs

**Present:** two in-process loops are spawned when the HTTP server starts. There is no fetch queue, durable job state, ownership/claim protocol, or restart recovery. The daily market-discovery marker is written after a failed attempt, so a failure can suppress the next daily attempt.

**Gap to close:** add a durable job record or a queue plus durable job outcome. A job should be keyed by series, provider, timeframe, and requested range so repeated portfolio requests are deduplicated. Minimum states are `queued`, `running`, `retry_wait`, `succeeded`, and `failed`; minimum fields are `attempts`, `available_at`, `started_at`, `finished_at`, `last_error`, and the requested `from`/`to` range. A worker must claim work, renew or recover stale claims, and write observations idempotently.

**MVP behavior:** when a requested series is absent or incomplete, return a pending or partial result with a job identifier. Do not make the request wait for a multi-year provider fetch.

### 2. Daily and weekly samples

**Present:** the domain enum contains only 5-minute and daily OHLC frequencies, and each provider adapter returns one latest close. There is no weekly frequency, no batch insert, and no product-window policy. See [`OHLCFrequency`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/domain/entity.rs#L122-L170).

**Gap to close:** define the canonical sample rule independently of provider defaults:

- `1Y`: daily samples.
- `3Y`, `5Y`, `Max`: weekly samples.
- No monthly sample in this MVP.
- Daily and weekly bucket boundaries must be explicit, including UTC versus exchange or source calendar.
- A weekly sample must have a documented close rule, such as the last valid daily observation in the bucket. Do not silently mix Kraken’s week definition, Yahoo’s exchange calendar, and ECB/Fed business dates.

For Kraken, daily and weekly source intervals are available but limited to the most recent 720 entries. For other assets, the chosen provider must prove that it can supply the required range or the backend must expose a shorter actual coverage range.

### 3. Current prices

**Present:** the five-minute worker updates the latest `Market` snapshot in Redis. The `/price/{asset}` route computes a current conversion rate from those snapshots. The current price timestamp is the backend fetch time, not necessarily the source observation time.

**Gap to close:** separate current-price semantics from historical samples. Return or store `value`, `provider`, `provider_symbol`, `observed_at` when available, `fetched_at`, and a freshness state. A current price can be stale or unavailable and must say so. A Fed or ECB value is the latest published reference observation, not a live quote.

The current-price path should not depend on a historical OHLC row that may be the provider’s uncommitted candle. For Kraken, use the documented ticker path for current data or explicitly label the OHLC close as a delayed sample.

### 4. Retries with backoff

**Present:** Kraken and CoinMarketCap calls are wrapped in a `failsafe` circuit breaker, but the request path has no explicit retry loop. CryptoWatch makes one immediate second request with an API key after a 429. Yahoo has no retry or `Retry-After` handling. See [`adapter/mod.rs`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/mod.rs#L11-L23), [`kraken.rs`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/kraken.rs#L329-L376), and [`cw.rs`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/ports/outbound/adapter/cw.rs#L155-L181).

**Gap to close:** classify errors into retryable (timeouts, connection failures, 429, provider throttling, and selected 5xx responses) and permanent (invalid symbol, unsupported interval, malformed payload, and other non-retryable 4xx responses). Use bounded exponential backoff with jitter, respect a provider-provided retry timestamp where present, and record the next attempt in job state. A circuit breaker can complement retries; it does not replace a persisted retry schedule.

### 5. Timestamps and revisions

**Present:** `Price` has one timestamp, but [`fetch_market_price`](https://github.com/dcapal/dcapal/blob/f652260de11204971f95b6235ffc40cf812911a1/dcapal-backend/src/app/domain/market_data_utils.rs#L7-L35) sets it to the local fetch time. The Kraken adapter reads the candle timestamp only to choose a row and then drops it. Yahoo’s normalized proxy schema has no timestamp field. Fed and ECB use source observation dates and have different publication behavior.

**Gap to close:** preserve separate fields:

- `observed_at`: when the market/provider says the price belongs to, normalized to UTC where an instant exists.
- `observed_on`: source business date for Fed/ECB daily reference values when a date is the authoritative key.
- `fetched_at`: when DcaPal retrieved the value.
- `published_at` or `release_date`: when a Fed/ECB release made the value available, when known.
- `is_complete`: whether a candle or bucket is final.
- `source_revision` or an equivalent policy: whether revised ECB/Fed observations replace or retain earlier versions.

The response must state which timestamp drives chart ordering and forward filling. Never use `fetched_at` as a substitute for the observation date.

### 6. `timeframe`, `from`, and `to` endpoints

**Present:** the existing chart route has `startPeriod` and `endPeriod` and forwards a fixed 5-minute Yahoo request. It does not accept `timeframe`, does not query stored observations, does not return job status, and does not validate provider-independent coverage.

**Gap to close:** expose a normalized endpoint with the proposal’s four inputs: provider-bound `ticker`, `timeframe` (`daily` or `weekly`), `from`, and `to`. The exact route name can be settled by the API decision ticket, but the response should contain at least:

```json
{
  "ticker": "provider-bound ticker",
  "provider": "provider id",
  "timeframe": "daily",
  "from": "2025-01-01T00:00:00Z",
  "to": "2026-01-01T00:00:00Z",
  "samples": [{"observedAt": "2025-01-02T00:00:00Z", "value": "..."}],
  "status": "ready",
  "coverage": {"firstObservedAt": "...", "lastObservedAt": "...", "missingBuckets": []}
}
```

The actual numeric wire type remains an API decision; the important research result is that values must be normalized and timestamped rather than returned as provider-specific parallel arrays. Validate `from < to`, allowed timeframes, maximum request range, and authorization before reading the store. Return `pending` or `partial` with a job reference when the series is being fetched.

### 7. Forward-fill-compatible data

**Present:** the existing Yahoo route exposes close arrays without a normalized timestamp contract. No backend path records or reports missing buckets. The proposal expects the client to fill missing points, but the current response does not give the client a reliable source timeline.

**Gap to close:** return actual observations as `{observedAt, value}` pairs in ascending order. Do not emit zeroes or invent points before the first observation. The backend should also return coverage and missing-bucket information, or enough interval metadata for the client to derive it.

The client may then create the expected daily or weekly grid, copy the previous real value forward only after the first real value, and mark carried values as derived. Forward filling must stop or become visibly stale when the series has no acceptable observation for the requested end; it must not hide a failed maintenance job. This matches the map’s downstream decision requirement in [decision ticket #748](https://github.com/dcapal/dcapal/issues/748).

### 8. Observability

**Present:** the backend exports general HTTP and visitor metrics and logs worker errors. There are no series freshness, provider request, retry, job backlog, or coverage metrics.

**Gap to close:** add low-cardinality metrics for:

- fetch jobs created, claimed, succeeded, retried, and permanently failed, by job type and provider;
- provider request count, latency, timeout, 429, 5xx, and circuit-open events;
- observation rows received and written, with duplicate/upsert counts;
- queue depth, oldest pending age, running jobs, and retry-wait jobs;
- series freshness lag, last successful observation time, and missing daily/weekly buckets.

Structured logs should include `job_id`, provider, provider instrument id, timeframe, requested range, attempt, error class, and next retry time. Tracing should cover the job and each provider request. Alerts should fire on a growing queue, stale required series, repeated provider throttling, and failed maintenance runs. Do not put an unbounded ticker or user identifier into Prometheus labels; keep those details in logs and job rows.

## Proposed implementation contract for the next decision ticket

This is a research recommendation, not product code:

| Contract | Recommendation |
| --- | --- |
| Series identity | `provider` + provider instrument/ticker + exchange/market + base/quote or source rate key. Keep the provider-bound identity from the proposal discussion. |
| Durable observations | PostgreSQL table with a non-null observation time/date, value, sample kind, source, fetched time, completeness, and an idempotent uniqueness rule. Use standard PostgreSQL partitioning first; make Timescale optional. |
| Latest price | Redis cache may serve the hot value, but the value must carry source/fetch times and stale status. Persisting latest observations in PostgreSQL makes restart and audit behavior clearer. |
| Fetch jobs | Durable `queued → running → retry_wait → succeeded/failed` state, deduplication key, bounded attempts, next-attempt time, error class, and claim recovery. |
| Workers | A fetcher for backfills/current refresh and a maintenance scheduler for missing/stale coverage. Both must be safe to restart and must not mark a run successful after a provider or write failure. |
| Sampling | Daily for `1Y`; weekly for `3Y`, `5Y`, and `Max`; explicit UTC/source-calendar and weekly-close rules; no monthly MVP series. |
| API | Stored, normalized samples with `ticker`, `timeframe`, `from`, `to`, ordered timestamp/value pairs, coverage, and `ready`/`pending`/`partial`/`failed` status. |
| Client filling | Sparse real samples plus coverage metadata; forward-fill only after the first valid sample, with derived values distinguishable from source observations. |
| Daily official rates | Separate ECB EXR and Fed H.10 adapters, with business-date and publication/release semantics. Do not present them as live market prices. |
| Operations | Provider-aware throttling, retry with jitter, persistent retry state, queue/freshness metrics, structured logs, and alerts for stale or failed series. |

## Remaining decisions and risks

This ticket resolves feasibility and the repository gaps. The following choices still belong to later decision or implementation work:

- the licensed and operationally supported non-crypto historical provider;
- whether “Fed/ECB rate” means H.10/ECB FX reference rates or a separate policy-rate series;
- the weekly bucket calendar and close rule;
- the maximum `Max` range and behavior when a provider cannot supply it;
- whether revised ECB/Fed observations replace prior values or are versioned;
- the exact REST route and numeric wire representation;
- whether production PostgreSQL is pinned to a Timescale-supported version or uses native partitioning.

