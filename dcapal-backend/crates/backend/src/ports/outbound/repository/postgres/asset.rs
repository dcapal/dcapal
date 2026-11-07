use std::collections::HashSet;

use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder, query_as};
use uuid::Uuid;

use crate::{
    app::domain::asset::{AssetRef, MetadataSource, ResolvedAsset, StoredAsset},
    ports::outbound::repository::{
        asset::{AssetDirectory, AssetDirectoryError},
        postgres::{
            convert::unnest_refs,
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
    provider: DbProvider,
    symbol: String,
    name: String,
    currency: String,
    asset_class: DbAssetClass,
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
        builder.push_values(assets, |mut row, asset| {
            row.push_bind(Uuid::now_v7())
                .push_bind(DbProvider::from(&asset.provider))
                .push_bind(&asset.symbol)
                .push_bind(&asset.name)
                .push_bind(&asset.currency)
                .push_bind(DbAssetClass::from(asset.asset_class))
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

/// Decodes a stored row; only the metadata source is validated here, because
/// the other enum columns are decoded by SQLx when the row is fetched.
fn stored_asset(row: AssetRow) -> Result<StoredAsset, AssetDirectoryError> {
    let source = MetadataSource::try_from(row.metadata_source).map_err(|_| {
        AssetDirectoryError::CorruptData(format!(
            "unknown metadata source code {}",
            row.metadata_source
        ))
    })?;
    Ok(StoredAsset {
        asset: ResolvedAsset {
            provider: row.provider.into(),
            symbol: row.symbol,
            name: row.name,
            currency: row.currency,
            asset_class: row.asset_class.into(),
        },
        source,
    })
}
