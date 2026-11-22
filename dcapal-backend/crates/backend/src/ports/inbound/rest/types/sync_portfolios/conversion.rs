//! Conversions between the v1 sync wire types and domain or storage types.

use rust_decimal::Decimal;

use super::{
    FeeStructure, PortfolioAssetResponse, PortfolioResponse, TransactionFeesRequest,
    TransactionFeesResponse,
};
use crate::{
    app::domain::portfolio::FeeStructure as AppFeeStructure,
    error::DcaError,
    ports::outbound::repository::postgres::types::{FeeFields, PortfolioAssetRow, PortfolioRow},
};

impl From<FeeStructure> for crate::app::domain::portfolio::FeeStructure {
    fn from(fees: FeeStructure) -> Self {
        match fees {
            FeeStructure::ZeroFee => Self::ZeroFee,
            FeeStructure::Fixed { fee_amount } => Self::Fixed { fee_amount },
            FeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => Self::Variable {
                fee_rate,
                min_fee: Some(min_fee),
                max_fee,
            },
        }
    }
}

impl From<TransactionFeesRequest> for crate::app::domain::portfolio::TransactionFees {
    fn from(fees: TransactionFeesRequest) -> Self {
        Self {
            max_fee_impact: fees.max_fee_impact,
            fee_structure: fees.fee_structure.into(),
        }
    }
}

/// Builds the v1 fee settings from the stored fee columns of a portfolio or
/// asset, or `None` when no fee type is stored.
fn legacy_fees(fields: &FeeFields) -> Result<Option<TransactionFeesResponse>, DcaError> {
    let fees = fields
        .clone()
        .into_fees()
        .map_err(|error| DcaError::Generic(error.to_string()))?;
    Ok(fees.map(|fees| TransactionFeesResponse {
        max_fee_impact: fees.max_fee_impact,
        fee_structure: match fees.fee_structure {
            AppFeeStructure::ZeroFee => FeeStructure::ZeroFee,
            AppFeeStructure::Fixed { fee_amount } => FeeStructure::Fixed { fee_amount },
            AppFeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => FeeStructure::Variable {
                fee_rate,
                // v2 treats an absent lower bound as no minimum;
                // v1 can represent that policy with a zero bound.
                min_fee: min_fee.unwrap_or(Decimal::ZERO),
                max_fee,
            },
        },
    }))
}

impl TryFrom<(PortfolioRow, Vec<PortfolioAssetRow>)> for PortfolioResponse {
    type Error = DcaError;

    fn try_from(input: (PortfolioRow, Vec<PortfolioAssetRow>)) -> Result<Self, Self::Error> {
        let (portfolio, assets) = input;
        let portfolio_assets: Vec<PortfolioAssetResponse> = assets
            .iter()
            .map(|asset| {
                let fees = legacy_fees(&asset.fees)?;

                Ok(PortfolioAssetResponse {
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    aclass: asset.effective_asset_class().as_legacy_name().to_string(),
                    base_ccy: asset.currency.clone(),
                    provider: asset.provider.as_legacy_name().to_string(),
                    qty: asset.quantity,
                    target_weight: asset.legacy_target_weight(),
                    // v2 makes both values optional; v1 requires both, so each stands in
                    // for the other and only an asset with neither cannot be projected.
                    price: asset
                        .manual_price
                        .or(asset.average_buy_price)
                        .ok_or_else(|| {
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
            .collect::<Result<Vec<_>, DcaError>>()?;

        Ok(Self {
            id: portfolio.id,
            name: portfolio.name.clone(),
            quote_ccy: portfolio.currency.clone(),
            fees: legacy_fees(&portfolio.fees)?,
            assets: portfolio_assets,
            last_updated_at: portfolio.last_updated_at,
        })
    }
}

#[cfg(test)]
mod test {

    use jiff::Timestamp;
    use rust_decimal::dec;
    use uuid::Uuid;

    use super::*;
    use crate::ports::outbound::repository::postgres::types::{
        AllocationMode as DbAllocationMode, AssetClass, FeeType as DbFeeType, Provider,
    };

    #[test]
    fn map_model_to_response() {
        let portfolio_id = Uuid::new_v4();

        let portfolio_model = PortfolioRow {
            id: portfolio_id,
            user_id: Uuid::new_v4(),
            name: String::from("my_pf"),
            currency: String::from("EUR"),
            deleted: false,
            last_updated_at: Timestamp::now(),
            fees: FeeFields {
                max_fee_impact: None,
                fee_type: Some(DbFeeType::Fixed),
                fee_amount: Some(dec!(2.95)),
                fee_rate: None,
                min_fee: None,
                max_fee: None,
            },
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
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
            allocation_mode: DbAllocationMode::Simple,
            weight: dec!(1.0),
            class_target: None,
            manual_price: Some(dec!(100.0)),
            fees: FeeFields {
                max_fee_impact: None,
                fee_type: None,
                fee_amount: None,
                fee_rate: None,
                min_fee: None,
                max_fee: None,
            },
            average_buy_price: Some(dec!(90.0)),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
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
                target_weight: asset_model.legacy_target_weight(),
                price: asset_model.manual_price.unwrap(),
                average_buy_price: dec!(90.0),
                fees: None,
            }],
            last_updated_at: portfolio_model.last_updated_at,
        };

        let actual: PortfolioResponse = (portfolio_model.clone(), assets_model.clone())
            .try_into()
            .unwrap();
        assert_eq!(actual, expected);

        let serialized = serde_json::to_value(&actual).unwrap();
        assert_eq!(serialized["assets"][0]["qty"], "10.0");
        assert_eq!(serialized["assets"][0]["averageBuyPrice"], "90.0");
        assert_eq!(serialized["fees"]["feeStructure"]["feeAmount"], "2.95");

        // GIVEN a v2 asset with only one of manual price and average buy price,
        // WHEN it is projected through v1, THEN the present value fills both
        // required legacy fields so v1 sync keeps working.
        let mut missing_manual_price = asset_model.clone();
        missing_manual_price.manual_price = None;
        let response =
            PortfolioResponse::try_from((portfolio_model.clone(), vec![missing_manual_price]))
                .expect("a missing manual price falls back to the cost basis");
        assert_eq!(response.assets[0].price, dec!(90.0));

        let mut missing_average_price = asset_model.clone();
        missing_average_price.average_buy_price = None;
        let response =
            PortfolioResponse::try_from((portfolio_model.clone(), vec![missing_average_price]))
                .expect("a missing cost basis falls back to the manual price");
        assert_eq!(response.assets[0].average_buy_price, dec!(100.0));

        // GIVEN a v2 asset with no cost basis, WHEN it is projected through v1,
        // THEN conversion fails rather than inventing a value.
        let mut missing_all_prices = asset_model;
        missing_all_prices.manual_price = None;
        missing_all_prices.average_buy_price = None;
        let error = PortfolioResponse::try_from((portfolio_model, vec![missing_all_prices]))
            .expect_err("missing manual price must fail closed");

        assert!(matches!(
            error,
            DcaError::Generic(message)
                if message == "v1 Portfolio Asset response requires a manual price."
        ));
    }

    #[test]
    fn v1_projection_maps_unbounded_v2_variable_fees_to_zero_minimum() {
        // GIVEN v2 variable fees with a rate but no minimum bound, WHEN the
        // aggregate is projected through v1, THEN zero represents the same
        // no-minimum policy required by the legacy shape.
        let portfolio_id = Uuid::new_v4();
        let now = Timestamp::now();
        let mut portfolio = PortfolioRow {
            id: portfolio_id,
            user_id: Uuid::new_v4(),
            name: "Portfolio".to_string(),
            currency: "EUR".to_string(),
            deleted: false,
            last_updated_at: now,
            fees: FeeFields {
                max_fee_impact: None,
                fee_type: Some(DbFeeType::Variable),
                fee_amount: None,
                fee_rate: Some(dec!(0.19)),
                min_fee: None,
                max_fee: None,
            },
            created_at: now,
            updated_at: now,
        };
        let mut asset = PortfolioAssetRow {
            id: Uuid::new_v4(),
            symbol: "VWCE".to_string(),
            portfolio_id,
            assets_data_id: Uuid::new_v4(),
            name: "Vanguard FTSE All-World UCITS ETF USD Acc".to_string(),
            asset_class: AssetClass::Equity,
            asset_class_override: None,
            currency: "EUR".to_string(),
            provider: Provider::YF,
            quantity: dec!(10),
            allocation_mode: DbAllocationMode::Simple,
            weight: dec!(1),
            class_target: None,
            manual_price: Some(dec!(100)),
            fees: FeeFields {
                max_fee_impact: None,
                fee_type: Some(DbFeeType::Variable),
                fee_amount: None,
                fee_rate: Some(dec!(0.19)),
                min_fee: None,
                max_fee: None,
            },
            average_buy_price: Some(dec!(90)),
            created_at: now,
            updated_at: now,
        };

        let response = PortfolioResponse::try_from((portfolio.clone(), vec![asset.clone()]))
            .expect("optional v2 fee bounds should project to v1");
        assert!(matches!(
            response.fees,
            Some(TransactionFeesResponse {
                fee_structure: FeeStructure::Variable { min_fee, .. },
                ..
            }) if min_fee == Decimal::ZERO
        ));
        assert!(matches!(
            response.assets[0].fees,
            Some(TransactionFeesResponse {
                fee_structure: FeeStructure::Variable { min_fee, .. },
                ..
            }) if min_fee == Decimal::ZERO
        ));
        let serialized = serde_json::to_value(&response).expect("v1 response should serialize");
        assert_eq!(serialized["fees"]["feeStructure"]["minFee"], "0");
        assert_eq!(
            serialized["assets"][0]["fees"]["feeStructure"]["minFee"],
            "0"
        );

        // Existing explicit v1 bounds remain unchanged by the compatibility mapping.
        portfolio.fees.min_fee = Some(dec!(2.95));
        asset.fees.min_fee = Some(dec!(1.25));
        let response = PortfolioResponse::try_from((portfolio, vec![asset]))
            .expect("explicit v1 fee bounds should remain valid");
        assert!(matches!(
            response.fees,
            Some(TransactionFeesResponse {
                fee_structure: FeeStructure::Variable { min_fee, .. },
                ..
            }) if min_fee == dec!(2.95)
        ));
        assert!(matches!(
            response.assets[0].fees,
            Some(TransactionFeesResponse {
                fee_structure: FeeStructure::Variable { min_fee, .. },
                ..
            }) if min_fee == dec!(1.25)
        ));
    }

    #[test]
    fn response_uses_canonical_v1_provider_and_asset_class_names() {
        // GIVEN canonical Portfolio Asset rows for every supported class, WHEN they are projected,
        // THEN v1 emits uppercase class names and the legacy provider aliases.
        let portfolio_id = Uuid::new_v4();
        let now = Timestamp::now();
        let portfolio = PortfolioRow {
            id: portfolio_id,
            user_id: Uuid::new_v4(),
            name: "Portfolio".to_string(),
            currency: "EUR".to_string(),
            deleted: false,
            last_updated_at: now,
            fees: FeeFields {
                max_fee_impact: None,
                fee_type: None,
                fee_amount: None,
                fee_rate: None,
                min_fee: None,
                max_fee: None,
            },
            created_at: now,
            updated_at: now,
        };

        let expected_classes = [
            (AssetClass::Other, "OTHER"),
            (AssetClass::Equity, "EQUITY"),
            (AssetClass::Bond, "BOND"),
            (AssetClass::Cash, "CURRENCY"),
            (AssetClass::Crypto, "CRYPTO"),
            (AssetClass::Commodity, "COMMODITY"),
        ];
        let assets = expected_classes
            .iter()
            .enumerate()
            .map(|(index, (asset_class, _))| PortfolioAssetRow {
                id: Uuid::new_v4(),
                symbol: format!("ASSET{index}"),
                portfolio_id,
                assets_data_id: Uuid::new_v4(),
                name: "Asset".to_string(),
                asset_class: *asset_class,
                asset_class_override: None,
                currency: "EUR".to_string(),
                provider: if index == 0 {
                    Provider::Kraken
                } else {
                    Provider::YF
                },
                quantity: dec!(1),
                allocation_mode: DbAllocationMode::Simple,
                weight: dec!(1),
                class_target: None,
                manual_price: Some(dec!(100)),
                fees: FeeFields {
                    max_fee_impact: None,
                    fee_type: None,
                    fee_amount: None,
                    fee_rate: None,
                    min_fee: None,
                    max_fee: None,
                },
                average_buy_price: Some(dec!(90)),
                created_at: now,
                updated_at: now,
            })
            .collect::<Vec<_>>();

        let response = PortfolioResponse::try_from((portfolio, assets)).unwrap();

        let actual_classes = response
            .assets
            .iter()
            .map(|asset| asset.aclass.as_str())
            .collect::<Vec<_>>();
        let expected_classes = expected_classes
            .iter()
            .map(|(_, wire_name)| *wire_name)
            .collect::<Vec<_>>();
        assert_eq!(actual_classes, expected_classes);
        assert_eq!(response.assets[0].provider, "DCAPal");
        assert_eq!(response.assets[1].provider, "YF");
    }
}
