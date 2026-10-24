use std::collections::HashSet;

use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder, query_as};
use uuid::Uuid;

use crate::{
    app::domain::asset::{AssetRef, MetadataSource, ResolvedAsset, StoredAsset},
    ports::outbound::repository::{
        asset::{AssetDirectory, AssetDirectoryError},
        postgres::{
            convert::{app_asset_class, app_provider, db_asset_class, db_provider, unnest_refs},
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
    metadata_source: i16,
}

#[async_trait]
impl AssetDirectory for SqlxAssetDirectory {
    async fn find_by_refs(
        &self,
        refs: &[AssetRef],
    ) -> Result<Vec<StoredAsset>, AssetDirectoryError> {
        if refs.is_empty() {
            return Ok(Vec::new());
        }

        let (providers, symbols) = unnest_refs(refs);

        let rows = query_as::<_, AssetRow>(
            "SELECT ad.provider, ad.symbol, ad.name, ad.currency, ad.asset_class,
                    ad.metadata_source
             FROM assets_data AS ad
             JOIN UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
               ON ad.provider = wanted.provider AND ad.symbol = wanted.symbol",
        )
        .bind(providers)
        .bind(symbols)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(stored_asset).collect()
    }

    async fn upsert_resolved(&self, assets: &[ResolvedAsset]) -> Result<(), AssetDirectoryError> {
        // One statement cannot touch the same row twice, so keep the first of any duplicates.
        let mut seen = HashSet::new();
        let assets = assets
            .iter()
            .filter(|asset| seen.insert(asset.as_ref()))
            .collect::<Vec<_>>();
        if assets.is_empty() {
            return Ok(());
        }

        let mut builder = QueryBuilder::new(
            "INSERT INTO assets_data
                 (id, provider, symbol, name, currency, asset_class, metadata_source,
                  created_at, updated_at) ",
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
                .push_bind(i16::from(MetadataSource::ProviderConfirmed))
                .push_bind(now)
                .push_bind(now);
        });
        builder.push(
            " ON CONFLICT (provider, symbol) DO UPDATE SET
                  name = EXCLUDED.name,
                  currency = EXCLUDED.currency,
                  asset_class = EXCLUDED.asset_class,
                  metadata_source = EXCLUDED.metadata_source,
                  updated_at = EXCLUDED.updated_at
              WHERE assets_data.metadata_source = ",
        );
        builder.push_bind(i16::from(MetadataSource::ClientSupplied));

        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn confirm(&self, refs: &[AssetRef]) -> Result<(), AssetDirectoryError> {
        if refs.is_empty() {
            return Ok(());
        }

        let (providers, symbols) = unnest_refs(refs);

        sqlx::query(
            "UPDATE assets_data AS ad
             SET metadata_source = $3, updated_at = NOW()
             FROM UNNEST($1::smallint[], $2::text[]) AS wanted(provider, symbol)
             WHERE ad.provider = wanted.provider
               AND ad.symbol = wanted.symbol
               AND ad.metadata_source = $4",
        )
        .bind(providers)
        .bind(symbols)
        .bind(i16::from(MetadataSource::ProviderConfirmed))
        .bind(i16::from(MetadataSource::ClientSupplied))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn stored_asset(row: AssetRow) -> Result<StoredAsset, AssetDirectoryError> {
    let source = MetadataSource::try_from(row.metadata_source).map_err(|_| {
        AssetDirectoryError::CorruptData(format!(
            "unknown metadata source code {}",
            row.metadata_source
        ))
    })?;
    Ok(StoredAsset {
        asset: resolved_asset(row)?,
        source,
    })
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
