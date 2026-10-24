use dcapal_backend::app::domain::{
    asset::{AssetRef, MetadataSource, ResolvedAsset, StoredAsset},
    portfolio::{AssetClass, Provider},
};
use dcapal_backend::ports::outbound::repository::{
    asset::AssetDirectory, postgres::SqlxAssetDirectory,
};
use sqlx::PgPool;

fn asset(provider: Provider, symbol: &str, name: &str) -> ResolvedAsset {
    ResolvedAsset {
        provider,
        symbol: symbol.to_string(),
        name: name.to_string(),
        currency: "EUR".to_string(),
        asset_class: AssetClass::Equity,
    }
}

fn stored(asset: ResolvedAsset, source: MetadataSource) -> StoredAsset {
    StoredAsset { asset, source }
}

/// Inserts a row the way a v1 (or pre-provenance) backend does: without naming
/// `metadata_source`, so the column default applies.
async fn insert_client_supplied(
    pool: &PgPool,
    asset: &ResolvedAsset,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO assets_data (id, provider, symbol, name, currency, asset_class)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
    )
    .bind(match asset.provider {
        Provider::YF => 2_i16,
        Provider::Kraken => 1_i16,
    })
    .bind(&asset.symbol)
    .bind(&asset.name)
    .bind(&asset.currency)
    .bind(1_i16) // Equity
    .execute(pool)
    .await?;
    Ok(())
}

async fn find(
    directory: &SqlxAssetDirectory,
    asset: &ResolvedAsset,
) -> Result<StoredAsset, Box<dyn std::error::Error>> {
    let mut found = directory.find_by_refs(&[asset.as_ref()]).await?;
    Ok(found.pop().expect("asset should be stored"))
}

/// GIVEN an empty directory, WHEN resolved assets are upserted and then read by reference,
/// THEN the stored metadata is returned as provider-confirmed and unknown references are omitted.
#[sqlx::test(migrations = "../../migrations")]
async fn stores_and_finds_assets_by_reference(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool);
    let vwce = asset(Provider::YF, "VWCE.DE", "Vanguard FTSE All-World");
    let btc = ResolvedAsset {
        asset_class: AssetClass::Crypto,
        ..asset(Provider::Kraken, "BTC", "BTC")
    };

    directory
        .upsert_resolved(&[vwce.clone(), btc.clone()])
        .await?;
    let mut found = directory
        .find_by_refs(&[
            AssetRef::new(Provider::YF, "vwce.de"),
            AssetRef::new(Provider::Kraken, "btc"),
            AssetRef::new(Provider::YF, "BTC"),
        ])
        .await?;
    found.sort_by(|a, b| a.asset.symbol.cmp(&b.asset.symbol));

    assert_eq!(
        found,
        vec![
            stored(btc, MetadataSource::ProviderConfirmed),
            stored(vwce, MetadataSource::ProviderConfirmed)
        ]
    );
    Ok(())
}

/// GIVEN a provider-confirmed asset, WHEN another resolution upserts the same provider and
/// symbol with different metadata, THEN the upsert succeeds and the first confirmed metadata remains.
#[sqlx::test(migrations = "../../migrations")]
async fn upsert_never_overwrites_provider_confirmed_rows(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool);
    let first = asset(Provider::YF, "AAPL", "Apple Inc.");
    let second = asset(Provider::YF, "AAPL", "Renamed");
    let other = asset(Provider::YF, "MSFT", "Microsoft");

    directory
        .upsert_resolved(std::slice::from_ref(&first))
        .await?;
    directory.upsert_resolved(&[second, other.clone()]).await?;
    directory.upsert_resolved(&[]).await?;

    assert_eq!(
        find(&directory, &first).await?,
        stored(first, MetadataSource::ProviderConfirmed)
    );
    assert_eq!(
        find(&directory, &other).await?,
        stored(other, MetadataSource::ProviderConfirmed)
    );
    Ok(())
}

/// GIVEN a client-supplied row, WHEN the provider's metadata is upserted,
/// THEN name, currency and class are replaced and the row becomes provider-confirmed.
#[sqlx::test(migrations = "../../migrations")]
async fn upsert_overwrites_client_supplied_rows_and_confirms_them(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool.clone());
    let client = asset(Provider::YF, "AAPL", "bogus name");
    insert_client_supplied(&pool, &client).await?;
    assert_eq!(
        find(&directory, &client).await?,
        stored(client.clone(), MetadataSource::ClientSupplied)
    );
    let official = ResolvedAsset {
        currency: "USD".to_string(),
        asset_class: AssetClass::Crypto,
        ..asset(Provider::YF, "AAPL", "Apple Inc.")
    };

    directory
        .upsert_resolved(std::slice::from_ref(&official))
        .await?;

    assert_eq!(
        find(&directory, &official).await?,
        stored(official, MetadataSource::ProviderConfirmed)
    );
    Ok(())
}

/// GIVEN a client-supplied row and a provider-confirmed row, WHEN both are confirmed,
/// THEN the client-supplied row flips to provider-confirmed with its metadata untouched
/// and the confirmed row is unchanged.
#[sqlx::test(migrations = "../../migrations")]
async fn confirm_flips_the_source_without_touching_metadata(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool.clone());
    let client = asset(Provider::Kraken, "ETH", "Ether (client)");
    let verified = asset(Provider::YF, "AAPL", "Apple Inc.");
    insert_client_supplied(&pool, &client).await?;
    directory
        .upsert_resolved(std::slice::from_ref(&verified))
        .await?;

    directory
        .confirm(&[client.as_ref(), verified.as_ref()])
        .await?;
    directory.confirm(&[]).await?;

    assert_eq!(
        find(&directory, &client).await?,
        stored(client, MetadataSource::ProviderConfirmed)
    );
    assert_eq!(
        find(&directory, &verified).await?,
        stored(verified, MetadataSource::ProviderConfirmed)
    );
    Ok(())
}

/// GIVEN no references, WHEN the directory is read, THEN the result is empty.
#[sqlx::test(migrations = "../../migrations")]
async fn finding_no_references_returns_nothing(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool);

    assert!(directory.find_by_refs(&[]).await?.is_empty());
    Ok(())
}
