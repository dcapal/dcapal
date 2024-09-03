use crate::app::domain::entity::{Portfolio, PortfolioHoldings};
use crate::app::infra::claim::Claims;
use crate::ports::inbound::rest::user::MessageResponse;
use crate::AppContext;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

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

#[derive(Debug, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioRequest {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub assets: Vec<PortfolioHoldingsRequest>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHoldingsRequest {
    pub symbol: String,
    pub name: String,
    pub quantity: BigDecimal,
    pub average_buy_price: BigDecimal,
    pub weight: BigDecimal,
    pub price: BigDecimal,
}

impl Into<Portfolio> for PortfolioRequest {
    fn into(self) -> Portfolio {
        Portfolio {
            id: self.id,
            name: self.name,
            description: self.description,
            currency: self.currency,
            assets: self.assets.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl Into<PortfolioHoldings> for PortfolioHoldingsRequest {
    fn into(self) -> PortfolioHoldings {
        PortfolioHoldings {
            symbol: self.symbol,
            name: self.name,
            quantity: self.quantity,
            average_buy_price: self.average_buy_price,
            weight: self.weight,
            total: BigDecimal::from(0), //TODO: set the proper total
            price: self.price,
        }
    }
}

impl From<&Portfolio> for PortfolioRequest {
    fn from(portfolio: &Portfolio) -> Self {
        PortfolioRequest {
            id: portfolio.id,
            name: portfolio.name.clone(),
            description: portfolio.description.clone(),
            currency: portfolio.currency.clone(),
            assets: portfolio
                .assets
                .iter()
                .map(PortfolioHoldingsRequest::from)
                .collect(),
        }
    }
}

impl From<&PortfolioHoldings> for PortfolioHoldingsRequest {
    fn from(holding: &PortfolioHoldings) -> Self {
        PortfolioHoldingsRequest {
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            quantity: holding.quantity.clone(),
            average_buy_price: holding.average_buy_price.clone(),
            weight: holding.weight.clone(),
            price: holding.price.clone(),
        }
    }
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

#[utoipa::path(
    post,
    path = "/api/v1/user/portfolios",
    request_body = PortfolioRequest,
    responses(
        (status = 200, description = "Success update profile information", body = [MessageResponse]),
        (status = 400, description = "Invalid data input", body = [AppResponseError]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn save_portfolio(
    State(ctx): State<AppContext>,
    claims: Claims,
    Json(req): Json<PortfolioRequest>,
) -> crate::error::Result<Response> {
    info!("Update profile user_id: {}.", claims.sub);
    match &ctx
        .services
        .portfolio
        .save_portfolio(claims.sub, req.into())
        .await
    {
        Ok(_) => {
            info!("Success update profile user user_id: {}.", claims.sub);
            Ok(Json(MessageResponse::new("User profile updated.")).into_response())
        }
        Err(e) => {
            info!("Unsuccessful update profile user: {e:?}");
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/user/portfolios",
    responses(
        (status = 200, description = "Success get user portfolios", body = [PortfolioHoldingsResponse]),
        (status = 400, description = "Invalid data input", body = [AppResponseError]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn get_portfolios(
    State(ctx): State<AppContext>,
    claims: Claims,
) -> crate::error::Result<Response> {
    info!("Get user portfolios user_id: {}.", claims.sub);
    match &ctx.services.portfolio.get_portfolios(claims.sub).await {
        Ok(portfolios) => {
            info!(
                "Successfully got user portfolios for user_id: {}.",
                claims.sub
            );
            let portfolio_requests: Vec<PortfolioRequest> =
                portfolios.iter().map(PortfolioRequest::from).collect();
            Ok(Json(portfolio_requests).into_response())
        }
        Err(e) => {
            info!("Unsuccessful get user portfolios user: {e:?}");
            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }
}
