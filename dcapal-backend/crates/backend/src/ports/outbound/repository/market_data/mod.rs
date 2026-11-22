//! PostgreSQL-backed storage for the market-asset catalog, markets and their
//! latest prices.
//!
//! Market assets are rows of the shared `assets_data` table: crypto assets listed
//! by Kraken (provider Kraken, class Crypto) and the fiat currencies of the fixed
//! catalog (provider Yahoo, class Cash). A market is a row of `market_price`
//! keyed by its base and quote `assets_data` ids. The v1 REST API still exposes
//! the lower-case symbol as asset id and the concatenation of base and quote ids
//! as market id, so lookups by those ids compare against `LOWER(symbol)`.
//!
//! Only provider-confirmed rows belong to the catalog: assets that clients
//! supplied through synchronization never show up in it.

use std::collections::HashSet;

use itertools::Itertools;
use jiff_sqlx::ToSqlx;
use sqlx::{PgPool, Postgres, Transaction, query, query_as, query_scalar};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    app::domain::{
        asset::{AssetRef, ResolvedAsset},
        entity::{Asset, AssetId, AssetKind, Crypto, Fiat, Market, MarketId, Price},
        fiat::fiat_symbols,
        portfolio::{AssetClass, Provider},
    },
    config::PriceProvider,
    error::{DcaError, Result},
    ports::outbound::repository::{
        asset::{AssetDirectory, AssetDirectoryError},
        postgres::{
            SqlxAssetDirectory,
            types::{
                AssetClass as DbAssetClass, MetadataSource as DbMetadataSource,
                Provider as DbProvider,
            },
        },
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

/// Reads and writes the market-asset catalog, markets and latest prices.
#[derive(Clone)]
pub struct MarketDataRepository {
    pool: PgPool,
    assets: SqlxAssetDirectory,
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

#[derive(sqlx::FromRow)]
struct AssetRow {
    provider: DbProvider,
    asset_class: DbAssetClass,
    symbol: String,
    name: String,
}

/// SQL expression of the v1 market id of a joined `market_price` row.
macro_rules! market_id_sql {
    () => {
        "LOWER(b.symbol) || LOWER(q.symbol)"
    };
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
        Self {
            assets: SqlxAssetDirectory::new(pool.clone()),
            pool,
        }
    }

    /// Finds a catalog asset by its lower-case id (the lower-cased symbol).
    pub async fn find_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        let row = query_as::<_, AssetRow>(const_format::formatcp!(
            "{SELECT_ASSETS} WHERE ad.symbol = UPPER($1) AND ad.metadata_source = $4 AND (
                 (ad.provider = $2 AND ad.asset_class = $3)
                 OR (ad.provider = $5 AND ad.asset_class = $6 AND ad.symbol = ANY($7)))"
        ))
        .bind(id)
        .bind(DbProvider::Kraken)
        .bind(DbAssetClass::Crypto)
        .bind(DbMetadataSource::ProviderConfirmed)
        .bind(DbProvider::YF)
        .bind(DbAssetClass::Cash)
        .bind(fiat_symbols())
        .fetch_optional(&self.pool)
        .await?;

        row.map(Asset::try_from).transpose()
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

    /// Stores a catalog asset in `assets_data`.
    ///
    /// Missing assets are inserted as provider-confirmed. Provider-confirmed rows
    /// are never changed. A client-supplied fiat row is replaced by the catalog's
    /// English name and class; a client-supplied crypto row keeps the name the
    /// client chose (Kraken only knows the symbol) and is just marked confirmed.
    pub async fn store_asset(&self, asset: &Asset) -> Result<()> {
        let resolved = resolved_from_asset(asset);
        let reference = AssetRef::from(&resolved);

        match asset {
            Asset::Fiat(_) => self
                .assets
                .upsert_resolved(&[resolved])
                .await
                .map_err(directory_error)?,
            Asset::Crypto(_) => {
                let known = self
                    .assets
                    .find_by_refs(std::slice::from_ref(&reference))
                    .await
                    .map_err(directory_error)?;
                if known.is_empty() {
                    self.assets
                        .upsert_resolved(&[resolved])
                        .await
                        .map_err(directory_error)?;
                }
                self.assets
                    .confirm(&[reference])
                    .await
                    .map_err(directory_error)?;
            }
        }

        debug!("Successfully stored market asset '{}'", asset.id());
        Ok(())
    }

    /// Finds a market, with its latest price when one has been stored.
    pub async fn find_market(&self, id: &MarketId) -> Result<Option<Market>> {
        let row = query_as::<_, MarketRow>(const_format::formatcp!(
            "{SELECT_MARKETS} WHERE {} = $1",
            market_id_sql!()
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Market::try_from).transpose()
    }

    /// Finds several markets at once.
    ///
    /// The result is aligned with `ids`: unknown markets are `None`.
    pub async fn find_markets(&self, ids: &[&MarketId]) -> Result<Vec<Option<Market>>> {
        let wanted = ids.iter().map(|id| id.as_str()).collect::<Vec<_>>();
        let rows = query_as::<_, MarketRow>(const_format::formatcp!(
            "{SELECT_MARKETS} WHERE {} = ANY($1)",
            market_id_sql!()
        ))
        .bind(&wanted)
        .fetch_all(&self.pool)
        .await?;

        let mut found = rows
            .into_iter()
            .map(|row| Market::try_from(row).map(|m| (m.id.clone(), m)))
            .collect::<Result<std::collections::HashMap<_, _>>>()?;

        Ok(ids.iter().map(|id| found.remove(*id)).collect())
    }

    /// Inserts a market if it does not exist yet, and stores its price when it has one.
    ///
    /// Both catalog assets must already be stored. `provider` is the provider that
    /// prices the market; an existing market keeps its stored provider until a
    /// price is stored.
    pub async fn store_market(&self, market: &Market, provider: PriceProvider) -> Result<()> {
        let stored_provider = stored_provider(provider)?;
        let (base_provider, base_symbol) = catalog_key(&market.base);
        let (quote_provider, quote_symbol) = catalog_key(&market.quote);
        let mut tx = self.pool.begin().await?;

        let ids = query_as::<_, (Uuid, Uuid)>(
            "SELECT b.id, q.id
             FROM assets_data AS b, assets_data AS q
             WHERE b.provider = $1 AND b.symbol = $2
               AND q.provider = $3 AND q.symbol = $4",
        )
        .bind(base_provider)
        .bind(&base_symbol)
        .bind(quote_provider)
        .bind(&quote_symbol)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((base_id, quote_id)) = ids else {
            warn!(mkt = market.id, "Failed to store market: asset not stored");
            return Err(DcaError::RepositoryStoreFailure(market.id.clone()));
        };

        query(
            "INSERT INTO market_price (base_asset_id, quote_asset_id, provider)
             VALUES ($1, $2, $3)
             ON CONFLICT (base_asset_id, quote_asset_id) DO NOTHING",
        )
        .bind(base_id)
        .bind(quote_id)
        .bind(stored_provider)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            warn!(mkt = market.id, "Failed to store market: {e}");
            DcaError::RepositoryStoreFailure(market.id.clone())
        })?;

        if let Some(price) = market.price() {
            upsert_prices(
                &mut tx,
                &[MarketPriceUpdate {
                    market_id: &market.id,
                    price: *price,
                    provider,
                }],
            )
            .await?;
        }

        tx.commit().await?;
        debug!("Successfully stored market '{}'", market.id);
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
               AND {id} <> ALL($1)
               AND NOT (b.provider = $2 AND b.asset_class = $3 AND b.symbol = ANY($4)
                    AND q.provider = $2 AND q.asset_class = $3 AND q.symbol = ANY($4))
             RETURNING {id}",
            id = market_id_sql!()
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
    /// Prices of markets that no longer exist, and non-finite prices, are skipped. When
    /// a market appears several times the last price wins. Returns the number of stored prices.
    pub async fn upsert_prices(&self, updates: &[MarketPriceUpdate<'_>]) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let stored = upsert_prices(&mut tx, updates).await?;
        tx.commit().await?;
        Ok(stored)
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
            let finite = u.price.price.is_finite();
            if !finite {
                warn!(mkt = u.market_id, "Skipping non-finite price");
            }
            finite
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
            .collect::<Result<Vec<_>>>()?;

        let result = query(const_format::formatcp!(
            "UPDATE market_price AS p
             SET price = u.price::numeric,
                 provider = u.provider,
                 observed_at = u.observed_at,
                 fetched_at = NOW(),
                 last_error = NULL,
                 last_error_at = NULL
             FROM UNNEST($1::text[], $2::text[], $3::timestamptz[], $4::smallint[])
                  AS u(market_id, price, observed_at, provider),
                  assets_data AS b, assets_data AS q
             WHERE b.id = p.base_asset_id AND q.id = p.quote_asset_id
               AND {} = u.market_id",
            market_id_sql!()
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

/// Converts the configured price provider into the provider code stored with a price.
///
/// CryptoWatch has no code in `assets_data.provider`, so its prices cannot be stored.
fn stored_provider(provider: PriceProvider) -> Result<DbProvider> {
    match provider {
        PriceProvider::Kraken => Ok(DbProvider::Kraken),
        PriceProvider::Yahoo => Ok(DbProvider::YF),
        PriceProvider::CryptoWatch => Err(DcaError::Generic(
            "CryptoWatch prices cannot be stored: it has no provider code".to_string(),
        )),
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

/// Builds the shared asset metadata a catalog asset is stored with.
///
/// The v1 `symbol` of a catalog asset (display name) is the stored name, and its
/// currency is its own upper-case symbol.
fn resolved_from_asset(asset: &Asset) -> ResolvedAsset {
    let symbol = asset.id().to_uppercase();
    let (provider, asset_class, name) = match asset {
        Asset::Crypto(a) => (Provider::Kraken, AssetClass::Crypto, a.symbol.clone()),
        Asset::Fiat(a) => (Provider::YF, AssetClass::Cash, a.symbol.clone()),
    };

    ResolvedAsset {
        provider,
        currency: symbol.clone(),
        symbol,
        name,
        asset_class,
    }
}

/// Converts asset-directory failures into application errors.
fn directory_error(error: AssetDirectoryError) -> DcaError {
    match error {
        AssetDirectoryError::Database(e) => DcaError::DatabaseError(e),
        AssetDirectoryError::CorruptData(message) => DcaError::Generic(message),
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

        let id = format!("{}{}", base.id(), quote.id());
        Ok(Market::new(id, base, quote, price))
    }
}
