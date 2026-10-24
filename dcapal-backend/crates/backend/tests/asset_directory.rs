use dcapal_backend::app::domain::{
    asset::{AssetRef, ResolvedAsset},
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

/// GIVEN an empty directory, WHEN assets are inserted and then read by reference,
/// THEN the stored metadata is returned and unknown references are omitted.
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
        .insert_missing(&[vwce.clone(), btc.clone()])
        .await?;
    let mut found = directory
        .find_by_refs(&[
            AssetRef::new(Provider::YF, "vwce.de"),
            AssetRef::new(Provider::Kraken, "btc"),
            AssetRef::new(Provider::YF, "BTC"),
        ])
        .await?;
    found.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    assert_eq!(found, vec![btc, vwce]);
    Ok(())
}

/// GIVEN a stored asset, WHEN another writer inserts the same provider and symbol with
/// different metadata, THEN the insert succeeds and the first writer's metadata remains.
#[sqlx::test(migrations = "../../migrations")]
async fn insert_missing_is_idempotent_and_first_writer_wins(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = SqlxAssetDirectory::new(pool);
    let first = asset(Provider::YF, "AAPL", "Apple Inc.");
    let second = asset(Provider::YF, "AAPL", "Renamed");
    let other = asset(Provider::YF, "MSFT", "Microsoft");

    directory
        .insert_missing(std::slice::from_ref(&first))
        .await?;
    directory.insert_missing(&[second, other.clone()]).await?;
    directory.insert_missing(&[]).await?;
    let mut found = directory
        .find_by_refs(&[first.as_ref(), other.as_ref()])
        .await?;
    found.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    assert_eq!(found, vec![first, other]);
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
