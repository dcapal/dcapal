//! PostgreSQL-backed storage for the market-asset catalog, markets and their
//! latest prices.
//!
//! Market assets are rows of the shared `assets_data` table: crypto assets listed
//! by Kraken (provider Kraken, class Crypto) and the fiat currencies of the fixed
//! catalog (provider Yahoo, class Cash). A market is a row of `market_price`
//! keyed by its base and quote `assets_data` ids. The v1 REST API still exposes
//! the lower-case symbol as asset id; the market id is `{base}/{quote}` (`btc/eur`), stored
//! on each `market_price` row as `market_id`.
//!
//! Only provider-confirmed rows belong to the catalog: assets that clients
//! supplied through synchronization never show up in it.

use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use jiff_sqlx::ToSqlx;
use sqlx::{PgPool, Postgres, Transaction, query, query_as, query_scalar};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    app::domain::{
        entity::{Asset, AssetId, AssetKind, Crypto, Fiat, Market, MarketId, Price, market_id},
        fiat::fiat_symbols,
        market_quote::{PriceFailure, StoredQuote, validate_price},
        portfolio::Provider as AppProvider,
    },
    config::PriceProvider,
    error::{DcaError, ErrorChain, Result},
    ports::outbound::repository::postgres::types::{
        AssetClass as DbAssetClass, MetadataSource as DbMetadataSource, Provider as DbProvider,
    },
};

/// A freshly observed price for one market, ready to be stored.
#[derive(Debug, Clone, Copy)]
pub struct MarketPriceUpdate<'a> {
    /// The market the price belongs to.
    pub market_id: &'a MarketId,
    /// The observed price and its observation time.
    pub price: Price,
    /// The provider that supplied the price.
    pub provider: PriceProvider,
}

/// A failed price refresh of one market, ready to be recorded next to its last known price.
#[derive(Debug, Clone, Copy)]
pub struct MarketPriceFailure<'a> {
    /// The market whose refresh failed.
    pub market_id: &'a MarketId,
    /// Why the refresh failed.
    pub failure: &'a PriceFailure,
}

/// A Kraken catalog crypto asset found by a catalog search.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct CatalogSearchHit {
    /// Upper-case Kraken symbol.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Whether a USD market of the asset has a stored price.
    pub priced: bool,
}

/// Escapes the `LIKE` wildcards of user text, with `\` as the escape character.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Reads and writes the market-asset catalog, markets and latest prices.
#[derive(Clone)]
pub struct MarketDataRepository {
    pool: PgPool,
}

/// A market's two catalog assets and, when present, its latest price.
#[derive(sqlx::FromRow)]
struct MarketRow {
    base_provider: DbProvider,
    base_class: DbAssetClass,
    base_symbol: String,
    base_name: String,
    quote_provider: DbProvider,
    quote_class: DbAssetClass,
    quote_symbol: String,
    quote_name: String,
    price: Option<f64>,
    observed_at: Option<jiff_sqlx::Timestamp>,
}

/// A market's latest price with its refresh failure, as stored in `market_price`.
#[derive(sqlx::FromRow)]
struct QuoteRow {
    market_id: String,
    base_symbol: String,
    quote_symbol: String,
    provider: DbProvider,
    price: Option<f64>,
    observed_at: Option<jiff_sqlx::Timestamp>,
    fetched_at: Option<jiff_sqlx::Timestamp>,
    last_error: Option<String>,
    last_error_at: Option<jiff_sqlx::Timestamp>,
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    provider: DbProvider,
    asset_class: DbAssetClass,
    symbol: String,
    name: String,
}

/// Markets joined with their two assets and prices; callers append a `WHERE`.
const SELECT_MARKETS: &str = r#"
    SELECT b.provider AS base_provider, b.asset_class AS base_class,
           b.symbol AS base_symbol, b.name AS base_name,
           q.provider AS quote_provider, q.asset_class AS quote_class,
           q.symbol AS quote_symbol, q.name AS quote_name,
           p.price::float8 AS price, p.observed_at
    FROM market_price AS p
    JOIN assets_data AS b ON b.id = p.base_asset_id
    JOIN assets_data AS q ON q.id = p.quote_asset_id
"#;

/// Selects the catalog assets; callers append a catalog predicate. The columns
/// are the same for every query, only the predicate's parameters differ.
const SELECT_ASSETS: &str =
    "SELECT ad.provider, ad.asset_class, ad.symbol, ad.name FROM assets_data AS ad";

/// Catalog crypto: confirmed Kraken crypto. Parameters: provider, class, source.
const CRYPTO_PREDICATE: &str =
    "ad.provider = $1 AND ad.asset_class = $2 AND ad.metadata_source = $3";

/// Catalog fiat: confirmed Yahoo cash among the fiat catalog symbols.
/// Parameters: provider, class, symbols, source.
const FIAT_PREDICATE: &str =
    "ad.provider = $1 AND ad.asset_class = $2 AND ad.symbol = ANY($3) AND ad.metadata_source = $4";

impl MarketDataRepository {
    /// Creates a repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Finds a catalog asset by its lower-case id (the lower-cased symbol).
    pub async fn find_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        Ok(self
            .find_assets(std::slice::from_ref(id))
            .await?
            .pop()
            .flatten())
    }

    /// Finds several catalog assets with one query, matching ids case-insensitively.
    ///
    /// The result is aligned with `ids`: unknown assets are `None`.
    pub async fn find_assets(&self, ids: &[impl AsRef<str>]) -> Result<Vec<Option<Asset>>> {
        let wanted = ids
            .iter()
            .map(|id| id.as_ref().to_uppercase())
            .collect::<Vec<_>>();
        let rows = query_as::<_, AssetRow>(const_format::formatcp!(
            "{SELECT_ASSETS} WHERE ad.symbol = ANY($1) AND ad.metadata_source = $5 AND (
                 (ad.provider = $2 AND ad.asset_class = $3)
                 OR (ad.provider = $6 AND ad.asset_class = $7 AND ad.symbol = ANY($4)))"
        ))
        .bind(&wanted)
        .bind(DbProvider::Kraken)
        .bind(DbAssetClass::Crypto)
        .bind(fiat_symbols())
        .bind(DbMetadataSource::ProviderConfirmed)
        .bind(DbProvider::YF)
        .bind(DbAssetClass::Cash)
        .fetch_all(&self.pool)
        .await?;

        let found = rows
            .into_iter()
            .map(|row| Asset::try_from(row).map(|asset| (asset.id().to_uppercase(), asset)))
            .collect::<Result<HashMap<_, _>>>()?;

        // Ids may repeat (a conversion from an asset to itself), so clone instead of remove.
        Ok(wanted.iter().map(|id| found.get(id).cloned()).collect())
    }

    /// Loads every catalog asset of a kind, in no particular order.
    pub async fn load_assets_by_type(&self, kind: AssetKind) -> Result<Vec<Asset>> {
        let rows = match kind {
            AssetKind::Crypto => {
                query_as::<_, AssetRow>(const_format::formatcp!(
                    "{SELECT_ASSETS} WHERE {CRYPTO_PREDICATE}"
                ))
                .bind(DbProvider::Kraken)
                .bind(DbAssetClass::Crypto)
                .bind(DbMetadataSource::ProviderConfirmed)
                .fetch_all(&self.pool)
                .await?
            }
            AssetKind::Fiat => {
                query_as::<_, AssetRow>(const_format::formatcp!(
                    "{SELECT_ASSETS} WHERE {FIAT_PREDICATE}"
                ))
                .bind(DbProvider::YF)
                .bind(DbAssetClass::Cash)
                .bind(fiat_symbols())
                .bind(DbMetadataSource::ProviderConfirmed)
                .fetch_all(&self.pool)
                .await?
            }
        };

        rows.into_iter().map(Asset::try_from).collect()
    }

    /// Searches the provider-confirmed Kraken crypto catalog by symbol or name.
    ///
    /// The match is a case-insensitive substring of either column with `%` and `_`
    /// taken literally. An exact symbol match comes first, then symbol order; at most
    /// `limit` assets are returned, and price availability is computed only for those.
    /// `priced` tells whether the asset has a USD market
    /// with a stored price.
    pub async fn search_crypto_catalog(
        &self,
        text: &str,
        limit: i64,
    ) -> Result<Vec<CatalogSearchHit>> {
        let hits = query_as::<_, CatalogSearchHit>(const_format::formatcp!(
            "WITH hits AS (
                 SELECT ad.id, ad.symbol, ad.name
                 FROM assets_data AS ad
                 WHERE {CRYPTO_PREDICATE}
                   AND (ad.symbol ILIKE $7 ESCAPE '\\' OR ad.name ILIKE $7 ESCAPE '\\')
                 ORDER BY (ad.symbol = $8) DESC, ad.symbol
                 LIMIT $9
             )
             SELECT h.symbol, h.name,
                    EXISTS (SELECT 1 FROM market_price AS p
                            JOIN assets_data AS q ON q.id = p.quote_asset_id
                            WHERE p.base_asset_id = h.id AND p.price IS NOT NULL
                              AND q.provider = $4 AND q.asset_class = $5 AND q.symbol = $6
                           ) AS priced
             FROM hits AS h
             ORDER BY (h.symbol = $8) DESC, h.symbol"
        ))
        .bind(DbProvider::Kraken)
        .bind(DbAssetClass::Crypto)
        .bind(DbMetadataSource::ProviderConfirmed)
        .bind(DbProvider::YF)
        .bind(DbAssetClass::Cash)
        .bind("USD")
        .bind(format!("%{}%", escape_like(text)))
        .bind(text.to_ascii_uppercase())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(hits)
    }

    /// Tells whether at least one provider-confirmed Kraken crypto asset is known.
    ///
    /// `false` means market discovery has not completed yet (cold start).
    pub async fn has_crypto_assets(&self) -> Result<bool> {
        Ok(query_scalar::<_, bool>(const_format::formatcp!(
            "SELECT EXISTS (SELECT 1 FROM assets_data AS ad WHERE {CRYPTO_PREDICATE})"
        ))
        .bind(DbProvider::Kraken)
        .bind(DbAssetClass::Crypto)
        .bind(DbMetadataSource::ProviderConfirmed)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Stores a catalog asset in `assets_data`; see [`Self::store_assets`].
    pub async fn store_asset(&self, asset: &Asset) -> Result<()> {
        self.store_assets(std::slice::from_ref(asset)).await
    }

    /// Stores catalog assets in `assets_data` with one statement, atomically.
    ///
    /// Missing assets are inserted as provider-confirmed. Provider-confirmed rows
    /// are never changed. A client-supplied fiat row is replaced by the catalog's
    /// English name and class; a client-supplied crypto row keeps the name the
    /// client chose (Kraken only knows the symbol) and is just marked confirmed.
    /// When an asset appears several times the first occurrence wins.
    pub async fn store_assets(&self, assets: &[Asset]) -> Result<()> {
        upsert_assets(&self.pool, assets).await?;
        debug!("Successfully stored {} market assets", assets.len());
        Ok(())
    }

    /// Stores catalog assets and then markets in one transaction.
    ///
    /// Either everything is stored or nothing is. See [`Self::store_assets`] and
    /// [`Self::store_markets`] for the semantics of each part.
    pub async fn store_catalog(
        &self,
        assets: &[Asset],
        markets: &[(&Market, PriceProvider)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        upsert_assets(&mut *tx, assets).await?;
        insert_markets(&mut tx, markets).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Finds a market, with its latest price when one has been stored.
    pub async fn find_market(&self, id: &MarketId) -> Result<Option<Market>> {
        let row = query_as::<_, MarketRow>(const_format::formatcp!(
            "{SELECT_MARKETS} WHERE p.market_id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Market::try_from).transpose()
    }

    /// Finds several markets at once.
    ///
    /// The result is aligned with `ids`: unknown markets are `None`.
    pub async fn find_markets(&self, ids: &[impl AsRef<str>]) -> Result<Vec<Option<Market>>> {
        let wanted = ids.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let rows = query_as::<_, MarketRow>(const_format::formatcp!(
            "{SELECT_MARKETS} WHERE p.market_id = ANY($1)"
        ))
        .bind(&wanted)
        .fetch_all(&self.pool)
        .await?;

        let mut found = rows
            .into_iter()
            .map(|row| Market::try_from(row).map(|m| (m.id.clone(), m)))
            .collect::<Result<std::collections::HashMap<_, _>>>()?;

        Ok(ids.iter().map(|id| found.remove(id.as_ref())).collect())
    }

    /// Reads the latest price, provenance and refresh failure of several markets.
    ///
    /// Markets are keyed by their v1 id; unknown markets are absent from the result.
    /// Unlike [`Self::find_markets`] this does not require catalog assets, so it reports
    /// the stored state as is.
    pub async fn find_market_quotes(
        &self,
        ids: &[impl AsRef<str>],
    ) -> Result<HashMap<MarketId, StoredQuote>> {
        let wanted = ids.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let rows = query_as::<_, QuoteRow>(const_format::formatcp!(
            "SELECT p.market_id, b.symbol AS base_symbol, q.symbol AS quote_symbol,
                    p.provider, p.price::float8 AS price, p.observed_at, p.fetched_at,
                    p.last_error, p.last_error_at
             FROM market_price AS p
             JOIN assets_data AS b ON b.id = p.base_asset_id
             JOIN assets_data AS q ON q.id = p.quote_asset_id
             WHERE p.market_id = ANY($1)"
        ))
        .bind(&wanted)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| (row.market_id.clone(), StoredQuote::from(row)))
            .collect())
    }

    /// Stores one market; see [`Self::store_markets`].
    pub async fn store_market(&self, market: &Market, provider: PriceProvider) -> Result<()> {
        self.store_markets(&[(market, provider)]).await
    }

    /// Inserts the markets that do not exist yet and stores the prices they carry,
    /// in one transaction and a constant number of statements.
    ///
    /// All catalog assets must already be stored; otherwise nothing is written and
    /// the first market with a missing asset is reported. `provider` is the provider
    /// that prices each market; an existing market keeps its stored provider until
    /// a price is stored. When a market appears several times the last price wins.
    pub async fn store_markets(&self, markets: &[(&Market, PriceProvider)]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        insert_markets(&mut tx, markets).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes crypto markets absent from a complete Kraken snapshot, with their prices.
    ///
    /// Fiat against fiat markets (priced by Yahoo, never listed by Kraken) are never
    /// deleted. Returns the deleted market ids. An empty snapshot is rejected so that
    /// a failed provider call can never wipe every market.
    pub async fn delete_markets_not_in(
        &self,
        market_ids: &HashSet<MarketId>,
    ) -> Result<Vec<MarketId>> {
        if market_ids.is_empty() {
            return Err(DcaError::Generic(
                "Cannot reconcile markets from an empty Kraken snapshot".to_string(),
            ));
        }

        let keep = market_ids.iter().map(String::as_str).collect::<Vec<_>>();
        let deleted = query_scalar::<_, String>(const_format::formatcp!(
            "DELETE FROM market_price AS p
             USING assets_data AS b, assets_data AS q
             WHERE b.id = p.base_asset_id AND q.id = p.quote_asset_id
               AND NOT EXISTS (SELECT 1 FROM UNNEST($1::text[]) AS k(id) WHERE k.id = p.market_id)
               AND NOT (b.provider = $2 AND b.asset_class = $3 AND b.symbol = ANY($4)
                    AND q.provider = $2 AND q.asset_class = $3 AND q.symbol = ANY($4))
             RETURNING p.market_id"
        ))
        .bind(&keep)
        .bind(DbProvider::YF)
        .bind(DbAssetClass::Cash)
        .bind(fiat_symbols())
        .fetch_all(&self.pool)
        .await?;

        if !deleted.is_empty() {
            debug!("Removed {} stale markets", deleted.len());
        }
        Ok(deleted)
    }

    /// Stores the latest price of many markets with a single statement per chunk.
    ///
    /// Prices of markets that no longer exist, and non-positive or non-finite prices, are skipped. When
    /// a market appears several times the last price wins. Returns the number of stored prices.
    pub async fn upsert_prices(&self, updates: &[MarketPriceUpdate<'_>]) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let stored = upsert_prices(&mut tx, updates).await?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Records why the latest refresh of many markets failed, with one statement per chunk.
    ///
    /// Only `last_error` and `last_error_at` change: the last known price, its
    /// timestamps and its provider stay readable. Failures of markets that no longer
    /// exist are skipped; when a market appears several times the last failure wins.
    /// Returns the number of recorded failures.
    pub async fn record_price_failures(&self, failures: &[MarketPriceFailure<'_>]) -> Result<u64> {
        // A single statement cannot touch one row twice: keep the last failure of each market.
        let failures = failures
            .iter()
            .rev()
            .unique_by(|f| f.market_id)
            .collect::<Vec<_>>();

        let mut tx = self.pool.begin().await?;
        let mut recorded = 0;
        for chunk in failures.chunks(PRICE_CHUNK_SIZE) {
            let ids = chunk
                .iter()
                .map(|f| f.market_id.as_str())
                .collect::<Vec<_>>();
            let errors = chunk
                .iter()
                .map(|f| f.failure.to_stored())
                .collect::<Vec<_>>();

            let result = query(const_format::formatcp!(
                "UPDATE market_price AS p
                 SET last_error = u.error, last_error_at = NOW()
                 FROM UNNEST($1::text[], $2::text[]) AS u(market_id, error)
                 WHERE p.market_id = u.market_id"
            ))
            .bind(&ids)
            .bind(&errors)
            .execute(&mut *tx)
            .await?;
            recorded += result.rows_affected();
        }
        tx.commit().await?;
        Ok(recorded)
    }

    /// Loads every market with its latest price, in no particular order.
    pub async fn load_markets(&self) -> Result<Vec<Market>> {
        let rows = query_as::<_, MarketRow>(SELECT_MARKETS)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(Market::try_from).collect()
    }
}

/// Number of prices written by one statement, so a long pass never holds one huge statement.
const PRICE_CHUNK_SIZE: usize = 50;

/// Replaces the stored price of existing markets, skipping unknown markets.
async fn upsert_prices(
    tx: &mut Transaction<'_, Postgres>,
    updates: &[MarketPriceUpdate<'_>],
) -> Result<u64> {
    // A single statement cannot touch one row twice: keep the last price of each market.
    let updates = updates
        .iter()
        .rev()
        .filter(|u| {
            let usable = u.price.price.is_finite() && u.price.price > 0.0;
            if !usable {
                warn!(
                    mkt = u.market_id,
                    "Skipping non-positive or non-finite price"
                );
            }
            usable
        })
        .unique_by(|u| u.market_id)
        .collect::<Vec<_>>();

    let mut stored = 0;
    for chunk in updates.chunks(PRICE_CHUNK_SIZE) {
        let ids = chunk
            .iter()
            .map(|u| u.market_id.as_str())
            .collect::<Vec<_>>();
        // The shortest round-trip decimal text keeps the f64 exactly through NUMERIC.
        let prices = chunk
            .iter()
            .map(|u| u.price.price.to_string())
            .collect::<Vec<_>>();
        let observed = chunk
            .iter()
            .map(|u| u.price.ts.to_sqlx())
            .collect::<Vec<_>>();
        let providers = chunk
            .iter()
            .map(|u| stored_provider(u.provider))
            .collect::<Vec<_>>();

        let result = query(const_format::formatcp!(
            "UPDATE market_price AS p
             SET price = u.price::numeric,
                 provider = u.provider,
                 observed_at = u.observed_at,
                 fetched_at = NOW(),
                 last_error = NULL,
                 last_error_at = NULL
             FROM UNNEST($1::text[], $2::text[], $3::timestamptz[], $4::smallint[])
                  AS u(market_id, price, observed_at, provider)
             WHERE p.market_id = u.market_id"
        ))
        .bind(&ids)
        .bind(&prices)
        .bind(&observed)
        .bind(&providers)
        .execute(&mut **tx)
        .await?;
        stored += result.rows_affected();
    }

    Ok(stored)
}

/// Converts the price provider into the provider code stored with a price.
fn stored_provider(provider: PriceProvider) -> DbProvider {
    match provider {
        PriceProvider::Kraken => DbProvider::Kraken,
        PriceProvider::Yahoo => DbProvider::YF,
    }
}

/// Returns the `assets_data` key (provider and upper-case symbol) of a catalog asset.
fn catalog_key(asset: &Asset) -> (DbProvider, String) {
    let provider = match asset {
        Asset::Crypto(_) => DbProvider::Kraken,
        Asset::Fiat(_) => DbProvider::YF,
    };
    (provider, asset.id().to_uppercase())
}

/// Upserts catalog assets with one set-based statement.
///
/// Fiat rows (Yahoo cash) replace the metadata of a client-supplied row; crypto
/// rows only mark it confirmed. Provider-confirmed rows are never changed.
async fn upsert_assets<'e>(executor: impl sqlx::PgExecutor<'e>, assets: &[Asset]) -> Result<()> {
    // A single statement cannot touch one row twice: keep the first of each key. Assets
    // whose id contains `/` are skipped: it separates the sides of a market id.
    let assets = assets
        .iter()
        .filter(|asset| {
            let valid = !asset.id().contains('/');
            if !valid {
                warn!(
                    asset = asset.id(),
                    "Skipping asset whose symbol contains '/'"
                );
            }
            valid
        })
        .unique_by(|asset| catalog_key(asset))
        .collect::<Vec<_>>();
    if assets.is_empty() {
        return Ok(());
    }

    let mut ids = Vec::with_capacity(assets.len());
    let mut providers = Vec::with_capacity(assets.len());
    let mut symbols = Vec::with_capacity(assets.len());
    let mut names = Vec::with_capacity(assets.len());
    let mut classes = Vec::with_capacity(assets.len());
    for asset in &assets {
        let (provider, symbol) = catalog_key(asset);
        let (class, name) = match asset {
            Asset::Crypto(a) => (DbAssetClass::Crypto, a.symbol.as_str()),
            Asset::Fiat(a) => (DbAssetClass::Cash, a.symbol.as_str()),
        };
        ids.push(Uuid::now_v7());
        providers.push(i16::from(provider));
        symbols.push(symbol);
        names.push(name);
        classes.push(i16::from(class));
    }

    // The currency of a catalog asset is its own symbol.
    query(
        "INSERT INTO assets_data
             (id, provider, symbol, name, currency, asset_class, metadata_source,
              created_at, updated_at)
         SELECT u.id, u.provider, u.symbol, u.name, u.symbol, u.asset_class, $6, NOW(), NOW()
         FROM UNNEST($1::uuid[], $2::smallint[], $3::text[], $4::text[], $5::smallint[])
              AS u(id, provider, symbol, name, asset_class)
         ON CONFLICT (provider, symbol) DO UPDATE SET
             name = CASE WHEN assets_data.provider = $7
                         THEN EXCLUDED.name ELSE assets_data.name END,
             currency = CASE WHEN assets_data.provider = $7
                             THEN EXCLUDED.currency ELSE assets_data.currency END,
             asset_class = CASE WHEN assets_data.provider = $7
                                THEN EXCLUDED.asset_class ELSE assets_data.asset_class END,
             metadata_source = EXCLUDED.metadata_source,
             updated_at = EXCLUDED.updated_at
         WHERE assets_data.metadata_source = $8",
    )
    .bind(&ids)
    .bind(&providers)
    .bind(&symbols)
    .bind(&names)
    .bind(&classes)
    .bind(DbMetadataSource::ProviderConfirmed)
    .bind(DbProvider::YF)
    .bind(DbMetadataSource::ClientSupplied)
    .execute(executor)
    .await?;

    Ok(())
}

/// Inserts the markets that do not exist yet and stores their prices inside `tx`.
///
/// Fails, leaving `tx` to roll back, when a market's catalog asset is missing.
async fn insert_markets(
    tx: &mut Transaction<'_, Postgres>,
    markets: &[(&Market, PriceProvider)],
) -> Result<()> {
    if markets.is_empty() {
        return Ok(());
    }

    let mut market_ids = Vec::with_capacity(markets.len());
    let mut base_providers = Vec::with_capacity(markets.len());
    let mut base_symbols = Vec::with_capacity(markets.len());
    let mut quote_providers = Vec::with_capacity(markets.len());
    let mut quote_symbols = Vec::with_capacity(markets.len());
    let mut providers = Vec::with_capacity(markets.len());
    for (market, provider) in markets {
        let (base_provider, base_symbol) = catalog_key(&market.base);
        let (quote_provider, quote_symbol) = catalog_key(&market.quote);
        market_ids.push(market.id.as_str());
        base_providers.push(i16::from(base_provider));
        base_symbols.push(base_symbol);
        quote_providers.push(i16::from(quote_provider));
        quote_symbols.push(quote_symbol);
        providers.push(i16::from(stored_provider(*provider)));
    }

    // One statement resolves the assets, inserts the resolvable markets and reports
    // the markets whose assets are missing.
    let missing = query_scalar::<_, String>(
        "WITH w AS (
             SELECT * FROM UNNEST($1::text[], $2::smallint[], $3::text[], $4::smallint[],
                                  $5::text[], $6::smallint[])
                  AS w(market_id, base_provider, base_symbol, quote_provider, quote_symbol, provider)
         ), resolved AS (
             SELECT w.market_id, b.id AS base_id, q.id AS quote_id, w.provider
             FROM w
             LEFT JOIN assets_data AS b
               ON b.provider = w.base_provider AND b.symbol = w.base_symbol
             LEFT JOIN assets_data AS q
               ON q.provider = w.quote_provider AND q.symbol = w.quote_symbol
         ), inserted AS (
             INSERT INTO market_price (market_id, base_asset_id, quote_asset_id, provider)
             SELECT market_id, base_id, quote_id, provider FROM resolved
             WHERE base_id IS NOT NULL AND quote_id IS NOT NULL
             ON CONFLICT DO NOTHING
         )
         SELECT market_id FROM resolved WHERE base_id IS NULL OR quote_id IS NULL",
    )
    .bind(&market_ids)
    .bind(&base_providers)
    .bind(&base_symbols)
    .bind(&quote_providers)
    .bind(&quote_symbols)
    .bind(&providers)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| {
        warn!("Failed to store {} markets: {}", markets.len(), ErrorChain(&e));
        DcaError::RepositoryStoreFailure(market_ids[0].to_string(), Some(e))
    })?;
    if let Some(first) = missing.first() {
        warn!(mkt = first, "Failed to store market: asset not stored");
        return Err(DcaError::RepositoryStoreFailure(first.clone(), None));
    }

    let updates = markets
        .iter()
        .filter_map(|(market, provider)| {
            (*market.price()).map(|price| MarketPriceUpdate {
                market_id: &market.id,
                price,
                provider: *provider,
            })
        })
        .collect::<Vec<_>>();
    upsert_prices(tx, &updates).await?;

    debug!("Successfully stored {} markets", markets.len());
    Ok(())
}

impl From<QuoteRow> for StoredQuote {
    /// Builds the stored quote of a market from its `market_price` row.
    fn from(row: QuoteRow) -> Self {
        let mut failure = row.last_error.as_deref().map(PriceFailure::from_stored);
        let mut failed_at = row.last_error_at.map(|t| t.to_jiff());
        // A stored zero, negative or non-finite price (legacy rows) is no price.
        let price = row.price.and_then(|p| match validate_price(p) {
            Ok(p) => Some(p),
            Err(invalid) => {
                failure.get_or_insert(invalid);
                failed_at = failed_at.or_else(|| row.fetched_at.map(|t| t.to_jiff()));
                None
            }
        });
        Self {
            base: row.base_symbol.to_lowercase(),
            quote: row.quote_symbol.to_lowercase(),
            provider: match row.provider {
                DbProvider::YF => AppProvider::YF,
                DbProvider::Kraken => AppProvider::Kraken,
            },
            price,
            observed_at: row.observed_at.map(|t| t.to_jiff()),
            fetched_at: row.fetched_at.map(|t| t.to_jiff()),
            failure,
            failed_at,
        }
    }
}

impl TryFrom<AssetRow> for Asset {
    type Error = DcaError;

    /// Builds a catalog asset from a stored row, rejecting rows outside the catalog.
    fn try_from(row: AssetRow) -> Result<Self> {
        asset_from_parts(row.provider, row.asset_class, row.symbol, row.name)
    }
}

/// Builds a catalog asset from its stored columns: Kraken crypto or Yahoo cash.
fn asset_from_parts(
    provider: DbProvider,
    asset_class: DbAssetClass,
    symbol: String,
    name: String,
) -> Result<Asset> {
    let id = symbol.to_lowercase();
    match (provider, asset_class) {
        (DbProvider::Kraken, DbAssetClass::Crypto) => {
            Ok(Asset::Crypto(Crypto { id, symbol: name }))
        }
        (DbProvider::YF, DbAssetClass::Cash) => Ok(Asset::Fiat(Fiat { id, symbol: name })),
        _ => Err(DcaError::Generic(format!(
            "Stored asset '{id}' is not a market asset (provider {provider:?}, class {asset_class:?})"
        ))),
    }
}

impl TryFrom<MarketRow> for Market {
    type Error = DcaError;

    /// Builds a market, with its catalog assets and latest price, from a joined row.
    fn try_from(row: MarketRow) -> Result<Self> {
        let base = asset_from_parts(
            row.base_provider,
            row.base_class,
            row.base_symbol,
            row.base_name,
        )?;
        let quote = asset_from_parts(
            row.quote_provider,
            row.quote_class,
            row.quote_symbol,
            row.quote_name,
        )?;
        let price = row
            .price
            .zip(row.observed_at)
            .map(|(price, ts)| Price::new(price, ts.to_jiff()));

        let id = market_id(base.id(), quote.id());
        Ok(Market::new(id, base, quote, price))
    }
}
