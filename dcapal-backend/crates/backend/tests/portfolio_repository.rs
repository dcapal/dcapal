use std::sync::Arc;

use chrono::Utc;
use dcapal_backend::app::services::portfolio::PortfolioService;
use dcapal_backend::ports::{
    inbound::rest::{
        FeeStructure,
        request::{PortfolioAssetRequest, PortfolioRequest, SyncPortfoliosRequest},
    },
    outbound::repository::{
        portfolio::{PortfolioRepository, PortfolioRepositoryError},
        postgres::{SqlxPortfolioRepository, types::Provider},
    },
};
use rust_decimal::dec;
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
        "SELECT COUNT(*) FROM assets_data WHERE provider = 2 AND symbol = 'VWCE'",
    )
    .fetch_one(&pool)
    .await?;
    let relationship_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 2 AND ad.symbol = 'VWCE'",
    )
    .fetch_one(&pool)
    .await?;
    let distinct_shared_ids: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT assets_data_id)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 2 AND ad.symbol = 'VWCE'",
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(shared_count, 1);
    assert_eq!(relationship_count, 2);
    assert_eq!(distinct_shared_ids, 1);

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
    let service = Arc::new(PortfolioService::new(Arc::new(
        SqlxPortfolioRepository::new(pool.clone()),
    )));

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
        "SELECT COUNT(*) FROM assets_data WHERE provider = 2 AND symbol = 'CONCURRENT'",
    )
    .fetch_one(&pool)
    .await?;
    let relationship_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM portfolio_asset AS pa
         JOIN assets_data AS ad ON ad.id = pa.assets_data_id
         WHERE ad.provider = 2 AND ad.symbol = 'CONCURRENT'",
    )
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
    assert_eq!(identities, vec![(1, "BTC".into()), (2, "BTC".into())]);

    Ok(())
}
