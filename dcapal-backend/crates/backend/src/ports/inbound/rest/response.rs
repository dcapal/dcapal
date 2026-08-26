use rust_decimal::Decimal;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    DateTime,
    error::DcaError,
    ports::{
        inbound::rest::FeeStructure,
        outbound::repository::postgres::types::{PortfolioAssetRow, PortfolioRow},
    },
};

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// The result of synchronizing the client's portfolio set.
pub struct SyncPortfoliosResponse {
    /// Portfolios whose server state should be applied by the client.
    pub updated_portfolios: Vec<PortfolioResponse>,
    /// Portfolio identifiers that the client should remove.
    pub deleted_portfolios: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// A portfolio returned by the synchronization endpoint.
pub struct PortfolioResponse {
    /// The portfolio identifier.
    pub id: Uuid,
    /// The portfolio display name.
    pub name: String,
    /// The portfolio quote currency.
    pub quote_ccy: String,
    /// The portfolio-level transaction fee settings.
    pub fees: Option<TransactionFeesResponse>,
    /// The assets held by the portfolio.
    pub assets: Vec<PortfolioAssetResponse>,
    /// The timestamp used to compare this state with a client copy.
    pub last_updated_at: DateTime,
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// An asset held by a portfolio.
///
/// Decimal values are serialized as JSON strings to preserve precision.
pub struct PortfolioAssetResponse {
    /// The provider symbol for the asset.
    pub symbol: String,
    /// The asset display name.
    pub name: String,
    /// The asset class.
    pub aclass: String,
    /// The asset currency.
    pub base_ccy: String,
    /// The data provider for the asset.
    pub provider: String,
    #[serde(with = "rust_decimal::serde::str")]
    /// The quantity held.
    pub qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The target portfolio weight.
    pub target_weight: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The latest known price.
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    /// The average price paid for the holding.
    pub average_buy_price: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The asset-level transaction fee settings.
    pub fees: Option<TransactionFeesResponse>,
}

impl TryFrom<(PortfolioRow, Vec<PortfolioAssetRow>)> for PortfolioResponse {
    type Error = DcaError;

    fn try_from(input: (PortfolioRow, Vec<PortfolioAssetRow>)) -> Result<Self, Self::Error> {
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
                    aclass: asset.effective_asset_class().as_legacy_name().to_string(),
                    base_ccy: asset.currency.clone(),
                    provider: asset.provider.as_legacy_name().to_string(),
                    qty: asset.quantity,
                    target_weight: asset.target_weight,
                    price: asset.manual_price.ok_or_else(|| {
                        DcaError::Generic(
                            "v1 Portfolio Asset response requires a manual price.".to_string(),
                        )
                    })?,
                    average_buy_price: asset.average_buy_price.or(asset.manual_price).ok_or_else(
                        || {
                            DcaError::Generic(
                                "v1 Portfolio Asset response requires a price basis.".to_string(),
                            )
                        },
                    )?,
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
            last_updated_at: portfolio.last_updated_at,
        })
    }
}

#[derive(Debug, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Transaction fee settings returned by the API.
pub struct TransactionFeesResponse {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "rust_decimal::serde::str_option"
    )]
    /// The maximum allowed fee impact, when configured.
    pub max_fee_impact: Option<Decimal>,
    /// The fee calculation model.
    pub fee_structure: FeeStructure,
}

#[cfg(test)]
mod test {

    use chrono::Utc;
    use rust_decimal::dec;

    use super::*;
    use crate::ports::outbound::repository::postgres::types::{AssetClass, Provider};

    #[test]
    fn map_model_to_response() {
        let portfolio_id = Uuid::new_v4();

        let portfolio_model = PortfolioRow {
            id: portfolio_id,
            user_id: Uuid::new_v4(),
            name: String::from("my_pf"),
            currency: String::from("EUR"),
            deleted: false,
            last_updated_at: Utc::now(),
            max_fee_impact: None,
            fee_type: Some(String::from("Fixed")),
            fee_amount: Some(dec!(2.95)),
            fee_rate: None,
            min_fee: None,
            max_fee: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let asset_model = PortfolioAssetRow {
            id: Uuid::new_v4(),
            symbol: String::from("VWCE"),
            portfolio_id,
            assets_data_id: Uuid::new_v4(),
            name: String::from("Vanguard FTSE All-World UCITS ETF USD Acc"),
            asset_class: AssetClass::Equity,
            asset_class_override: None,
            currency: String::from("EUR"),
            provider: Provider::YF,
            quantity: dec!(10.0),
            target_weight: dec!(1.0),
            manual_price: Some(dec!(100.0)),
            max_fee_impact: None,
            fee_type: None,
            fee_amount: None,
            fee_rate: None,
            min_fee: None,
            max_fee: None,
            average_buy_price: Some(dec!(90.0)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let assets_model = vec![asset_model.clone()];

        let expected = PortfolioResponse {
            id: portfolio_model.id,
            name: portfolio_model.name.clone(),
            quote_ccy: portfolio_model.currency.clone(),
            fees: Some(TransactionFeesResponse {
                max_fee_impact: None,
                fee_structure: FeeStructure::Fixed {
                    fee_amount: dec!(2.95),
                },
            }),
            assets: vec![PortfolioAssetResponse {
                symbol: asset_model.symbol.clone(),
                name: asset_model.name.clone(),
                aclass: asset_model
                    .effective_asset_class()
                    .as_legacy_name()
                    .to_string(),
                base_ccy: asset_model.currency.clone(),
                provider: asset_model.provider.as_legacy_name().to_string(),
                qty: asset_model.quantity,
                target_weight: asset_model.target_weight,
                price: asset_model.manual_price.unwrap(),
                average_buy_price: dec!(90.0),
                fees: None,
            }],
            last_updated_at: portfolio_model.last_updated_at,
        };

        let actual: PortfolioResponse = (portfolio_model, assets_model).try_into().unwrap();
        assert_eq!(actual, expected);

        let serialized = serde_json::to_value(&actual).unwrap();
        assert_eq!(serialized["assets"][0]["qty"], "10.0");
        assert_eq!(serialized["assets"][0]["averageBuyPrice"], "90.0");
        assert_eq!(serialized["fees"]["feeStructure"]["feeAmount"], "2.95");
    }
}
