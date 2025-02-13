use crate::app::domain::db::{portfolio_asset, portfolios};
use crate::error::DcaError;
use crate::ports::inbound::rest::FeeStructure;
use crate::DateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncPortfoliosResponse {
    pub updated_portfolios: Vec<PortfolioResponse>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioResponse {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesResponse>,
    pub assets: Vec<PortfolioAssetResponse>,
    pub last_updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAssetResponse {
    pub symbol: String,
    pub name: String,
    pub aclass: String,
    pub base_ccy: String,
    pub provider: String,
    pub qty: Decimal,
    pub target_weight: Decimal,
    pub price: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees: Option<TransactionFeesResponse>,
}

impl TryFrom<(portfolios::Model, Vec<portfolio_asset::Model>)> for PortfolioResponse {
    type Error = DcaError;

    fn try_from(
        input: (portfolios::Model, Vec<portfolio_asset::Model>),
    ) -> Result<Self, Self::Error> {
        let (portfolio, assets) = input;
        let portfolio_assets: Vec<PortfolioAssetResponse> = assets
            .iter()
            .map(|asset| {
                let fees = if let Some(fee_type) = asset.fee_type.clone() {
                    Some(TransactionFeesResponse {
                        max_fee_impact: asset.max_fee_impact,
                        fee_structure: match fee_type {
                            val if val == *"ZeroFee" => FeeStructure::ZeroFee,
                            val if val == *"Fixed" => {
                                if let Some(fee_amount) = asset.fee_amount {
                                    FeeStructure::Fixed { fee_amount }
                                } else {
                                    return Err(DcaError::Generic(
                                        "Fixed fee requires fee_amount to be Some.".to_string(),
                                    ));
                                }
                            }
                            val if val == *"Variable" => {
                                if let (Some(fee_rate), Some(min_fee)) =
                                    (asset.fee_rate, asset.min_fee)
                                {
                                    FeeStructure::Variable {
                                        fee_rate,
                                        min_fee,
                                        max_fee: asset.max_fee, // `max_fee` is optional, so we can pass it directly
                                    }
                                } else {
                                    return Err(DcaError::Generic(
                                        "Variable fee requires fee_rate and min_fee to be Some."
                                            .to_string(),
                                    ));
                                }
                            }
                            _ => {
                                return Err(DcaError::Generic(
                                    "Fee type is not specified.".to_string(),
                                ))
                            }
                        },
                    })
                } else {
                    None
                };

                Ok(PortfolioAssetResponse {
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    aclass: asset.asset_class.clone(),
                    base_ccy: asset.currency.clone(),
                    provider: asset.provider.clone(),
                    qty: asset.quantity,
                    target_weight: asset.target_weight,
                    price: asset.price,
                    fees,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            id: portfolio.id,
            name: portfolio.name.clone(),
            quote_ccy: portfolio.currency.clone(),
            fees: if let Some(fee_type) = portfolio.fee_type {
                Some(TransactionFeesResponse {
                    max_fee_impact: portfolio.max_fee_impact,
                    fee_structure: match fee_type {
                        val if val == *"ZeroFee" => FeeStructure::ZeroFee,
                        val if val == *"Fixed" => {
                            if let Some(fee_amount) = portfolio.fee_amount {
                                FeeStructure::Fixed { fee_amount }
                            } else {
                                return Err(DcaError::Generic(
                                    "Fixed fee requires fee_amount to be Some.".to_string(),
                                ));
                            }
                        }
                        val if val == *"Variable" => {
                            if let (Some(fee_rate), Some(min_fee)) =
                                (portfolio.fee_rate, portfolio.min_fee)
                            {
                                FeeStructure::Variable {
                                    fee_rate,
                                    min_fee,
                                    max_fee: portfolio.max_fee, // `max_fee` is optional, so we can pass it directly
                                }
                            } else {
                                return Err(DcaError::Generic(
                                    "Variable fee requires fee_rate and min_fee to be Some."
                                        .to_string(),
                                ));
                            }
                        }
                        _ => {
                            return Err(DcaError::Generic("Fee type is not specified.".to_string()))
                        }
                    },
                })
            } else {
                None
            },
            assets: portfolio_assets,
            last_updated_at: portfolio.last_updated_at.into(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesResponse {
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructure,
}
