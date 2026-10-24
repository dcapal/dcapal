use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder, query_as};
use uuid::Uuid;

use crate::{
    app::domain::asset::{AssetRef, ResolvedAsset},
    ports::outbound::repository::{
        asset::{AssetDirectory, AssetDirectoryError},
        postgres::{
            convert::{app_asset_class, app_provider, db_asset_class, db_provider},
            types::{AssetClass as DbAssetClass, Provider as DbProvider},
        },
    },
};

/// PostgreSQL implementation of the shared asset directory.
#[derive(Clone)]
pub struct SqlxAssetDirectory {
    pool: PgPool,
}

impl SqlxAssetDirectory {
    /// Creates an asset directory backed by the provided PostgreSQL pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    provider: i16,
    symbol: String,
    name: String,
    currency: String,
    asset_class: i16,
}

#[async_trait]
impl AssetDirectory for SqlxAssetDirectory {
    async fn find_by_refs(
        &self,
        refs: &[AssetRef],
    ) -> Result<Vec<ResolvedAsset>, AssetDirectoryError> {
        if refs.is_empty() {
            return Ok(Vec::new());
        }

        let mut providers = Vec::with_capacity(refs.len());
        let mut symbols = Vec::with_capacity(refs.len());
        for reference in refs {
            providers.push(i16::from(db_provider(&reference.provider)));
            symbols.push(reference.symbol.clone());
        }

        let rows = query_as::<_, AssetRow>(
            "SELECT ad.provider, ad.symbol, ad.name, ad.currency, ad.asset_class
             FROM assets_data AS ad
             JOIN UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
               ON ad.provider = wanted.provider AND ad.symbol = wanted.symbol",
        )
        .bind(providers)
        .bind(symbols)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(resolved_asset).collect()
    }

    async fn insert_missing(&self, assets: &[ResolvedAsset]) -> Result<(), AssetDirectoryError> {
        if assets.is_empty() {
            return Ok(());
        }

        let mut builder = QueryBuilder::new(
            "INSERT INTO assets_data
                 (id, provider, symbol, name, currency, asset_class, created_at, updated_at) ",
        );
        let now = chrono::Utc::now();
        let mut values = Vec::with_capacity(assets.len());
        for asset in assets {
            values.push((
                Uuid::now_v7(),
                i16::from(db_provider(&asset.provider)),
                i16::from(db_asset_class(asset.asset_class)),
                asset,
            ));
        }
        builder.push_values(values, |mut row, (id, provider, class, asset)| {
            row.push_bind(id)
                .push_bind(provider)
                .push_bind(&asset.symbol)
                .push_bind(&asset.name)
                .push_bind(&asset.currency)
                .push_bind(class)
                .push_bind(now)
                .push_bind(now);
        });
        builder.push(" ON CONFLICT (provider, symbol) DO NOTHING");

        builder.build().execute(&self.pool).await?;
        Ok(())
    }
}

fn resolved_asset(row: AssetRow) -> Result<ResolvedAsset, AssetDirectoryError> {
    let provider = DbProvider::try_from(row.provider).map_err(|_| {
        AssetDirectoryError::CorruptData(format!("unknown provider code {}", row.provider))
    })?;
    let class = DbAssetClass::try_from(row.asset_class).map_err(|_| {
        AssetDirectoryError::CorruptData(format!("unknown asset class code {}", row.asset_class))
    })?;

    Ok(ResolvedAsset {
        provider: app_provider(provider),
        symbol: row.symbol,
        name: row.name,
        currency: row.currency,
        asset_class: app_asset_class(class),
    })
}
