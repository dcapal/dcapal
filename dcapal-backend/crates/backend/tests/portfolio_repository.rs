use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use dcapal_backend::app::commands::sync_portfolios_v2::SyncPortfoliosV2Command;
use dcapal_backend::app::domain::asset::{AssetRef, MetadataSource, ResolvedAsset};
use dcapal_backend::app::domain::portfolio::{
    AllocationMode, AssetClass, PortfolioAssetV2, PortfolioV2, Provider as AppProvider,
    StrategicAllocationV2, StrategicAssetClassTargetV2, StrategicAssetTargetV2,
    SyncPortfoliosV2Request, SyncPortfoliosV2Response,
};
use dcapal_backend::app::domain::portfolio_sync::{WriteKind, plan_sync};
use dcapal_backend::app::services::{asset::AssetService, portfolio::PortfolioService};
use dcapal_backend::ports::{
    inbound::rest::{
        FeeStructure,
        request::{PortfolioAssetRequest, PortfolioRequest, SyncPortfoliosRequest},
    },
    outbound::repository::{
        asset::AssetDirectory,
        portfolio::{OwnershipConflict, PortfolioRepository, PortfolioRepositoryError},
        postgres::{SqlxAssetDirectory, SqlxPortfolioRepository, types::Provider},
    },
};
use rust_decimal::{Decimal, dec};
use sqlx::PgPool;
use uuid::Uuid;

const USER_ID: Uuid = Uuid::from_u128(1);
const OTHER_USER_ID: Uuid = Uuid::from_u128(2);
const PORTFOLIO_ID: Uuid = Uuid::from_u128(0x10000000000000000000000000000001);
const SECOND_PORTFOLIO_ID: Uuid = Uuid::from_u128(0x10000000000000000000000000000002);

fn asset(symbol: &str) -> PortfolioAssetRequest {
    PortfolioAssetRequest {
        symbol: symbol.to_string(),
        name: format!("{symbol} asset"),
        aclass: "EQUITY".to_string(),
        base_ccy: "EUR".to_string(),
        provider: "YF".to_string(),
        qty: dec!(2),
        target_weight: dec!(1),
        price: dec!(120),
        average_buy_price: dec!(110),
        fees: Some(
            dcapal_backend::ports::inbound::rest::request::TransactionFeesRequest {
                max_fee_impact: None,
                fee_structure: FeeStructure::ZeroFee,
            },
        ),
    }
}

fn portfolio_request(assets: Vec<PortfolioAssetRequest>) -> PortfolioRequest {
    PortfolioRequest {
        id: PORTFOLIO_ID,
        name: "Updated portfolio".to_string(),
        quote_ccy: "USD".to_string(),
        fees: Some(
            dcapal_backend::ports::inbound::rest::request::TransactionFeesRequest {
                max_fee_impact: Some(dec!(0.5)),
                fee_structure: FeeStructure::Fixed {
                    fee_amount: dec!(1.25),
                },
            },
        ),
        assets,
        last_updated_at: Utc::now(),
    }
}

const FIXTURE_VWCE_RELATIONSHIP: Uuid = Uuid::from_u128(0x00000000000070008000000000000011);
const FIXTURE_CASH_RELATIONSHIP: Uuid = Uuid::from_u128(0x00000000000070008000000000000012);

fn v2_ref(symbol: &str) -> AssetRef {
    AssetRef::new(AppProvider::YF, symbol)
}

fn v2_asset(symbol: &str) -> PortfolioAssetV2 {
    PortfolioAssetV2 {
        asset: v2_ref(symbol),
        asset_class_override: None,
        quantity: dec!(2.125),
        target_weight: None,
        manual_price: Some(dec!(120.10)),
        average_buy_price: Some(dec!(110.05)),
        fees: None,
    }
}

/// A Simple-mode aggregate for `PORTFOLIO_ID` that is newer than the fixture row.
fn v2_simple_request(assets: Vec<(PortfolioAssetV2, Decimal)>) -> SyncPortfoliosV2Request {
    SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios: vec![PortfolioV2 {
            id: PORTFOLIO_ID,
            name: "Synced portfolio".to_string(),
            quote_currency: "EUR".to_string(),
            allocation_mode: AllocationMode::Simple,
            drift_band: None,
            fees: None,
            assets: assets
                .into_iter()
                .map(|(asset, weight)| PortfolioAssetV2 {
                    target_weight: Some(weight),
                    ..asset
                })
                .collect(),
            strategic_allocation: None,
            last_updated_at: Utc::now(),
        }],
        deleted_portfolios: Vec::new(),
    }
}

/// Stores a YF asset in the shared directory, as the service does before a sync.
async fn onboard(pool: &PgPool, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    SqlxAssetDirectory::new(pool.clone())
        .upsert_resolved(&[ResolvedAsset {
            provider: AppProvider::YF,
            symbol: symbol.to_string(),
            name: format!("{symbol} asset"),
            currency: "EUR".to_string(),
            asset_class: AssetClass::Equity,
        }])
        .await?;
    Ok(())
}

/// Synchronizes `request` for `user_id` the way the service does: loads the
/// snapshot, plans, applies the plan once and returns the planned response.
async fn sync_v2(
    repository: &SqlxPortfolioRepository,
    user_id: Uuid,
    request: &SyncPortfoliosV2Request,
) -> Result<SyncPortfoliosV2Response, PortfolioRepositoryError> {
    let snapshot = repository.load_sync_snapshot(user_id).await?;
    let plan = plan_sync(&snapshot, request);
    repository.apply_sync_plan(user_id, &plan).await?;
    Ok(plan.response)
}

/// Provider stub that knows exactly one symbol, `NEWCO`.
struct NewcoProvider;

#[async_trait]
impl dcapal_backend::ports::outbound::adapter::AssetProvider for NewcoProvider {
    async fn lookup(
        &self,
        asset: &AssetRef,
    ) -> Result<Option<ResolvedAsset>, dcapal_backend::ports::outbound::adapter::AssetProviderError>
    {
        Ok((asset.symbol == "NEWCO").then(|| ResolvedAsset {
            provider: asset.provider.clone(),
            symbol: asset.symbol.clone(),
            name: "New company".to_string(),
            currency: "USD".to_string(),
            asset_class: AssetClass::Equity,
        }))
    }
}

fn asset_service(pool: &PgPool) -> Arc<AssetService> {
    Arc::new(AssetService::new(
        Arc::new(SqlxAssetDirectory::new(pool.clone())),
        Arc::new(NewcoProvider),
    ))
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn reads_portfolios_with_their_assets(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = SqlxPortfolioRepository::new(pool);
    let portfolios = repository.get_user_portfolios_with_assets(USER_ID).await?;

    assert_eq!(portfolios.len(), 1);
    assert_eq!(portfolios[0].0.id, PORTFOLIO_ID);
    assert_eq!(portfolios[0].1.len(), 2);
    assert_eq!(portfolios[0].1[0].symbol, "VWCE");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn rejects_invalid_provider_codes_from_storage(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a canonical portfolio asset with an invalid provider code, WHEN the repository reads it,
    // THEN it returns a column-decode error with the enum conversion cause intact.
    sqlx::query(
        "UPDATE assets_data
         SET provider = 99
         WHERE symbol = 'VWCE'",
    )
    .execute(&pool)
    .await?;

    let error = SqlxPortfolioRepository::new(pool)
        .get_user_portfolios_with_assets(USER_ID)
        .await
        .unwrap_err();

    assert!(matches!(
        &error,
        PortfolioRepositoryError::Database(sqlx::Error::ColumnDecode { index, .. })
            if index == "provider"
    ));
    let database_error = std::error::Error::source(&error).expect("database error source");
    assert!(
        std::error::Error::source(database_error).is_some(),
        "column decode should retain the enum conversion cause"
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn rejects_invalid_asset_class_codes_from_storage(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a canonical portfolio asset with an invalid Asset Class code, WHEN the repository reads it,
    // THEN it returns a column-decode error with the enum conversion cause intact.
    sqlx::query(
        "UPDATE assets_data
         SET asset_class = 99
         WHERE symbol = 'VWCE'",
    )
    .execute(&pool)
    .await?;

    let error = SqlxPortfolioRepository::new(pool)
        .get_user_portfolios_with_assets(USER_ID)
        .await
        .unwrap_err();

    assert!(matches!(
        &error,
        PortfolioRepositoryError::Database(sqlx::Error::ColumnDecode { index, .. })
            if index == "asset_class"
    ));
    let database_error = std::error::Error::source(&error).expect("database error source");
    assert!(
        std::error::Error::source(database_error).is_some(),
        "column decode should retain the enum conversion cause"
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn rejects_corrupt_v2_snapshot_fee_data(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a persisted v2 fee policy with a missing required amount, WHEN a
    // synchronization snapshot is loaded, THEN corruption is returned instead
    // of silently dropping the fee policy.
    sqlx::query(
        "UPDATE portfolio_asset
         SET fee_type = 'Fixed', fee_amount = NULL
         WHERE id = '00000000-0000-7000-8000-000000000012'",
    )
    .execute(&pool)
    .await?;

    let error = SqlxPortfolioRepository::new(pool)
        .load_sync_snapshot(USER_ID)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        PortfolioRepositoryError::CorruptData(message)
            if message.contains("missing feeAmount")
    ));
    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn rejects_corrupt_v2_snapshot_asset_override(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a persisted Portfolio Asset with an unknown Asset Class override,
    // WHEN a synchronization snapshot is loaded, THEN corruption is returned
    // instead of treating the override as null.
    sqlx::query(
        "UPDATE portfolio_asset
         SET asset_class_override = 99
         WHERE id = '00000000-0000-7000-8000-000000000012'",
    )
    .execute(&pool)
    .await?;

    let error = SqlxPortfolioRepository::new(pool)
        .load_sync_snapshot(USER_ID)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        PortfolioRepositoryError::CorruptData(message)
            if message.contains("invalid aclassOverride")
    ));
    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn upsert_updates_assets_and_removes_missing_assets(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let (portfolio, assets) = repository
        .upsert(USER_ID, portfolio_request(vec![asset("VWCE")]))
        .await?;

    assert_eq!(portfolio.currency, "USD");
    assert_eq!(portfolio.fee_type.as_deref(), Some("Fixed"));
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].symbol, "VWCE");
    assert_eq!(assets[0].name, "Vanguard FTSE All-World");
    assert_eq!(assets[0].currency, "EUR");
    assert_eq!(assets[0].provider, Provider::YF);
    assert_eq!(assets[0].quantity, dec!(2));
    assert_eq!(assets[0].manual_price, Some(dec!(120)));
    assert_eq!(assets[0].assets_data_id.get_version_num(), 7);

    let asset_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset WHERE portfolio_id = $1")
            .bind(PORTFOLIO_ID)
            .fetch_one(&pool)
            .await?;
    assert_eq!(asset_count, 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn upsert_normalizes_symbols_before_matching_and_writing(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a stored Portfolio Asset, WHEN clients submit lower and mixed-case symbols,
    // THEN synchronization retains one row and stores the canonical upper-case symbol.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let (_, inserted_assets) = repository
        .upsert(USER_ID, portfolio_request(vec![asset("vwce")]))
        .await?;
    let inserted_id = inserted_assets[0].id;

    let (_, updated_assets) = repository
        .upsert(USER_ID, portfolio_request(vec![asset("VwCe")]))
        .await?;

    assert_eq!(updated_assets.len(), 1);
    assert_eq!(updated_assets[0].id, inserted_id);
    assert_eq!(
        updated_assets[0].assets_data_id,
        inserted_assets[0].assets_data_id
    );
    assert_eq!(updated_assets[0].symbol, "VWCE");

    let asset_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset WHERE portfolio_id = $1")
            .bind(PORTFOLIO_ID)
            .fetch_one(&pool)
            .await?;
    assert_eq!(asset_count, 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn ownership_is_required_for_upsert_and_delete(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let result = repository
        .upsert(OTHER_USER_ID, portfolio_request(vec![asset("VWCE")]))
        .await;

    assert!(matches!(
        result,
        Err(PortfolioRepositoryError::CannotUpdate)
    ));
    repository.soft_delete(OTHER_USER_ID, PORTFOLIO_ID).await?;

    let deleted: bool = sqlx::query_scalar("SELECT deleted FROM portfolios WHERE id = $1")
        .bind(PORTFOLIO_ID)
        .fetch_one(&pool)
        .await?;
    assert!(!deleted);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn find_owners_includes_soft_deleted_rows_and_omits_unknown_ids(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an existing soft-deleted Portfolio plus an unknown identifier,
    // WHEN ownership is preflighted,
    // THEN the existing owner is returned and the unknown ID stays absent for
    // creation or tombstones.
    sqlx::query("UPDATE portfolios SET deleted = TRUE WHERE id = $1")
        .bind(PORTFOLIO_ID)
        .execute(&pool)
        .await?;

    let repository = SqlxPortfolioRepository::new(pool);
    let owners = repository
        .find_owners(&[
            PORTFOLIO_ID,
            Uuid::from_u128(0x10000000000000000000000000000099),
        ])
        .await?;

    assert_eq!(owners.get(&PORTFOLIO_ID), Some(&USER_ID));
    assert_eq!(owners.len(), 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn soft_delete_updates_an_owned_portfolio(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = SqlxPortfolioRepository::new(pool.clone());
    repository.soft_delete(USER_ID, PORTFOLIO_ID).await?;

    let deleted: bool = sqlx::query_scalar("SELECT deleted FROM portfolios WHERE id = $1")
        .bind(PORTFOLIO_ID)
        .fetch_one(&pool)
        .await?;
    assert!(deleted);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn reuses_shared_asset_metadata_but_creates_a_new_relationship(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an asset already linked to one Portfolio, WHEN another Portfolio
    // submits the same provider and symbol, THEN storage reuses assets_data
    // while preserving independent relationship rows.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let (_, first_assets) = repository
        .upsert(USER_ID, portfolio_request(vec![asset("VWCE")]))
        .await?;
    let first_relationship_id = first_assets[0].id;
    let shared_id = first_assets[0].assets_data_id;

    let mut second = portfolio_request(vec![asset("vwce")]);
    second.id = SECOND_PORTFOLIO_ID;
    let (_, second_assets) = repository.upsert(USER_ID, second).await?;
    let second_relationship_id = second_assets[0].id;

    assert_eq!(second_assets[0].assets_data_id, shared_id);
    assert_ne!(second_relationship_id, first_relationship_id);
    assert_eq!(second_relationship_id.get_version_num(), 7);
    assert_eq!(shared_id.get_version_num(), 7);

    let shared_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets_data WHERE provider = $1 AND symbol = 'VWCE'",
    )
    .bind(i16::from(Provider::YF))
    .fetch_one(&pool)
    .await?;
    let relationship_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = $1 AND ad.symbol = 'VWCE'",
    )
    .bind(i16::from(Provider::YF))
    .fetch_one(&pool)
    .await?;
    let distinct_shared_ids: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT assets_data_id)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = $1 AND ad.symbol = 'VWCE'",
    )
    .bind(i16::from(Provider::YF))
    .fetch_one(&pool)
    .await?;

    assert_eq!(shared_count, 1);
    assert_eq!(relationship_count, 2);
    assert_eq!(distinct_shared_ids, 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v1_upsert_creates_client_supplied_shared_assets(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a v1 Portfolio carrying client-chosen metadata for a new asset,
    // WHEN the v1 upsert path stores it,
    // THEN the shared row exists with the client-supplied provenance, so v2
    // later verifies it against the provider before trusting it.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    repository
        .upsert(USER_ID, portfolio_request(vec![asset("V1ONLY")]))
        .await?;

    let source: i16 =
        sqlx::query_scalar("SELECT metadata_source FROM assets_data WHERE symbol = 'V1ONLY'")
            .fetch_one(&pool)
            .await?;

    assert_eq!(source, i16::from(MetadataSource::ClientSupplied));
    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn new_shared_and_relationship_rows_use_application_generated_uuidv7_ids(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN database UUID defaults are unavailable, WHEN a new asset is linked,
    // THEN application-generated UUIDv7 identifiers are sufficient for both rows.
    sqlx::query("ALTER TABLE assets_data ALTER COLUMN id DROP DEFAULT")
        .execute(&pool)
        .await?;
    sqlx::query("ALTER TABLE portfolio_asset ALTER COLUMN id DROP DEFAULT")
        .execute(&pool)
        .await?;

    let repository = SqlxPortfolioRepository::new(pool.clone());
    let (_, assets) = repository
        .upsert(USER_ID, portfolio_request(vec![asset("NEWASSET")]))
        .await?;
    let returned = &assets[0];

    let stored_ids: (Uuid, Uuid) = sqlx::query_as(
        "SELECT ad.id, pa.id
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE pa.portfolio_id = $1",
    )
    .bind(PORTFOLIO_ID)
    .fetch_one(&pool)
    .await?;

    assert_eq!((returned.assets_data_id, returned.id), stored_ids);
    assert_eq!(returned.assets_data_id.get_version_num(), 7);
    assert_eq!(returned.id.get_version_num(), 7);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn metadata_is_immutable_and_class_override_can_be_reset(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a shared asset, WHEN a later Portfolio sync submits different
    // metadata and a class, THEN canonical metadata remains unchanged and the
    // relationship records the class override until it is reset.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let mut changed = asset("VWCE");
    changed.name = "Changed name".to_string();
    changed.base_ccy = "USD".to_string();
    changed.aclass = "BOND".to_string();
    let (_, returned_assets) = repository
        .upsert(USER_ID, portfolio_request(vec![changed]))
        .await?;
    assert_eq!(returned_assets[0].name, "Vanguard FTSE All-World");
    assert_eq!(returned_assets[0].currency, "EUR");
    assert_eq!(
        returned_assets[0].asset_class,
        dcapal_backend::ports::outbound::repository::postgres::types::AssetClass::Equity
    );
    assert_eq!(
        returned_assets[0].effective_asset_class(),
        dcapal_backend::ports::outbound::repository::postgres::types::AssetClass::Bond
    );

    let first: (String, String, Option<i16>) = sqlx::query_as(
        "SELECT ad.name, ad.currency, pa.asset_class_override
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE pa.portfolio_id = $1",
    )
    .bind(PORTFOLIO_ID)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        first,
        ("Vanguard FTSE All-World".into(), "EUR".into(), Some(2))
    );

    let mut reset = asset("VWCE");
    reset.aclass = "EQUITY".to_string();
    repository
        .upsert(USER_ID, portfolio_request(vec![reset]))
        .await?;

    let override_after_reset: Option<i16> = sqlx::query_scalar(
        "SELECT asset_class_override
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE pa.portfolio_id = $1 AND ad.symbol = 'VWCE'",
    )
    .bind(PORTFOLIO_ID)
    .fetch_one(&pool)
    .await?;
    assert_eq!(override_after_reset, None);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn concurrent_syncs_reuse_shared_data_without_duplicate_relationships(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN two Portfolios are synchronized concurrently with the same asset,
    // WHEN serializable transactions race on shared metadata, THEN retries leave
    // one shared record and one relationship per Portfolio.
    let service = Arc::new(PortfolioService::new(
        Arc::new(SqlxPortfolioRepository::new(pool.clone())),
        asset_service(&pool),
    ));

    let mut first = portfolio_request(vec![asset("CONCURRENT")]);
    first.id = Uuid::from_u128(0x10000000000000000000000000000011);
    let mut second = portfolio_request(vec![asset("concurrent")]);
    second.id = Uuid::from_u128(0x10000000000000000000000000000012);

    let first_service = service.clone();
    let second_service = service.clone();
    let (first_result, second_result) = tokio::join!(
        first_service.sync_portfolios(
            USER_ID,
            SyncPortfoliosRequest {
                portfolios: vec![first],
                deleted_portfolios: Vec::new(),
            },
        ),
        second_service.sync_portfolios(
            USER_ID,
            SyncPortfoliosRequest {
                portfolios: vec![second],
                deleted_portfolios: Vec::new(),
            },
        ),
    );
    first_result?;
    second_result?;

    let shared_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets_data WHERE provider = $1 AND symbol = 'CONCURRENT'",
    )
    .bind(i16::from(Provider::YF))
    .fetch_one(&pool)
    .await?;
    let relationship_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = $1 AND ad.symbol = 'CONCURRENT'",
    )
    .bind(i16::from(Provider::YF))
    .fetch_one(&pool)
    .await?;

    assert_eq!(shared_count, 1);
    assert_eq!(relationship_count, 2);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn provider_is_part_of_shared_asset_identity(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN the same symbol submitted through two providers, WHEN both are
    // saved in one Portfolio, THEN each provider gets separate shared data.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let mut yahoo = asset("BTC");
    yahoo.name = "Yahoo Bitcoin".to_string();
    let mut kraken = asset("BTC");
    kraken.provider = "Kraken".to_string();
    kraken.name = "Kraken Bitcoin".to_string();
    repository
        .upsert(USER_ID, portfolio_request(vec![yahoo, kraken]))
        .await?;

    let identities: Vec<(i16, String)> = sqlx::query_as(
        "SELECT ad.provider, ad.symbol
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE pa.portfolio_id = $1
         ORDER BY ad.provider",
    )
    .bind(PORTFOLIO_ID)
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        identities,
        vec![
            (i16::from(Provider::Kraken), "BTC".into()),
            (i16::from(Provider::YF), "BTC".into())
        ]
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_persists_strategic_descendants_and_exact_decimals(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a valid Strategic v2 aggregate, WHEN it is synchronized, THEN the
    // Portfolio mode, class targets, relative targets, and decimal source
    // values survive the round trip through canonical storage.
    let first = v2_asset("VWCE");
    let second = v2_asset("CASH");
    let request = SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios: vec![PortfolioV2 {
            id: PORTFOLIO_ID,
            name: "Strategic portfolio".to_string(),
            quote_currency: "EUR".to_string(),
            allocation_mode: AllocationMode::Strategic,
            drift_band: Some(dec!(1.25)),
            fees: None,
            assets: vec![first, second],
            strategic_allocation: Some(StrategicAllocationV2 {
                asset_class_targets: vec![
                    StrategicAssetClassTargetV2 {
                        asset_class: AssetClass::Equity,
                        target_weight: dec!(60.00),
                    },
                    StrategicAssetClassTargetV2 {
                        asset_class: AssetClass::Cash,
                        target_weight: dec!(40.00),
                    },
                ],
                asset_targets: vec![
                    StrategicAssetTargetV2 {
                        asset: v2_ref("VWCE"),
                        relative_target_weight: dec!(100.00),
                    },
                    StrategicAssetTargetV2 {
                        asset: v2_ref("CASH"),
                        relative_target_weight: dec!(100.00),
                    },
                ],
            }),
            last_updated_at: Utc::now(),
        }],
        deleted_portfolios: Vec::new(),
    };

    let repository = SqlxPortfolioRepository::new(pool.clone());
    let response = sync_v2(&repository, USER_ID, &request).await?;

    let portfolio = &response.updated_portfolios[0];
    assert_eq!(portfolio.allocation_mode, AllocationMode::Strategic);
    assert_eq!(portfolio.drift_band, Some(dec!(1.25)));
    assert_eq!(portfolio.assets[0].quantity, dec!(2.125));
    assert_eq!(portfolio.assets[0].manual_price, Some(dec!(120.10)));
    assert!(portfolio.assets[0].target_weight.is_none());
    let strategic = portfolio
        .strategic_allocation
        .as_ref()
        .expect("strategic allocation");
    assert_eq!(strategic.asset_class_targets.len(), 2);
    assert_eq!(strategic.asset_class_targets[0].target_weight, dec!(60.00));
    assert_eq!(
        strategic.asset_targets[0].relative_target_weight,
        dec!(100.00)
    );
    let serialized = serde_json::to_value(
        dcapal_backend::ports::inbound::rest::types::sync_portfolio_v2::PortfolioV2Response::from(
            portfolio.clone(),
        ),
    )?;
    assert_eq!(serialized["driftBand"], "1.25");
    assert_eq!(serialized["assets"][0]["quantity"], "2.125");
    assert_eq!(serialized["assets"][0]["manualPrice"], "120.10");

    let stored_weights: Vec<Decimal> = sqlx::query_scalar(
        "SELECT weight
         FROM portfolio_asset_allocation
         WHERE portfolio_id = $1
         ORDER BY portfolio_asset_id",
    )
    .bind(PORTFOLIO_ID)
    .fetch_all(&pool)
    .await?;
    assert_eq!(stored_weights, vec![dec!(100.00), dec!(100.00)]);

    let legacy_target_column: Option<String> = sqlx::query_scalar(
        "SELECT column_name
         FROM information_schema.columns
         WHERE table_name = 'portfolio_asset' AND column_name = 'target_weight'",
    )
    .fetch_optional(&pool)
    .await?;
    assert_eq!(legacy_target_column, None);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_reuses_relationships_by_portfolio_and_asset(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a Portfolio already holding VWCE and CASH with stored allocation rows,
    // WHEN a newer aggregate lists the same assets (in the other order) with new
    // quantities and weights,
    // THEN the existing relationship rows are updated in place, keeping their
    // identifiers, and their allocation rows carry the new weights.
    let mut vwce = v2_asset("VWCE");
    vwce.quantity = dec!(7);
    let cash = v2_asset("CASH");
    let request = v2_simple_request(vec![(cash, dec!(25)), (vwce, dec!(75))]);

    let repository = SqlxPortfolioRepository::new(pool.clone());
    let response = sync_v2(&repository, USER_ID, &request).await?;
    assert_eq!(response.updated_portfolios[0].assets.len(), 2);

    let stored: Vec<(Uuid, String, Decimal, Decimal)> = sqlx::query_as(
        "SELECT pa.id, ad.symbol, pa.quantity, paa.weight
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         JOIN portfolio_asset_allocation AS paa
           ON paa.portfolio_id = pa.portfolio_id AND paa.portfolio_asset_id = pa.id
         WHERE pa.portfolio_id = $1
         ORDER BY ad.symbol",
    )
    .bind(PORTFOLIO_ID)
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        stored,
        vec![
            (
                FIXTURE_CASH_RELATIONSHIP,
                "CASH".to_string(),
                dec!(2.125),
                dec!(25)
            ),
            (
                FIXTURE_VWCE_RELATIONSHIP,
                "VWCE".to_string(),
                dec!(7),
                dec!(75)
            ),
        ]
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_deletes_relationships_for_dropped_assets(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a Portfolio holding VWCE and CASH,
    // WHEN a newer aggregate lists only VWCE,
    // THEN the CASH relationship and its allocation row are deleted while the
    // shared CASH asset record stays in the directory.
    let request = v2_simple_request(vec![(v2_asset("VWCE"), dec!(100))]);

    let repository = SqlxPortfolioRepository::new(pool.clone());
    let response = sync_v2(&repository, USER_ID, &request).await?;
    assert_eq!(response.updated_portfolios[0].assets.len(), 1);

    let relationships: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM portfolio_asset WHERE portfolio_id = $1")
            .bind(PORTFOLIO_ID)
            .fetch_all(&pool)
            .await?;
    assert_eq!(relationships, vec![FIXTURE_VWCE_RELATIONSHIP]);
    let allocations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portfolio_asset_allocation WHERE portfolio_asset_id = $1",
    )
    .bind(FIXTURE_CASH_RELATIONSHIP)
    .fetch_one(&pool)
    .await?;
    assert_eq!(allocations, 0);
    let shared: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets_data WHERE symbol = 'CASH'")
        .fetch_one(&pool)
        .await?;
    assert_eq!(shared, 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_switching_an_asset_is_a_delete_and_an_insert(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a Portfolio holding VWCE and CASH,
    // WHEN a newer aggregate replaces CASH with another asset,
    // THEN VWCE keeps its relationship, the CASH relationship is gone, and the
    // replacement gets a new UUIDv7 relationship with its own allocation row.
    onboard(&pool, "SWITCHED").await?;
    let request = v2_simple_request(vec![
        (v2_asset("VWCE"), dec!(60)),
        (v2_asset("SWITCHED"), dec!(40)),
    ]);

    let repository = SqlxPortfolioRepository::new(pool.clone());
    sync_v2(&repository, USER_ID, &request).await?;

    let stored: Vec<(Uuid, String, Decimal)> = sqlx::query_as(
        "SELECT pa.id, ad.symbol, paa.weight
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         JOIN portfolio_asset_allocation AS paa
           ON paa.portfolio_id = pa.portfolio_id AND paa.portfolio_asset_id = pa.id
         WHERE pa.portfolio_id = $1
         ORDER BY ad.symbol",
    )
    .bind(PORTFOLIO_ID)
    .fetch_all(&pool)
    .await?;
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].1, "SWITCHED");
    assert_eq!(stored[0].2, dec!(40));
    assert_eq!(stored[0].0.get_version_num(), 7);
    assert_ne!(stored[0].0, FIXTURE_CASH_RELATIONSHIP);
    assert_eq!(
        stored[1],
        (FIXTURE_VWCE_RELATIONSHIP, "VWCE".to_string(), dec!(60))
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_stores_class_overrides_verbatim_and_hides_relationship_ids(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN VWCE (stored as equity) submitted with an equity override,
    // WHEN the aggregate is synchronized and serialized,
    // THEN the override is returned as submitted (no normalisation against the
    // shared class) and the wire asset exposes only provider and symbol.
    let mut vwce = v2_asset("VWCE");
    vwce.asset_class_override = Some(AssetClass::Equity);
    let request = v2_simple_request(vec![(vwce, dec!(100))]);

    let repository = SqlxPortfolioRepository::new(pool);
    let response = sync_v2(&repository, USER_ID, &request).await?;

    let portfolio = response.updated_portfolios[0].clone();
    assert_eq!(
        portfolio.assets[0].asset_class_override,
        Some(AssetClass::Equity)
    );
    let wire = serde_json::to_value(
        dcapal_backend::ports::inbound::rest::types::sync_portfolio_v2::PortfolioV2Response::from(
            portfolio,
        ),
    )?;
    assert_eq!(
        wire["assets"][0]["asset"],
        serde_json::json!({"provider": "yf", "symbol": "VWCE"})
    );
    assert!(wire["assets"][0].get("id").is_none());

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_service_onboards_unknown_assets_before_persisting(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an asset the directory lacks but its provider knows,
    // WHEN the command resolves it and the service synchronizes the aggregate,
    // THEN the shared record is created and the Portfolio references it.
    let repository = Arc::new(SqlxPortfolioRepository::new(pool.clone()));
    let assets = asset_service(&pool);
    let service = PortfolioService::new(repository.clone(), assets.clone());
    let request = v2_simple_request(vec![(v2_asset("newco"), dec!(100))]);

    let command =
        SyncPortfoliosV2Command::try_new(USER_ID, request, repository.as_ref(), &assets).await?;
    let response = service.sync_portfolios_v2(command).await?;

    assert_eq!(
        response.updated_portfolios[0].assets[0].asset,
        v2_ref("NEWCO")
    );
    let stored: (String, String) = sqlx::query_as(
        "SELECT ad.name, ad.currency
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE pa.portfolio_id = $1 AND ad.symbol = 'NEWCO'",
    )
    .bind(PORTFOLIO_ID)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored, ("New company".to_string(), "USD".to_string()));
    let source: i16 =
        sqlx::query_scalar("SELECT metadata_source FROM assets_data WHERE symbol = 'NEWCO'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(source, i16::from(MetadataSource::ProviderConfirmed));

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_rejects_a_portfolio_owned_by_another_user(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an existing Portfolio owned by another user, WHEN v2 sync submits
    // a newer aggregate for its ID, THEN the repository rejects the write and
    // leaves the canonical row unchanged.
    let request = SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios: vec![PortfolioV2 {
            id: PORTFOLIO_ID,
            name: "Should not update".to_string(),
            quote_currency: "USD".to_string(),
            allocation_mode: AllocationMode::Simple,
            drift_band: None,
            fees: None,
            assets: Vec::new(),
            strategic_allocation: None,
            last_updated_at: Utc::now(),
        }],
        deleted_portfolios: Vec::new(),
    };

    let repository = SqlxPortfolioRepository::new(pool.clone());
    assert!(matches!(
        sync_v2(&repository, OTHER_USER_ID, &request).await,
        Err(PortfolioRepositoryError::OwnershipConflict(OwnershipConflict::Portfolio { id }))
            if id == PORTFOLIO_ID
    ));

    let name: String = sqlx::query_scalar("SELECT name FROM portfolios WHERE id = $1")
        .bind(PORTFOLIO_ID)
        .fetch_one(&pool)
        .await?;
    assert_eq!(name, "Existing portfolio");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_keeps_unknown_deletions_as_terminal_tombstones(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a deletion for an ID that is not stored yet, WHEN v2 synchronization
    // acknowledges it, THEN a user-owned tombstone prevents a later stale
    // aggregate from recreating that Portfolio.
    let tombstone_id = Uuid::from_u128(0x10000000000000000000000000000031);
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let deletion = SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios: Vec::new(),
        deleted_portfolios: vec![tombstone_id],
    };

    let response = sync_v2(&repository, USER_ID, &deletion).await?;
    assert_eq!(response.deleted_portfolios, vec![tombstone_id]);

    let tombstone: (Uuid, bool) =
        sqlx::query_as("SELECT user_id, deleted FROM portfolios WHERE id = $1")
            .bind(tombstone_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(tombstone, (USER_ID, true));

    let stale = SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios: vec![PortfolioV2 {
            id: tombstone_id,
            name: "Stale copy".to_string(),
            quote_currency: "EUR".to_string(),
            allocation_mode: AllocationMode::Simple,
            drift_band: None,
            fees: None,
            assets: Vec::new(),
            strategic_allocation: None,
            last_updated_at: Utc::now(),
        }],
        deleted_portfolios: Vec::new(),
    };
    let stale_response = sync_v2(&repository, USER_ID, &stale).await?;
    assert_eq!(stale_response.deleted_portfolios, vec![tombstone_id]);

    let name: String = sqlx::query_scalar("SELECT name FROM portfolios WHERE id = $1")
        .bind(tombstone_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(name, "");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn v2_sync_rolls_back_parent_when_an_asset_was_not_onboarded(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a new aggregate referencing an asset missing from the directory,
    // WHEN the repository cannot find its shared record after writing the parent,
    // THEN the sync fails and the parent row is rolled back.
    let portfolio_id = Uuid::from_u128(0x10000000000000000000000000000021);
    let mut request = v2_simple_request(vec![(v2_asset("GHOST"), dec!(100))]);
    request.portfolios[0].id = portfolio_id;

    let repository = SqlxPortfolioRepository::new(pool.clone());
    assert!(matches!(
        sync_v2(&repository, USER_ID, &request).await,
        Err(PortfolioRepositoryError::AssetNotOnboarded(_))
    ));

    let portfolio_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portfolios WHERE id = $1")
        .bind(portfolio_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(portfolio_count, 0);

    Ok(())
}

/// A Simple aggregate with the given identifier and assets (symbol, target).
fn simple_aggregate(id: Uuid, name: &str, assets: &[(&str, Decimal)]) -> PortfolioV2 {
    PortfolioV2 {
        id,
        name: name.to_string(),
        quote_currency: "EUR".to_string(),
        allocation_mode: AllocationMode::Simple,
        drift_band: None,
        fees: None,
        assets: assets
            .iter()
            .map(|(symbol, weight)| PortfolioAssetV2 {
                target_weight: Some(*weight),
                ..v2_asset(symbol)
            })
            .collect(),
        strategic_allocation: None,
        last_updated_at: Utc::now(),
    }
}

fn request_of(portfolios: Vec<PortfolioV2>, deleted: Vec<Uuid>) -> SyncPortfoliosV2Request {
    SyncPortfoliosV2Request {
        schema_version: 2,
        portfolios,
        deleted_portfolios: deleted,
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn applying_a_plan_over_a_changed_row_is_stale_and_writes_nothing(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a plan that updates the stored Portfolio, creates another one and
    // tombstones an unknown id,
    // WHEN the stored Portfolio is modified (by another writer) before the plan
    // is applied,
    // THEN applying fails with a retryable StaleSnapshot and the transaction
    // rolls back: no Portfolio was created, tombstoned or updated.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let new_id = Uuid::from_u128(0x10000000000000000000000000000041);
    let tombstone_id = Uuid::from_u128(0x10000000000000000000000000000042);
    let request = request_of(
        vec![
            simple_aggregate(PORTFOLIO_ID, "Planned update", &[("VWCE", dec!(100))]),
            simple_aggregate(new_id, "Planned creation", &[("VWCE", dec!(100))]),
        ],
        vec![tombstone_id],
    );
    let snapshot = repository.load_sync_snapshot(USER_ID).await?;
    let plan = plan_sync(&snapshot, &request);
    sqlx::query("UPDATE portfolios SET name = 'Concurrent edit', updated_at = NOW() WHERE id = $1")
        .bind(PORTFOLIO_ID)
        .execute(&pool)
        .await?;

    let error = repository
        .apply_sync_plan(USER_ID, &plan)
        .await
        .unwrap_err();

    assert!(matches!(error, PortfolioRepositoryError::StaleSnapshot));
    assert!(error.is_retryable_concurrency());
    let names: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM portfolios ORDER BY id")
        .fetch_all(&pool)
        .await?;
    assert_eq!(names, vec![(PORTFOLIO_ID, "Concurrent edit".to_string())]);
    let relationships: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_asset WHERE portfolio_id = $1")
            .bind(PORTFOLIO_ID)
            .fetch_one(&pool)
            .await?;
    assert_eq!(relationships, 2);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn applying_a_plan_detects_rows_that_appeared_or_belong_to_others(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a plan creating a Portfolio that did not exist when it was planned,
    // WHEN a row with that identifier appears for the same user, THEN applying
    // is a retryable StaleSnapshot; WHEN it instead belongs to another user,
    // THEN applying is an ownership conflict.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let id = Uuid::from_u128(0x10000000000000000000000000000051);
    let request = request_of(vec![simple_aggregate(id, "Created", &[])], vec![]);
    let plan = plan_sync(&repository.load_sync_snapshot(USER_ID).await?, &request);

    sqlx::query(
        "INSERT INTO portfolios (id, user_id, name, currency, deleted, last_updated_at)
         VALUES ($1, $2, 'Appeared', 'EUR', FALSE, NOW())",
    )
    .bind(id)
    .bind(USER_ID)
    .execute(&pool)
    .await?;
    assert!(matches!(
        repository.apply_sync_plan(USER_ID, &plan).await,
        Err(PortfolioRepositoryError::StaleSnapshot)
    ));

    sqlx::query("UPDATE portfolios SET user_id = $2 WHERE id = $1")
        .bind(id)
        .bind(OTHER_USER_ID)
        .execute(&pool)
        .await?;
    assert!(matches!(
        repository.apply_sync_plan(USER_ID, &plan).await,
        Err(PortfolioRepositoryError::OwnershipConflict(OwnershipConflict::Portfolio { id: conflict }))
            if conflict == id
    ));

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn concurrent_insert_of_the_same_portfolio_is_a_retryable_stale_snapshot(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a plan creating a Portfolio and an uncommitted transaction of another
    // connection that inserts the same identifier (invisible to the plan's
    // verification, so the insert waits on the unique index),
    // WHEN the other transaction commits, THEN the unique violation surfaces as
    // a retryable StaleSnapshot, and re-planning then sees the row.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let id = Uuid::from_u128(0x10000000000000000000000000000061);
    let request = request_of(vec![simple_aggregate(id, "Created", &[])], vec![]);
    let plan = plan_sync(&repository.load_sync_snapshot(USER_ID).await?, &request);

    let mut competing = pool.begin().await?;
    sqlx::query(
        "INSERT INTO portfolios (id, user_id, name, currency, deleted, last_updated_at)
         VALUES ($1, $2, 'Competing', 'EUR', FALSE, '2020-01-01T00:00:00Z')",
    )
    .bind(id)
    .bind(USER_ID)
    .execute(&mut *competing)
    .await?;

    let applying = {
        let repository = repository.clone();
        tokio::spawn(async move { repository.apply_sync_plan(USER_ID, &plan).await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    competing.commit().await?;

    let error = applying.await?.unwrap_err();
    assert!(matches!(error, PortfolioRepositoryError::StaleSnapshot));
    assert!(error.is_retryable_concurrency());

    let replanned = plan_sync(&repository.load_sync_snapshot(USER_ID).await?, &request);
    assert_eq!(replanned.writes[0].kind, WriteKind::Update);
    repository.apply_sync_plan(USER_ID, &replanned).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM portfolios WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(name, "Created");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("users", "portfolio"))]
async fn one_plan_writes_several_portfolios_sharing_assets_and_matches_a_reload(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN one plan that empties the stored Portfolio, creates a Simple
    // Portfolio (with a hidden class target and a sub-microsecond timestamp)
    // and a Strategic one, both holding the same shared assets,
    // WHEN it is applied once,
    // THEN every Portfolio, relationship, allocation and class target is
    // stored, shared assets are not duplicated, and a fresh snapshot projects
    // exactly to the response the plan promised.
    let repository = SqlxPortfolioRepository::new(pool.clone());
    let simple_id = Uuid::from_u128(0x10000000000000000000000000000071);
    let strategic_id = Uuid::from_u128(0x10000000000000000000000000000072);
    let emptied = simple_aggregate(PORTFOLIO_ID, "Emptied", &[]);
    let mut simple_portfolio = simple_aggregate(
        simple_id,
        "Simple",
        &[("VWCE", dec!(60.50)), ("CASH", dec!(39.50))],
    );
    simple_portfolio.last_updated_at += chrono::Duration::nanoseconds(789);
    simple_portfolio.strategic_allocation = Some(StrategicAllocationV2 {
        asset_class_targets: vec![StrategicAssetClassTargetV2 {
            asset_class: AssetClass::Equity,
            target_weight: dec!(100),
        }],
        asset_targets: Vec::new(),
    });
    let strategic_portfolio = PortfolioV2 {
        id: strategic_id,
        name: "Strategic".to_string(),
        allocation_mode: AllocationMode::Strategic,
        assets: vec![v2_asset("VWCE"), v2_asset("CASH")],
        strategic_allocation: Some(StrategicAllocationV2 {
            asset_class_targets: vec![
                StrategicAssetClassTargetV2 {
                    asset_class: AssetClass::Cash,
                    target_weight: dec!(30.0),
                },
                StrategicAssetClassTargetV2 {
                    asset_class: AssetClass::Equity,
                    target_weight: dec!(70.0),
                },
            ],
            asset_targets: vec![StrategicAssetTargetV2 {
                asset: v2_ref("VWCE"),
                relative_target_weight: dec!(100.00),
            }],
        }),
        ..simple_aggregate(strategic_id, "Strategic", &[])
    };
    let request = request_of(vec![emptied, simple_portfolio, strategic_portfolio], vec![]);
    let plan = plan_sync(&repository.load_sync_snapshot(USER_ID).await?, &request);

    repository.apply_sync_plan(USER_ID, &plan).await?;

    let relationships: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT pa.portfolio_id, ad.symbol
         FROM portfolio_asset AS pa JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         ORDER BY pa.portfolio_id, ad.symbol",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        relationships,
        vec![
            (simple_id, "CASH".to_string()),
            (simple_id, "VWCE".to_string()),
            (strategic_id, "CASH".to_string()),
            (strategic_id, "VWCE".to_string()),
        ]
    );
    let shared: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets_data")
        .fetch_one(&pool)
        .await?;
    assert_eq!(shared, 2);
    let allocations: Vec<(Uuid, String, Decimal)> = sqlx::query_as(
        "SELECT paa.portfolio_id, ad.symbol, paa.weight
         FROM portfolio_asset_allocation AS paa
         JOIN portfolio_asset AS pa ON pa.id = paa.portfolio_asset_id
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         ORDER BY paa.portfolio_id, ad.symbol",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        allocations,
        vec![
            (simple_id, "CASH".to_string(), dec!(39.50)),
            (simple_id, "VWCE".to_string(), dec!(60.50)),
            (strategic_id, "CASH".to_string(), dec!(0)),
            (strategic_id, "VWCE".to_string(), dec!(100.00)),
        ]
    );
    let class_targets: Vec<(Uuid, i16)> = sqlx::query_as(
        "SELECT portfolio_id, asset_class FROM strategic_asset_class_targets
         ORDER BY portfolio_id, asset_class",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(class_targets.len(), 3);

    // The plan's promised response equals what a later read projects.
    let reloaded = plan_sync(
        &repository.load_sync_snapshot(USER_ID).await?,
        &request_of(vec![], vec![]),
    );
    let by_id = |portfolios: &[PortfolioV2]| {
        let mut sorted = portfolios.to_vec();
        sorted.sort_by_key(|portfolio| portfolio.id);
        sorted
    };
    assert_eq!(
        by_id(&reloaded.response.updated_portfolios),
        by_id(&plan.response.updated_portfolios)
    );
    assert!(reloaded.response.deleted_portfolios.is_empty());

    Ok(())
}
