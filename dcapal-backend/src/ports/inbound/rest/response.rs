use rust_decimal::Decimal;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    DateTime,
    app::domain::db::{portfolio_asset, portfolios},
    error::DcaError,
    ports::inbound::rest::FeeStructure,
};

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncPortfoliosResponse {
    pub updated_portfolios: Vec<PortfolioResponse>,
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioResponse {
    pub id: Uuid,
    pub name: String,
    pub quote_ccy: String,
    pub fees: Option<TransactionFeesResponse>,
    pub assets: Vec<PortfolioAssetResponse>,
    pub last_updated_at: DateTime,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAssetResponse {
    pub symbol: String,
    pub name: String,
    pub aclass: String,
    pub base_ccy: String,
    pub provider: String,
    #[serde(with = "rust_decimal::serde::float")]
    pub qty: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub target_weight: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
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
                                        max_fee: asset.max_fee,
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
                                ));
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
                                    max_fee: portfolio.max_fee,
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
                            ));
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

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFeesResponse {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "rust_decimal::serde::float_option"
    )]
    pub max_fee_impact: Option<Decimal>,
    pub fee_structure: FeeStructure,
}

#[cfg(test)]
mod test {
    use chrono::Utc;
    use rust_decimal::dec;
    use uuid::Uuid;

    use crate::{
        app::domain::db::{portfolio_asset, portfolios},
        ports::inbound::rest::response::PortfolioAssetResponse,
    };

    use super::PortfolioResponse;

    #[test]
    fn map_model_to_response() {
        let portfolio_id = Uuid::new_v4();

        let portfolio_model = portfolios::Model {
            id: portfolio_id,
            user_id: Uuid::new_v4(),
            name: String::from("my_pf"),
            currency: String::from("EUR"),
            deleted: false,
            last_updated_at: Utc::now().into(),
            max_fee_impact: None,
            fee_type: None,
            fee_amount: None,
            fee_rate: None,
            min_fee: None,
            max_fee: None,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        };

        let asset_model = portfolio_asset::Model {
            id: Uuid::new_v4(),
            symbol: String::from("VWCE"),
            portfolio_id,
            name: String::from("Vanguard FTSE All-World UCITS ETF USD Acc"),
            asset_class: String::from("Stock"),
            currency: String::from("EUR"),
            provider: String::from("IBKR"),
            quantity: dec!(10.0),
            target_weight: dec!(1.0),
            price: dec!(100.0),
            max_fee_impact: None,
            fee_type: None,
            fee_amount: None,
            fee_rate: None,
            min_fee: None,
            max_fee: None,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        };
        let assets_model = vec![asset_model.clone()];

        let expected = PortfolioResponse {
            id: portfolio_model.id,
            name: portfolio_model.name.clone(),
            quote_ccy: portfolio_model.currency.clone(),
            fees: None,
            assets: vec![PortfolioAssetResponse {
                symbol: asset_model.symbol.clone(),
                name: asset_model.name.clone(),
                aclass: asset_model.asset_class.clone(),
                base_ccy: asset_model.currency.clone(),
                provider: asset_model.provider.clone(),
                qty: asset_model.quantity,
                target_weight: asset_model.target_weight,
                price: asset_model.price,
                fees: None,
            }],
            last_updated_at: portfolio_model.last_updated_at.into(),
        };

        let actual: PortfolioResponse = (portfolio_model, assets_model).try_into().unwrap();
        assert_eq!(actual, expected);
    }
}
