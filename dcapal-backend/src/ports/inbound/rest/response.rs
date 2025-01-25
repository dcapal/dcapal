use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::app::domain::db::fee::FeeStructure;
use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::DateTime;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncPortfoliosResponse {
    pub updated_portfolios: Vec<PortfolioResponse>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioResponse {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesResponse>,
    pub assets: Vec<PortfolioAssetResponse>,
    pub last_updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioAssetResponse {
    pub symbol: String,
    pub name: String,
    pub aclass: String,
    pub base_ccy: String,
    pub provider: String,
    pub qty: BigDecimal,
    pub target_weight: BigDecimal,
    pub price: BigDecimal,
    pub fees: Option<TransactionFeesResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionFeesResponse {
    pub max_fee_impact: BigDecimal,
    pub fee_type: FeeStructure,
}

impl From<(Vec<portfolio::Model>, Vec<portfolio_asset::Model>)> for SyncPortfoliosResponse {
    fn from(input: (Vec<portfolio::Model>, Vec<portfolio_asset::Model>)) -> Self {
        let (portfolios_data, assets_data) = input;
        let mut portfolios = Vec::new();

        for portfolio in portfolios_data {
            let portfolio_assets: Vec<PortfolioAssetResponse> = assets_data
                .iter()
                .filter(|asset| asset.portfolio_id == portfolio.id)
                .map(|asset| {
                    let fees = asset.fees.as_ref().map(|fees| TransactionFeesResponse {
                        max_fee_impact: fees.max_fee_impact.clone(),
                        fee_type: fees.fee_type.clone(),
                    });

                    PortfolioAssetResponse {
                        symbol: asset.symbol.clone(),
                        name: asset.name.clone(),
                        aclass: asset.aclass.clone(),
                        base_ccy: asset.base_ccy.clone(),
                        provider: asset.provider.clone(),
                        qty: asset.qty.clone(),
                        target_weight: asset.target_weight.clone(),
                        price: asset.price.clone(),
                        fees,
                    }
                })
                .collect();

            portfolios.push(PortfolioResponse {
                id: portfolio.id,
                name: portfolio.name.clone(),
                quote_ccy: portfolio.quote_ccy.clone(),
                fees: portfolio.fees.as_ref().map(|fees| TransactionFeesResponse {
                    max_fee_impact: fees.max_fee_impact.clone(),
                    fee_type: fees.fee_type.clone(),
                }),
                assets: portfolio_assets,
                last_updated_at: portfolio.last_updated_at,
            });
        }

        Self { updated_portfolios: portfolios }
    }
}