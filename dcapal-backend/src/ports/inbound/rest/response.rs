use crate::app::domain::db::fee_type::FeeType;
use crate::app::domain::db::{portfolio, portfolio_asset};
use crate::ports::inbound::rest::FeeStructure;
use crate::DateTime;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
    pub fees: TransactionFeesResponse,
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
    pub fees: TransactionFeesResponse,
}

impl From<(portfolio::Model, Vec<portfolio_asset::Model>)> for PortfolioResponse {
    fn from(input: (portfolio::Model, Vec<portfolio_asset::Model>)) -> Self {
        let (portfolio, assets) = input;
        let portfolio_assets: Vec<PortfolioAssetResponse> = assets
            .iter()
            .map(|asset| {
                let fees = TransactionFeesResponse {
                    max_fee_impact: asset.max_fee_impact,
                    fee_structure: match asset.fee_type {
                        Some(FeeType::ZeroFee) => FeeStructure::ZeroFee,
                        Some(FeeType::Fixed) => {
                            if let Some(fee_amount) = asset.fee_amount {
                                FeeStructure::Fixed { fee_amount }
                            } else {
                                return Err("Fixed fee requires fee_amount to be Some.".to_string());
                            }
                        }
                        Some(FeeType::Variable) => {
                            if let (Some(fee_rate), Some(min_fee)) = (asset.fee_rate, asset.min_fee)
                            {
                                FeeStructure::Variable {
                                    fee_rate,
                                    min_fee,
                                    max_fee: asset.max_fee, // `max_fee` is optional, so we can pass it directly
                                }
                            } else {
                                return Err(
                                    "Variable fee requires fee_rate and min_fee to be Some."
                                        .to_string(),
                                );
                            }
                        }
                        _ => return Err("Fee type is not specified.".to_string()),
                    },
                };

                PortfolioAssetResponse {
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    aclass: asset.asset_class.clone(),
                    base_ccy: asset.currency.clone(),
                    provider: asset.provider.clone(),
                    qty: asset.quantity.clone(),
                    target_weight: asset.target_weight.clone(),
                    price: asset.price.clone(),
                    fees,
                }
            })
            .collect();

        Self {
            id: portfolio.id,
            name: portfolio.name.clone(),
            quote_ccy: portfolio.currency.clone(),
            fees: TransactionFeesResponse {
                max_fee_impact: portfolio.max_fee_impact,
                fee_structure: match portfolio.fee_type {
                    Some(FeeType::ZeroFee) => FeeStructure::ZeroFee,
                    Some(FeeType::Fixed) => {
                        if let Some(fee_amount) = portfolio.fee_amount {
                            FeeStructure::Fixed { fee_amount }
                        } else {
                            return Err("Fixed fee requires fee_amount to be Some.".to_string());
                        }
                    }
                    Some(FeeType::Variable) => {
                        if let (Some(fee_rate), Some(min_fee)) = (portfolio.fee_rate, portfolio.min_fee)
                        {
                            FeeStructure::Variable {
                                fee_rate,
                                min_fee,
                                max_fee: portfolio.max_fee, // `max_fee` is optional, so we can pass it directly
                            }
                        } else {
                            return Err(
                                "Variable fee requires fee_rate and min_fee to be Some.".to_string(),
                            );
                        }
                    }
                    _ => return Err("Fee type is not specified.".to_string()),
                },
            },
            assets: portfolio_assets,
            last_updated_at: portfolio.last_updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionFeesResponse {
    pub max_fee_impact: Option<BigDecimal>,
    pub fee_structure: FeeStructure,
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
                    let fees = TransactionFeesResponse {
                        max_fee_impact: asset.clone().max_fee_impact,
                        fee_type: asset.clone().fee_structure,
                    };

                    PortfolioAssetResponse {
                        symbol: asset.symbol.clone(),
                        name: asset.name.clone(),
                        aclass: asset.asset_class.clone(),
                        base_ccy: asset.currency.clone(),
                        provider: asset.provider.clone(),
                        qty: asset.quantity.clone(),
                        target_weight: asset.target_weight.clone(),
                        price: asset.price.clone(),
                        fees,
                    }
                })
                .collect();

            portfolios.push(PortfolioResponse {
                id: portfolio.id,
                name: portfolio.name.clone(),
                quote_ccy: portfolio.currency.clone(),
                fees: TransactionFeesResponse {
                    max_fee_impact: portfolio.max_fee_impact,
                    fee_type: portfolio.fee_structure,
                },
                assets: portfolio_assets,
                last_updated_at: portfolio.last_updated_at,
            });
        }

        Self {
            updated_portfolios: portfolios,
            deleted_portfolios: vec![],
        } //TODO: check if this is correct
    }
}
