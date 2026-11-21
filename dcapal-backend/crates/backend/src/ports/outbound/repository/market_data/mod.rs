//! PostgreSQL-backed storage for the market-asset catalog, markets and their
//! latest prices.
//!
//! Identifiers are the lower-case provider-derived ids the v1 REST API exposes,
//! so they are stored as natural text keys.

use std::collections::HashSet;

use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sqlx::{PgPool, Postgres, Transaction, query, query_as, query_scalar};
use tracing::{debug, warn};

use crate::{
    DateTime,
    app::domain::entity::{Asset, AssetId, AssetKind, Crypto, Fiat, Market, MarketId, Price},
    config::PriceProvider,
    error::{DcaError, Result},
};

/// The asset-kind codes persisted in `market_asset.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, sqlx::Type)]
#[repr(i16)]
enum DbAssetKind {
    Crypto = 0,
    Fiat = 1,
}

impl From<AssetKind> for DbAssetKind {
    fn from(kind: AssetKind) -> Self {
        match kind {
            AssetKind::Crypto => Self::Crypto,
            AssetKind::Fiat => Self::Fiat,
        }
    }
}

/// A freshly observed price for one market, ready to be stored.
#[derive(Debug, Clone, Copy)]
pub struct MarketPriceUpdate<'a> {
    /// The market the price belongs to.
    pub market_id: &'a MarketId,
    /// The observed price and its observation time.
    pub price: Price,
}

/// Reads and writes the market-asset catalog, markets and latest prices.
#[derive(Clone)]
pub struct MarketDataRepository {
    pool: PgPool,
}

/// A market joined with its two catalog assets and, when present, its latest price.
#[derive(sqlx::FromRow)]
struct MarketRow {
    id: String,
    pair: String,
    base_id: String,
    base_kind: i16,
    base_symbol: String,
    quote_id: String,
    quote_kind: i16,
    quote_symbol: String,
    price: Option<f64>,
    observed_at: Option<DateTime>,
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: String,
    kind: i16,
    symbol: String,
}

const SELECT_MARKETS: &str = r#"
    SELECT m.id, m.pair,
           b.id AS base_id, b.kind AS base_kind, b.symbol AS base_symbol,
           q.id AS quote_id, q.kind AS quote_kind, q.symbol AS quote_symbol,
           p.price::float8 AS price, p.observed_at
    FROM market AS m
    JOIN market_asset AS b ON b.id = m.base_asset_id
    JOIN market_asset AS q ON q.id = m.quote_asset_id
    LEFT JOIN market_price AS p ON p.market_id = m.id
"#;

impl MarketDataRepository {
    /// Creates a repository backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Finds a catalog asset by its lower-case id.
    pub async fn find_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        let row =
            query_as::<_, AssetRow>("SELECT id, kind, symbol FROM market_asset WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(asset_from_row).transpose()
    }

    /// Loads every catalog asset of a kind, ordered by id in byte order.
    pub async fn load_assets_by_type(&self, kind: AssetKind) -> Result<Vec<Asset>> {
        let rows = query_as::<_, AssetRow>(
            r#"SELECT id, kind, symbol FROM market_asset
               WHERE kind = $1
               ORDER BY id COLLATE "C""#,
        )
        .bind(DbAssetKind::from(kind))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(asset_from_row).collect()
    }

    /// Tells whether the catalog holds at least one crypto or fiat asset.
    pub async fn has_assets(&self) -> Result<bool> {
        Ok(
            query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM market_asset)")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Inserts a catalog asset, or replaces the stored one with the same id.
    pub async fn store_asset(&self, asset: &Asset) -> Result<()> {
        let symbol = match asset {
            Asset::Crypto(a) => &a.symbol,
            Asset::Fiat(a) => &a.symbol,
        };

        query(
            "INSERT INTO market_asset (id, kind, symbol) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE
             SET kind = EXCLUDED.kind, symbol = EXCLUDED.symbol, updated_at = NOW()",
        )
        .bind(asset.id())
        .bind(DbAssetKind::from(asset.kind()))
        .bind(symbol)
        .execute(&self.pool)
        .await?;

        debug!("Successfully stored market asset '{}'", asset.id());
        Ok(())
    }

    /// Finds a market, with its latest price when one has been stored.
    pub async fn find_market(&self, id: &MarketId) -> Result<Option<Market>> {
        let row =
            query_as::<_, MarketRow>(const_format::formatcp!("{SELECT_MARKETS} WHERE m.id = $1"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(market_from_row).transpose()
    }

    /// Finds several markets at once.
    ///
    /// The result is aligned with `ids`: unknown markets are `None`.
    pub async fn find_markets(&self, ids: &[&MarketId]) -> Result<Vec<Option<Market>>> {
        let wanted = ids.iter().map(|id| id.as_str()).collect::<Vec<_>>();
        let rows = query_as::<_, MarketRow>(const_format::formatcp!(
            "{SELECT_MARKETS} WHERE m.id = ANY($1)"
        ))
        .bind(&wanted)
        .fetch_all(&self.pool)
        .await?;

        let mut found = rows
            .into_iter()
            .map(|row| market_from_row(row).map(|m| (m.id.clone(), m)))
            .collect::<Result<std::collections::HashMap<_, _>>>()?;

        Ok(ids.iter().map(|id| found.remove(*id)).collect())
    }

    /// Inserts a market, or updates its pair and assets, and stores its price when it has one.
    ///
    /// Both catalog assets must already be stored.
    pub async fn store_market(&self, market: &Market, provider: PriceProvider) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        query(
            "INSERT INTO market (id, pair, base_asset_id, quote_asset_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE
             SET pair = EXCLUDED.pair,
                 base_asset_id = EXCLUDED.base_asset_id,
                 quote_asset_id = EXCLUDED.quote_asset_id,
                 updated_at = NOW()",
        )
        .bind(&market.id)
        .bind(&market.pair)
        .bind(market.base.id())
        .bind(market.quote.id())
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
                }],
                provider,
            )
            .await?;
        }

        tx.commit().await?;
        debug!("Successfully stored market '{}'", market.id);
        Ok(())
    }

    /// Deletes markets absent from a complete provider snapshot, with their prices.
    ///
    /// Returns the deleted market ids. An empty snapshot is rejected so that a failed
    /// provider call can never wipe every market.
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
        let deleted =
            query_scalar::<_, String>("DELETE FROM market WHERE id <> ALL($1) RETURNING id")
                .bind(&keep)
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
    pub async fn upsert_prices(
        &self,
        updates: &[MarketPriceUpdate<'_>],
        provider: PriceProvider,
    ) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let stored = upsert_prices(&mut tx, updates, provider).await?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Loads every market with its latest price, ordered by id.
    pub async fn load_markets(&self) -> Result<Vec<Market>> {
        let rows = query_as::<_, MarketRow>(const_format::formatcp!(
            r#"{SELECT_MARKETS} ORDER BY m.id COLLATE "C""#
        ))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(market_from_row).collect()
    }
}

/// Number of prices written by one statement, so a long pass never holds one huge statement.
const PRICE_CHUNK_SIZE: usize = 50;

async fn upsert_prices(
    tx: &mut Transaction<'_, Postgres>,
    updates: &[MarketPriceUpdate<'_>],
    provider: PriceProvider,
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
        let observed = chunk.iter().map(|u| u.price.ts).collect::<Vec<_>>();

        let result = query(
            "INSERT INTO market_price (market_id, price, provider, observed_at, fetched_at)
             SELECT p.market_id, p.price::numeric, $4, p.observed_at, NOW()
             FROM UNNEST($1::text[], $2::text[], $3::timestamptz[])
                  AS p(market_id, price, observed_at)
             JOIN market AS m ON m.id = p.market_id
             ON CONFLICT (market_id) DO UPDATE
             SET price = EXCLUDED.price,
                 provider = EXCLUDED.provider,
                 observed_at = EXCLUDED.observed_at,
                 fetched_at = EXCLUDED.fetched_at,
                 last_error = NULL,
                 last_error_at = NULL",
        )
        .bind(&ids)
        .bind(&prices)
        .bind(&observed)
        .bind(provider.to_string())
        .execute(&mut **tx)
        .await?;
        stored += result.rows_affected();
    }

    Ok(stored)
}

fn asset_from_row(row: AssetRow) -> Result<Asset> {
    asset_from_parts(row.id, row.kind, row.symbol)
}

fn asset_from_parts(id: String, kind: i16, symbol: String) -> Result<Asset> {
    match DbAssetKind::try_from(kind) {
        Ok(DbAssetKind::Crypto) => Ok(Asset::Crypto(Crypto { id, symbol })),
        Ok(DbAssetKind::Fiat) => Ok(Asset::Fiat(Fiat { id, symbol })),
        Err(_) => Err(DcaError::Generic(format!(
            "Unknown market asset kind '{kind}' for '{id}'"
        ))),
    }
}

fn market_from_row(row: MarketRow) -> Result<Market> {
    let base = asset_from_parts(row.base_id, row.base_kind, row.base_symbol)?;
    let quote = asset_from_parts(row.quote_id, row.quote_kind, row.quote_symbol)?;
    let price = row
        .price
        .zip(row.observed_at)
        .map(|(price, ts)| Price::new(price, ts));

    let mut market = Market::new(row.id, base, quote, price);
    market.pair = row.pair;
    Ok(market)
}
