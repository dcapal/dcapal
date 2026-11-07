use super::*;
use crate::app::domain::asset::AssetRef;

impl V2ValidationError {
    /// Creates a failure holding one issue for `field`.
    fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![V2FieldError {
                field: field.into(),
                message: message.into(),
            }],
        }
    }

    /// Prefixes the path of every issue with `prefix`.
    fn prepend(mut self, prefix: &str) -> Self {
        for error in &mut self.errors {
            error.field = format!("{prefix}.{}", error.field);
        }
        self
    }

    /// Appends the issues of `other`.
    fn extend(&mut self, other: Self) {
        self.errors.extend(other.errors);
    }
}

/// Converts every item of a REST array, reporting the issues of failed items
/// into `errors` under `<prefix>[<index>]` and returning the converted ones.
fn convert_all<T, U>(items: Vec<T>, prefix: &str, errors: &mut V2ValidationError) -> Vec<U>
where
    U: TryFrom<T, Error = V2ValidationError>,
{
    let mut converted = Vec::with_capacity(items.len());
    for (index, item) in items.into_iter().enumerate() {
        match U::try_from(item) {
            Ok(item) => converted.push(item),
            Err(error) => errors.extend(error.prepend(&format!("{prefix}[{index}]"))),
        }
    }
    converted
}

impl From<ValidationError> for V2ValidationError {
    fn from(error: ValidationError) -> Self {
        Self {
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
    type Error = V2ValidationError;

    fn try_from(request: SyncPortfoliosV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ValidationError::default();
        let portfolios = convert_all(request.portfolios, "portfolios", &mut errors);
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
    type Error = V2ValidationError;

    fn try_from(portfolio: PortfolioV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ValidationError::default();
        let assets = convert_all(portfolio.assets, "assets", &mut errors);
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
    type Error = V2ValidationError;

    fn try_from(asset: PortfolioAssetV2Request) -> Result<Self, Self::Error> {
        let reference = AssetRef::try_from(asset.asset).map_err(|error| error.prepend("asset"))?;
        let asset_class_override = asset
            .aclass_override
            .map(|value| {
                app::AssetClass::parse(&value).ok_or_else(|| {
                    V2ValidationError::field("aclassOverride", KNOWN_ASSET_CLASS_MESSAGE)
                })
            })
            .transpose()?;

        Ok(Self {
            asset: reference,
            asset_class_override,
            quantity: asset.quantity,
            target_weight: asset.target_weight,
            manual_price: asset.manual_price,
            average_buy_price: asset.average_buy_price,
            fees: asset.fees.map(Into::into),
        })
    }
}

impl TryFrom<AssetRefV2> for AssetRef {
    type Error = V2ValidationError;

    fn try_from(reference: AssetRefV2) -> Result<Self, Self::Error> {
        let provider = app::Provider::parse(&reference.provider)
            .ok_or_else(|| V2ValidationError::field("provider", known_provider_message()))?;
        Ok(Self::new(provider, &reference.symbol))
    }
}

impl From<AssetRef> for AssetRefV2 {
    fn from(reference: AssetRef) -> Self {
        Self {
            provider: reference.provider.to_string(),
            symbol: reference.symbol,
        }
    }
}

impl TryFrom<StrategicAllocationV2Request> for app::StrategicAllocationV2 {
    type Error = V2ValidationError;

    fn try_from(allocation: StrategicAllocationV2Request) -> Result<Self, Self::Error> {
        let mut errors = V2ValidationError::default();
        let asset_class_targets = convert_all(
            allocation.asset_class_targets,
            "assetClassTargets",
            &mut errors,
        );
        let asset_targets = convert_all(allocation.asset_targets, "assetTargets", &mut errors);
        if errors.errors.is_empty() {
            Ok(Self {
                asset_class_targets,
                asset_targets,
            })
        } else {
            Err(errors)
        }
    }
}

impl TryFrom<StrategicAssetClassTargetV2Request> for app::StrategicAssetClassTargetV2 {
    type Error = V2ValidationError;

    fn try_from(target: StrategicAssetClassTargetV2Request) -> Result<Self, Self::Error> {
        Ok(Self {
            asset_class: app::AssetClass::parse(&target.asset_class)
                .ok_or_else(|| V2ValidationError::field("assetClass", KNOWN_ASSET_CLASS_MESSAGE))?,
            target_weight: target.target_weight,
        })
    }
}

impl TryFrom<StrategicAssetTargetV2Request> for app::StrategicAssetTargetV2 {
    type Error = V2ValidationError;

    fn try_from(target: StrategicAssetTargetV2Request) -> Result<Self, Self::Error> {
        Ok(Self {
            asset: AssetRef::try_from(target.asset).map_err(|error| error.prepend("asset"))?,
            relative_target_weight: target.relative_target_weight,
        })
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
            asset: asset.asset.into(),
            aclass_override: asset.asset_class_override.map(|class| class.to_string()),
            quantity: asset.quantity,
            target_weight: asset.target_weight,
            manual_price: asset.manual_price,
            average_buy_price: asset.average_buy_price,
            fees: asset.fees.map(Into::into),
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
            asset_class: target.asset_class.to_string(),
            target_weight: target.target_weight,
        }
    }
}

impl From<app::StrategicAssetTargetV2> for StrategicAssetTargetV2Response {
    fn from(target: app::StrategicAssetTargetV2) -> Self {
        Self {
            asset: target.asset.into(),
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
