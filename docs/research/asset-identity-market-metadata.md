# Findings: asset identity, market metadata, and trading-unit capability

Research date: 2026-08-03

## Context

- Ticket: [Research: Asset identity, market metadata, and trading-unit capability](https://github.com/dcapal/dcapal/issues/749)
- Wayfinder map: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/742)
- Product epic: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/738)
- Related uniqueness direction: [Enforce unique portfolio assets per saved portfolio](https://github.com/dcapal/dcapal/issues/714)

This is a research finding, not a product-code change. It uses the current repository source and first-party Yahoo Finance and Kraken documentation.

## Executive findings

1. **Keep symbol-based Portfolio identity, but scope it to a Portfolio.** The current frontend, REST payload, optimizer, and related ticket already use a provider-recognised `symbol` as the Portfolio Asset key. The compatible database invariant is `(portfolio_id, symbol)`: one symbol per saved Portfolio, the same symbol allowed in different Portfolios. This does not make `symbol` a globally unique market identity. A market-data identity remains provider- and venue-scoped metadata.

2. **Do not use the market catalog's `AssetId` as the Portfolio Asset key.** Backend market assets are currently catalog entries identified by a string `id`, while a Portfolio Asset also carries user holdings, target weights, price, fees, and a source/provider label. These are different concepts. Preserve the exact provider symbol used for pricing, its provider namespace, and any exchange/market identifier alongside the Portfolio-local symbol.

3. **Treat `provider` as price provenance, not exchange identity.** Frontend Portfolio values use `DCAPal` or `YF`; backend runtime providers are CryptoWatch, Kraken, and Yahoo. The persisted `provider` field therefore cannot by itself identify a venue or a canonical instrument.

4. **The six planning classes are a product taxonomy, not provider asset types.** The epic names `Equities`, `Bonds`, `Cash`, `Crypto`, `Commodities`, and `Other`. Current code has only `EQUITY`, `CRYPTO`, and `CURRENCY`. `CURRENCY` is a market-data/fiat category, so it can default to `Cash` for cash holdings, but it is not proof that every currency-denominated instrument is cash. `Bonds`, `Commodities`, and `Other` need explicit classification; current provider data is not enough to infer them safely.

5. **Fractional capability must be an explicit, provider/market-scoped fact or `unknown`.** Current code infers whole units from `EQUITY` and passes a single boolean to the optimizer. Yahoo supplies quote/search and price metadata, not order quantity rules. Kraken's `AssetPairs` response supplies pair-specific quantity precision and minimums, which can support a fractional-unit inference for Kraken spot pairs. Neither source justifies a universal “equities are whole, crypto is fractional” rule.

6. **The canonical schema must separate source price currency from Portfolio quote currency.** Yahoo chart metadata reports the currency of the source close series; DcaPal converts that price to the Portfolio quote currency. The current `baseCcy` field is used for this on Yahoo but is set to the asset symbol for DcaPal assets, and the backend persistence column is called `currency`. Preserve both the source price currency and the Portfolio quote currency without overloading one field.

## Current repository model

### Frontend Portfolio identity and provider model

- `Portfolio.assets` is a `Record<string, PortfolioAsset>`. Every Portfolio Asset contains `symbol`, display `name`, numeric `aclass`, `baseCcy`, `provider`, quantity, price, targets, and fees. The numeric classes are currently `UNDEFINED`, `EQUITY`, `CRYPTO`, and `CURRENCY`; the only class-specific unit rule is `EQUITY => whole units`. [Frontend domain model](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/state/portfolioDomain.ts#L1-L114)
- `addAsset` uses the incoming symbol as the object key and rejects the addition when that symbol already exists in the current Portfolio. All edit, remove, price-refresh, fee, and quantity actions look up the asset by symbol. [Portfolio store identity and actions](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/state/portfolioStore.ts#L591-L752)
- Synchronization lowercases the symbol at the REST boundary, but the in-memory object key and displayed symbol can retain the original casing. The current code does not document whether symbol equality is case-sensitive or whether exchange suffixes are mandatory. [Synchronization payload mapping](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/portfolioSync.ts#L92-L117)
- The frontend price-provider enum is only `DCAPal` and `YF`. It selects a pricing adapter; it does not identify a broker, exchange, or trading venue. [Frontend provider adapter](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/priceProviders.ts#L11-L27)

### Frontend search and quote conversion

- DcaPal catalog assets arrive as `{id, symbol}`. The frontend maps `id` to the Portfolio `symbol`, `symbol` to display `name`, and hard-codes `CURRENCY` or `CRYPTO` as the class. Yahoo search results use `symbol`, `longname`/`shortname`, `quoteType`, and `exchange`; the mapper hard-codes every accepted Yahoo result to `EQUITY`. [Search result types and mappings](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.tsx#L23-L102)
- Search currently keeps only Yahoo `EQUITY`, `ETF`, and `MUTUALFUND` results. It drops Yahoo commodities, currencies, crypto, futures, and other result types before the Portfolio sees them. The result type includes `exchange`, but the rendered result does not display it. [Search filtering](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.tsx#L149-L169) and [search result rendering](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.tsx#L230-L265)
- A selected Yahoo result is priced from Yahoo chart data. The chart's `meta.currency` becomes `baseCcy`; if it differs from the Portfolio quote currency, the frontend asks DcaPal for a conversion rate. The resulting Portfolio price is therefore in the Portfolio quote currency, while the source currency is separately available during selection. [Yahoo price conversion](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/priceProviders.ts#L76-L110) and [Yahoo asset selection](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.tsx#L351-L385)
- DcaPal asset selection calls `GET /price/{asset}?quote=...` and stores the selected asset symbol as `baseCcy`. This is a compatibility shortcut, not a reliable statement that the asset's source price currency equals its symbol. [DcaPal asset selection](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/allocationFlow/steps/portfolio/searchBar.tsx#L275-L307)

### Backend market catalog and providers

- The backend market catalog defines `AssetId = String` and `MarketId = String`. A market asset is either a `Crypto { id, symbol }` or a `Fiat { id, symbol }`; a `Market` has an `id`, derived display pair, base asset, quote asset, and optional cached price. [Backend market entities](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/app/domain/entity.rs#L7-L48) and [market entity](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/app/domain/entity.rs#L173-L229)
- Redis stores assets by `Asset::id()` and markets by `Market::id`; a market DTO stores only market id, pair, base asset id, quote asset id, and cached price. This catalog identity is separate from the Portfolio Asset row. [Redis asset storage](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/market_data/redis_asset.rs#L28-L67), [Redis market storage](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/market_data/redis_market.rs#L36-L66), and [market DTO](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/dto.rs#L5-L27)
- The backend runtime can price markets with CryptoWatch, Kraken, or Yahoo, selected by configuration. Market discovery always discovers crypto markets through Kraken, then stores the discovered assets and the fetched market price. [Provider configuration](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/config.rs#L5-L38), [provider dispatch](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/app/domain/market_data_utils.rs#L7-L35), and [market discovery](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/app/workers/market_discovery.rs#L89-L135)
- The persisted Portfolio Asset row currently contains `symbol`, display `name`, free-text `asset_class`, `currency`, `provider`, decimal quantity, target weight, price, fees, and average buy price. It has no exchange, market id, provider asset id, source price currency, quantity increment, minimum quantity, or price provenance field. [Portfolio Asset persistence row](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/postgres/types/portfolio_asset.rs#L5-L45) and [current table migration](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/migrations/20250201132246_create_table_portfolio_asset.up.sql#L1-L23)

### Yahoo adapter and REST contract

- Backend Yahoo prices use `query1.finance.yahoo.com/v8/finance/chart/{symbol}`. Backend Yahoo search uses `query2.finance.yahoo.com/v1/finance/search?q=...`. The adapter constructs the Yahoo symbol from the market base and quote, including special fiat and crypto mappings. [Yahoo adapter endpoints](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/yahoo.rs#L41-L52) and [Yahoo search/chart forwarding](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/yahoo.rs#L120-L150)
- The typed proxy contract retains only Yahoo search `quoteType`, names, `symbol`, and `exchange`; the typed chart contract retains only `meta.currency` and close values. The live proxy may pass through more JSON, but the application contract does not preserve it. [Yahoo proxy types](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/inbound/rest/proxy_types.rs#L4-L49)
- The current REST portfolio schema permits only `EQUITY`, `CRYPTO`, and `CURRENCY`, and describes `provider` as `DCAPal` or `YF`. It accepts decimal strings for price and quantity but has no market, exchange, source currency, or unit-capability fields. [Portfolio schema](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/docs/schema/portfolio/v1/schema.json#L124-L186)
- The schema's `assets.uniqueItems: true` rejects exact duplicate objects, not two objects with the same symbol but different name, price, class, or other fields. It is not a symbol uniqueness rule. [Portfolio asset-array schema](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/docs/schema/portfolio/v1/schema.json#L20-L31)

### Kraken adapter

- The current Kraken adapter calls `GET /0/public/AssetPairs`, keeps only pairs whose `status` is `online`, and reads only `wsname` and `status`. It normalizes `XBT` to `BTC`, lowercases the pair, splits it into base and quote ids, then synthesizes assets and markets. It discards the provider pair key, alternate name, provider base/quote ids, asset classes, precision, increments, minimums, and other pair metadata. [Kraken request and filtering](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L44-L78), [current response type](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L379-L389), and [normalization](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L426-L463)
- If CoinMarketCap is configured, the adapter enriches crypto display names from CoinMarketCap by symbol; otherwise it falls back to Kraken ids as both id and display symbol. This enrichment does not preserve an exchange-qualified identity or unit constraints. [Kraken/CMC enrichment](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L171-L205) and [Kraken-only fallback](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L391-L493)
- Kraken's official `AssetPairs` documentation exposes a richer pair identity and trading contract: internal/display pair identifiers, base and quote ids, `aclass_base`/`aclass_quote`, `lot`, `lot_decimals`, `lot_multiplier`, `ordermin`, `costmin`, `tick_size`, and `status`. `assetVersion=1` switches identifiers to canonical display names such as `BTC/USD`; the default response uses internal `X`/`Z`-prefixed names. [Kraken Get Tradable Asset Pairs](https://docs.kraken.com/api-reference/market-data/get-tradable-asset-pairs)
- Kraken's official `Assets` endpoint separately exposes internal asset keys, display `altname`, asset class, quantity `decimals`, display decimals, and status. The current adapter does not call this endpoint. [Kraken Get Asset Info](https://docs.kraken.com/api-reference/market-data/get-asset-info)

## Provider facts and limits

### Yahoo Finance

Yahoo Finance search is a useful discovery and display source. Yahoo's own help says its finance search covers companies, ticker symbols, ETFs, indices, commodities, mutual funds, and cryptocurrency, and that it is not an ISIN search. Yahoo also documents exchange suffixes as part of the symbol used to look up an instrument on a covered exchange. This supports preserving the exact Yahoo symbol, including a suffix such as `.DE`, and the exchange value when available. [Yahoo search help](https://uk.help.yahoo.com/kb/find-quote-yahoo-finance-sln2340.html) and [Yahoo exchanges and data providers](https://help.yahoo.com/kb/finance/SLN2310.html)

Yahoo's official quote-page documentation treats an exchange as part of the information shown for an ETF and describes the quote page as informational research. Yahoo also states that it is not a broker-dealer and does not facilitate trading. Therefore Yahoo metadata can support identity, display, classification hints, and price provenance, but it cannot establish whether the user's broker accepts fractional or whole-unit orders. [Yahoo quote pages](https://help.yahoo.com/kb/research-stocks-mutual-funds-etfs-yahoo-finance-quote-pages-sln28277.html) and [Yahoo Finance quote disclosure](https://finance.yahoo.com/quote/META/)

The repository's Yahoo chart path reads `meta.currency` and close values. That is enough to identify the source quote currency of the returned price series and to convert it to the Portfolio quote currency. It is not enough to identify a trading venue's order increment or minimum. [Backend chart model](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/inbound/rest/proxy_types.rs#L19-L49) and [frontend conversion path](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/priceProviders.ts#L76-L110)

### Kraken

Kraken's official pair data is the strongest available source in this repository for trading-unit capability, but it is still **pair-specific execution metadata**, not a universal property of an asset. Kraken's order API defines `volume` as the order quantity in the base asset and directs clients to `AssetPairs` for price/quantity precision and order minimums. [Kraken Add Order](https://docs.kraken.com/api-reference/trading/add-order)

For a Kraken spot pair, `lot_decimals` can be represented as a quantity precision and `ordermin`/`costmin` as minimum constraints. A fractional capability can be inferred when the accepted quantity step is below one; a whole-unit rule can be inferred only when the pair's accepted step is one or greater. That inference must remain scoped to the pair, provider, and observation time. `display_decimals` from `Assets` is a display precision, not an order increment. These are inferences from the documented fields, not fields currently supplied to DcaPal.

Kraken's data also shows why one symbol is not enough for market identity: the same asset can have internal and display ids, and a pair is defined by base plus quote. A Portfolio can still use a symbol key for the MVP, but a provider/venue-qualified market reference must remain available when one symbol maps to multiple markets.

### CryptoWatch and CoinMarketCap in the current backend

CryptoWatch is used to discover Kraken market pairs and to fetch OHLC data. Its current adapter stores only currency ids/names and the pair id; it does not model exchange, precision, or order rules. [CryptoWatch adapter](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/cw.rs#L33-L101) and [CryptoWatch asset response mapping](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/cw.rs#L213-L252)

CoinMarketCap is optional display-name enrichment in the Kraken path. It is not the source of the current Portfolio provider enum and is not a source of trading-unit capability in this repository. [CoinMarketCap enrichment](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/adapter/kraken.rs#L283-L326)

## Identity decision and the unique-asset direction

The symbol-based Portfolio decision and the unique-asset direction are compatible, with one important boundary:

- **Portfolio identity:** `(saved portfolio, canonical symbol)`. This is the key used by the frontend map and by the existing synchronization logic. The related ticket explicitly requires one symbol per saved Portfolio, permits the same symbol in different Portfolios, rejects duplicate symbols in one sync request, and requires a database uniqueness boundary. [Issue #714](https://github.com/dcapal/dcapal/issues/714)
- **Market identity:** `(market-data provider, venue/exchange, provider asset or pair id)`. This is the identity needed to find a price, base/quote pair, exchange, or unit constraint. It must not replace the Portfolio-local symbol in this MVP.
- **Required normalization decision:** define whether symbols compare case-insensitively and whether normalization happens before persistence. The frontend currently lowercases outgoing symbols, while the backend repository compares stored symbols exactly. A uniqueness migration must preflight duplicate values after applying the chosen equality rule; it must not silently merge conflicting rows. [Frontend sync mapping](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/api/portfolioSync.ts#L102-L114) and [backend symbol matching](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/postgres/portfolio.rs#L44-L149)
- **Do not add provider to the existing uniqueness key without a product decision.** The existing direction says `(portfolio, symbol)`, not `(portfolio, provider, symbol)`. Provider and raw symbol should be preserved for pricing and migration diagnostics, but changing the uniqueness key would allow two rows that the current frontend treats as the same asset.
- **Preserve aliases rather than silently merging them.** Kraken's `XBT`/`BTC` and internal/display ids demonstrate that provider aliases exist. If a future canonicalization changes a symbol, retain the source symbol and provider identity long enough to explain or review the migration. The current frontend already rejects a duplicate selected symbol, and the current database has no uniqueness constraint. [Frontend duplicate guard](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/state/portfolioStore.ts#L591-L631) and [current database constraint set](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/migrations/20250201132246_create_table_portfolio_asset.up.sql#L1-L23)

## Six-class migration

The product epic defines these six planning classes: `Equities`, `Bonds`, `Cash`, `Crypto`, `Commodities`, and `Other`. It also says Bonds and Cash are defensive for planning, while the other four are risk-on by default. This is a Portfolio taxonomy and must not be copied directly from a market provider's technical `aclass` or `quoteType`. [Product epic, class decision](https://github.com/dcapal/dcapal/issues/738)

| Current value | Default migration | Confidence and required handling |
| --- | --- | --- |
| `EQUITY` | `Equities` | High for the current Yahoo-filtered equity/ETF/mutual-fund flow. Preserve the raw Yahoo `quoteType` because ETF and mutual-fund are provider types, not identical economic classes. |
| `CRYPTO` | `Crypto` | High for the current DcaPal crypto catalog. Preserve provider id/symbol because aliases and provider namespaces still matter. |
| `CURRENCY` | `Cash` by default | Medium. Current DcaPal fiat assets and the optimizer's unallocated-cash row use this value, but a currency/FX exposure is not automatically a cash holding. Flag non-cash currency use for review rather than silently treating it as cash. |
| No current value | `Bonds` | No automatic legacy mapping exists. A provider instrument type, name, or user selection is needed. |
| No current value | `Commodities` | No automatic legacy mapping exists. Current Yahoo search filters commodities out before persistence. |
| No current value | `Other` | Safe fallback for an explicitly supported but unclassified instrument; do not use it to hide unresolved provider data. |

Current migration gaps are visible in all calculation layers:

- Frontend numeric constants and parsing support only three persisted classes. [Frontend class enum and parser](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/state/portfolioDomain.ts#L1-L114)
- The import JSON Schema allows only three class strings. [Import schema enum](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/docs/schema/portfolio/v1/schema.json#L148-L155)
- Backend synchronization accepts `aclass` as unconstrained free text, so import and sync do not have one class contract. [Sync request model](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/inbound/rest/request.rs#L35-L49)
- The optimizer scenario model also has only `Equity`, `Crypto`, and `Currency`, and its whole-unit rule is tied to `Equity`. [Optimizer scenario class model](https://github.com/dcapal/dcapal/blob/master/dcapal-optimizer-wasm/crates/optimizer/tests/test_runner/scenario.rs#L66-L93)
- The allocation path passes `is_whole_shares` as a boolean, either from the class heuristic or from the global “whole shares” option. It has no per-asset provider capability or quantity step. [Allocation input](https://github.com/dcapal/dcapal-frontend/src/components/allocationFlow/steps/invest.js#L17-L31) and [solver input](https://github.com/dcapal/dcapal-frontend/src/components/allocationFlow/steps/end/index.js#L114-L130)

The migration should preserve the raw provider type/category and any existing user class during the transition. A new canonical class should be a stable planning enum; provider `quoteType`, `aclass`, `asset kind`, and exchange metadata should remain source metadata used to suggest or validate the class, not become the class itself.

## Fractional and whole-unit capability

### What is known today

- Portfolio quantities are decimal strings in the import schema and decimal values in the PostgreSQL row. The current schema therefore already needs to retain fractional holdings even when the optimizer is configured to recommend whole units. [Schema quantity](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/docs/schema/portfolio/v1/schema.json#L161-L175) and [persistence quantity](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/crates/backend/src/ports/outbound/repository/postgres/types/portfolio_asset.rs#L21-L28)
- The current optimizer's whole-unit behavior is a user/configuration rule, not provider capability. It can force whole units for equities or for all assets when the global option is enabled, but it cannot express a pair-specific step such as `0.0001`. [Whole-unit helper](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/state/portfolioDomain.ts#L88-L96) and [optimizer contract](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/compute/types.ts#L1-L43)
- Yahoo has no broker/execution relationship in the current product and its documented data is informational. No Yahoo source in this integration supplies a quantity increment or minimum order. [Yahoo Finance disclosure](https://finance.yahoo.com/quote/META/)
- Kraken `AssetPairs` supplies the fields needed to represent pair-specific order constraints, and Kraken's order endpoint defines order volume in base-asset units. [Kraken pair metadata](https://docs.kraken.com/api-reference/market-data/get-tradable-asset-pairs) and [Kraken order volume](https://docs.kraken.com/api-reference/trading/add-order)

### Canonical capability shape

The schema should preserve capability as structured, optional metadata rather than a single derived boolean:

- `unitCapability`: `fractional`, `whole`, or `unknown` for the selected provider/market context;
- `quantityStep` or `quantityPrecision`, when the provider publishes it;
- `minimumQuantity` and `minimumNotional`, when published;
- the provider, venue/market id, and observation timestamp for those constraints;
- the Portfolio holding quantity as an exact decimal independent of display precision.

For this MVP, `unknown` is the correct value for Yahoo-priced assets and for any asset with no execution-provider metadata. A user-selected “whole units” option may constrain the recommendation, but it must not rewrite or mislabel the stored holding quantity.

## Canonical schema facts to preserve

At minimum, a coordinated Portfolio/market-data migration should retain these facts:

1. **Portfolio-local identity:** canonical symbol used by the Portfolio, with a documented normalization/equality rule and `(portfolio_id, symbol)` uniqueness.
2. **Source identity:** price provider, raw provider symbol, provider asset id, and exchange/venue or market id when the provider exposes them. Do not assume one global asset id across Yahoo, Kraken, CryptoWatch, or CoinMarketCap.
3. **Display metadata:** name plus exchange/market label and provider type/category. Display name is not identity.
4. **Planning classification:** one of the six product classes, separate from raw provider type and market-data asset kind.
5. **Pricing provenance:** source price currency, Portfolio quote currency, price value, provider/market source, and observation/fetch timestamp. Keep direct market price distinct from a synthetic conversion rate.
6. **Holding and allocation values:** exact decimal quantity, average buy price, current price in the Portfolio quote currency, target fields, and fees. Do not round quantities to provider display decimals or erase fractional holdings during class migration.
7. **Unit constraints, when known:** provider/market-scoped step or precision, minimum quantity, minimum notional, and an explicit unknown state when no execution metadata exists.
8. **Migration evidence:** original class, original symbol/provider identity, and duplicate/preflight diagnostics until the coordinated migration has been reviewed. Never silently merge rows with conflicting holdings or targets.

The current Portfolio schema has only part of this set: symbol, name, three-class `aclass`, base currency, price, quantity, target weight, provider, and fees. [Current asset schema](https://github.com/dcapal/dcapal/blob/master/dcapal-backend/docs/schema/portfolio/v1/schema.json#L124-L186)

## Implementation-ready route

1. Keep the Portfolio key symbol-based and enforce the already specified `(saved portfolio, symbol)` uniqueness rule after a non-destructive duplicate preflight. Define case normalization before the migration and preserve conflicting source values for review.
2. Keep market identity separate. Add a normalized provider metadata boundary that can retain exact symbol, provider asset id, venue/exchange, market id, base/quote ids, source currency, and raw provider type.
3. Make the six-class enum a planning contract. Migrate `EQUITY → Equities`, `CRYPTO → Crypto`, and `CURRENCY → Cash` as a reviewed default; require explicit handling for Bonds, Commodities, and Other and retain raw provider metadata.
4. Replace the class-based whole-unit heuristic with a per-asset unit constraint in the optimizer contract. Use provider precision only when available; otherwise use `unknown` and let the user's whole/fractional preference be a separate recommendation rule.
5. Preserve source price currency and Portfolio quote currency as separate fields. Continue to value holdings in the Portfolio quote currency, but retain enough provenance to explain the conversion and refresh timestamp.

This route resolves the research question without changing product code. It leaves only the exact symbol normalization policy and the user-facing handling of unresolved six-class and unit-capability cases for the relevant decision tickets.
