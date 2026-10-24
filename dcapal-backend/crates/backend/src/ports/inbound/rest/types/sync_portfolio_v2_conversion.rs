use super::*;

impl From<V2ConversionError> for V2ValidationError {
    fn from(error: V2ConversionError) -> Self {
        Self {
            response_code: VALIDATION_CODE,
            response_message: VALIDATION_MESSAGE,
            errors: error.errors,
        }
    }
}

impl V2ConversionError {
    fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![V2FieldError {
                field: field.into(),
                message: message.into(),
            }],
        }
    }

    fn prepend(mut self, prefix: &str) -> Self {
        for error in &mut self.errors {
            error.field = format!("{prefix}.{}", error.field);
        }
        self
    }

    fn extend(&mut self, other: Self) {
        self.errors.extend(other.errors);
    }
}

impl From<ValidationError> for V2ValidationError {
    fn from(error: ValidationError) -> Self {
        Self {
            response_code: VALIDATION_CODE,
            response_message: VALIDATION_MESSAGE,
            errors: error
                .issues
                .into_iter()
                .map(|issue| V2FieldError {
                    field: issue.path.render(),
                    message: issue.message.to_string(),
                })
                .collect(),
        }
    }
}

impl TryFrom<SyncPortfoliosV2Request> for app::SyncPortfoliosV2Request {
    type Error = V2ConversionError;

    fn try_from(request: SyncPortfoliosV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ConversionError { errors: Vec::new() };
        let mut portfolios = Vec::with_capacity(request.portfolios.len());
        for (index, portfolio) in request.portfolios.into_iter().enumerate() {
            match app::PortfolioV2::try_from(portfolio) {
                Ok(portfolio) => portfolios.push(portfolio),
                Err(error) => errors.extend(error.prepend(&format!("portfolios[{index}]"))),
            }
        }
        if errors.errors.is_empty() {
            Ok(Self {
                schema_version: request.schema_version,
                portfolios,
                deleted_portfolios: request.deleted_portfolios,
            })
        } else {
            Err(errors)
        }
    }
}

impl TryFrom<PortfolioV2Request> for app::PortfolioV2 {
    type Error = V2ConversionError;

    fn try_from(portfolio: PortfolioV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ConversionError { errors: Vec::new() };
        let mut assets = Vec::with_capacity(portfolio.assets.len());
        for (index, asset) in portfolio.assets.into_iter().enumerate() {
            match app::PortfolioAssetV2::try_from(asset) {
                Ok(asset) => assets.push(asset),
                Err(error) => errors.extend(error.prepend(&format!("assets[{index}]"))),
            }
        }
        let strategic_allocation = match portfolio.strategic_allocation {
            Some(allocation) => match app::StrategicAllocationV2::try_from(allocation) {
                Ok(allocation) => Some(allocation),
                Err(error) => {
                    errors.extend(error.prepend("strategicAllocation"));
                    None
                }
            },
            None => None,
        };
        if errors.errors.is_empty() {
            Ok(Self {
                id: portfolio.id,
                name: portfolio.name,
                quote_currency: portfolio.quote_currency,
                allocation_mode: portfolio.allocation_mode.into(),
                drift_band: portfolio.drift_band,
                fees: portfolio.fees.map(Into::into),
                assets,
                strategic_allocation,
                last_updated_at: portfolio.last_updated_at,
            })
        } else {
            Err(errors)
        }
    }
}

impl From<AllocationModeWire> for app::AllocationMode {
    fn from(mode: AllocationModeWire) -> Self {
        match mode {
            AllocationModeWire::Simple => Self::Simple,
            AllocationModeWire::Strategic => Self::Strategic,
        }
    }
}

impl TryFrom<PortfolioAssetV2Request> for app::PortfolioAssetV2 {
    type Error = V2ConversionError;

    fn try_from(asset: PortfolioAssetV2Request) -> Result<Self, Self::Error> {
        let provider = app::Provider::parse(&asset.asset.provider).unwrap_or_else(|| {
            app::Provider::Unsupported(asset.asset.provider.trim().to_ascii_lowercase())
        });
        let unsupported_provider = matches!(provider, app::Provider::Unsupported(_));
        let shared_asset = app::SharedAssetV2::try_from(asset.asset)?;
        let asset_class_override = match asset.aclass_override {
            Some(value) => match app::AssetClass::parse(&value) {
                Some(class) => Some(class),
                None if unsupported_provider => None,
                None => {
                    return Err(V2ConversionError::field(
                        "aclassOverride",
                        KNOWN_ASSET_CLASS_MESSAGE,
                    ));
                }
            },
            None => None,
        };

        Ok(Self {
            id: asset.id,
            asset: shared_asset,
            asset_class_override,
            quantity: asset.quantity,
            target_weight: asset.target_weight,
            manual_price: asset.manual_price,
            average_buy_price: asset.average_buy_price,
            fees: asset.fees.map(Into::into),
        })
    }
}

impl TryFrom<SharedAssetV2Request> for app::SharedAssetV2 {
    type Error = V2ConversionError;

    fn try_from(asset: SharedAssetV2Request) -> Result<Self, Self::Error> {
        let provider = app::Provider::parse(&asset.provider).unwrap_or_else(|| {
            app::Provider::Unsupported(asset.provider.trim().to_ascii_lowercase())
        });
        let asset_class = app::AssetClass::parse(&asset.aclass)
            .or_else(|| {
                matches!(provider, app::Provider::Unsupported(_)).then_some(app::AssetClass::Other)
            })
            .ok_or_else(|| V2ConversionError::field("asset.aclass", ASSET_CLASS_ENUM_MESSAGE))?;
        Ok(Self {
            provider,
            symbol: asset.symbol,
            name: asset.name,
            currency: asset.currency,
            asset_class,
        })
    }
}

impl TryFrom<StrategicAllocationV2Request> for app::StrategicAllocationV2 {
    type Error = V2ConversionError;

    fn try_from(allocation: StrategicAllocationV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ConversionError { errors: Vec::new() };
        let mut asset_class_targets = Vec::with_capacity(allocation.asset_class_targets.len());
        for (index, target) in allocation.asset_class_targets.into_iter().enumerate() {
            match app::StrategicAssetClassTargetV2::try_from(target) {
                Ok(target) => asset_class_targets.push(target),
                Err(error) => errors.extend(error.prepend(&format!("assetClassTargets[{index}]"))),
            }
        }
        if errors.errors.is_empty() {
            Ok(Self {
                asset_class_targets,
                asset_targets: allocation
                    .asset_targets
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            })
        } else {
            Err(errors)
        }
    }
}

impl TryFrom<StrategicAssetClassTargetV2Request> for app::StrategicAssetClassTargetV2 {
    type Error = V2ConversionError;

    fn try_from(target: StrategicAssetClassTargetV2Request) -> Result<Self, Self::Error> {
        Ok(Self {
            asset_class: app::AssetClass::parse(&target.asset_class)
                .ok_or_else(|| V2ConversionError::field("assetClass", KNOWN_ASSET_CLASS_MESSAGE))?,
            target_weight: target.target_weight,
        })
    }
}

impl From<StrategicAssetTargetV2Request> for app::StrategicAssetTargetV2 {
    fn from(target: StrategicAssetTargetV2Request) -> Self {
        Self {
            portfolio_asset_id: target.portfolio_asset_id,
            relative_target_weight: target.relative_target_weight,
        }
    }
}

impl From<TransactionFeesV2Request> for app::TransactionFees {
    fn from(fees: TransactionFeesV2Request) -> Self {
        Self {
            max_fee_impact: fees.max_fee_impact,
            fee_structure: fees.fee_structure.into(),
        }
    }
}

impl From<FeeStructureV2> for app::FeeStructure {
    fn from(fees: FeeStructureV2) -> Self {
        match fees {
            FeeStructureV2::ZeroFee => Self::ZeroFee,
            FeeStructureV2::Fixed { fee_amount } => Self::Fixed { fee_amount },
            FeeStructureV2::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => Self::Variable {
                fee_rate,
                min_fee,
                max_fee,
            },
        }
    }
}

impl From<app::SyncPortfoliosV2Response> for SyncPortfoliosV2Response {
    fn from(response: app::SyncPortfoliosV2Response) -> Self {
        Self {
            updated_portfolios: response
                .updated_portfolios
                .into_iter()
                .map(Into::into)
                .collect(),
            deleted_portfolios: response.deleted_portfolios,
        }
    }
}

impl From<app::PortfolioV2> for PortfolioV2Response {
    fn from(portfolio: app::PortfolioV2) -> Self {
        Self {
            id: portfolio.id,
            name: portfolio.name,
            quote_currency: portfolio.quote_currency,
            allocation_mode: portfolio.allocation_mode.into(),
            drift_band: portfolio.drift_band,
            fees: portfolio.fees.map(Into::into),
            assets: portfolio.assets.into_iter().map(Into::into).collect(),
            strategic_allocation: portfolio.strategic_allocation.map(Into::into),
            last_updated_at: portfolio.last_updated_at,
        }
    }
}

impl From<app::AllocationMode> for AllocationModeWire {
    fn from(mode: app::AllocationMode) -> Self {
        match mode {
            app::AllocationMode::Simple => Self::Simple,
            app::AllocationMode::Strategic => Self::Strategic,
        }
    }
}

impl From<app::PortfolioAssetV2> for PortfolioAssetV2Response {
    fn from(asset: app::PortfolioAssetV2) -> Self {
        Self {
            id: asset.id,
            asset: asset.asset.into(),
            aclass_override: asset
                .asset_class_override
                .map(|class| class.as_str().to_string()),
            quantity: asset.quantity,
            target_weight: asset.target_weight,
            manual_price: asset.manual_price,
            average_buy_price: asset.average_buy_price,
            fees: asset.fees.map(Into::into),
        }
    }
}

impl From<app::SharedAssetV2> for SharedAssetV2Response {
    fn from(asset: app::SharedAssetV2) -> Self {
        Self {
            provider: asset.provider.as_str().to_string(),
            symbol: asset.symbol,
            name: asset.name,
            currency: asset.currency,
            aclass: asset.asset_class.as_str().to_string(),
        }
    }
}

impl From<app::StrategicAllocationV2> for StrategicAllocationV2Response {
    fn from(allocation: app::StrategicAllocationV2) -> Self {
        Self {
            asset_class_targets: allocation
                .asset_class_targets
                .into_iter()
                .map(Into::into)
                .collect(),
            asset_targets: allocation
                .asset_targets
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<app::StrategicAssetClassTargetV2> for StrategicAssetClassTargetV2Response {
    fn from(target: app::StrategicAssetClassTargetV2) -> Self {
        Self {
            asset_class: target.asset_class.as_str().to_string(),
            target_weight: target.target_weight,
        }
    }
}

impl From<app::StrategicAssetTargetV2> for StrategicAssetTargetV2Response {
    fn from(target: app::StrategicAssetTargetV2) -> Self {
        Self {
            portfolio_asset_id: target.portfolio_asset_id,
            relative_target_weight: target.relative_target_weight,
        }
    }
}

impl From<app::TransactionFees> for TransactionFeesV2Response {
    fn from(fees: app::TransactionFees) -> Self {
        Self {
            max_fee_impact: fees.max_fee_impact,
            fee_structure: fees.fee_structure.into(),
        }
    }
}

impl From<app::FeeStructure> for FeeStructureV2 {
    fn from(fees: app::FeeStructure) -> Self {
        match fees {
            app::FeeStructure::ZeroFee => Self::ZeroFee,
            app::FeeStructure::Fixed { fee_amount } => Self::Fixed { fee_amount },
            app::FeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => Self::Variable {
                fee_rate,
                min_fee,
                max_fee,
            },
        }
    }
}
