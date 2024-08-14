use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct PortfolioHoldingsResponse {
    pub holdings: Vec<PortfolioResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct PortfolioResponse {
    pub name: String,
    pub ticker: String,
    pub price: f64,
    pub quantity: f64,
    pub weight: f64,
    pub total: f64,
}

pub async fn get_portfolio_holdings() -> crate::error::Result<Response> {
    Ok(Json(PortfolioHoldingsResponse {
        holdings: vec![
            PortfolioResponse {
                name: "Bitcoin".to_string(),
                ticker: "BTC".to_string(),
                price: 50000.0,
                quantity: 1.0,
                weight: 30.0,
                total: 50000.0,
            },
            PortfolioResponse {
                name: "Ethereum".to_string(),
                ticker: "ETH".to_string(),
                price: 3000.0,
                quantity: 2.0,
                weight: 30.0,
                total: 6000.0,
            },
            PortfolioResponse {
                name: "Cardano".to_string(),
                ticker: "ADA".to_string(),
                price: 2.0,
                quantity: 1000.0,
                weight: 20.0,
                total: 2000.0,
            },
            PortfolioResponse {
                name: "All-World ETF".to_string(),
                ticker: "ALLWD".to_string(),
                price: 2.0,
                quantity: 1000.0,
                weight: 10.0,
                total: 2000.0,
            },
        ],
    })
    .into_response())
}
