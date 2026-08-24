use chrono::Utc;
use dcapal_backend::ports::{
    inbound::rest::{
        FeeStructure,
        request::{PortfolioAssetRequest, PortfolioRequest},
    },
    outbound::repository::{
        portfolio::{PortfolioRepository, PortfolioRepositoryError},
        postgres::SqlxPortfolioRepository,
    },
};
use rust_decimal::dec;
use sqlx::PgPool;
use uuid::Uuid;

const USER_ID: Uuid = Uuid::from_u128(1);
const OTHER_USER_ID: Uuid = Uuid::from_u128(2);
const PORTFOLIO_ID: Uuid = Uuid::from_u128(0x10000000000000000000000000000001);

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
    sqlx::query("UPDATE portfolio_asset SET provider = 99 WHERE symbol = 'VWCE'")
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
    sqlx::query("UPDATE portfolio_asset SET asset_class = 99 WHERE symbol = 'VWCE'")
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
    assert_eq!(assets[0].quantity, dec!(2));

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
