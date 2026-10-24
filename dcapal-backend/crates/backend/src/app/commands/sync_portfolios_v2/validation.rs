use std::collections::HashSet;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::app::domain::{
    asset::AssetRef,
    portfolio::{
        AllocationMode, FeeStructure, PortfolioV2, StrategicAllocationV2, SyncPortfoliosV2Request,
        TransactionFees,
    },
};

const SCHEMA_VERSION: u16 = 2;

const FIELD_SCHEMA_VERSION: &str = "schemaVersion";
const FIELD_DELETED_PORTFOLIOS: &str = "deletedPortfolios";
const FIELD_PORTFOLIOS: &str = "portfolios";
const FIELD_ID: &str = "id";
const FIELD_NAME: &str = "name";
const FIELD_QUOTE_CURRENCY: &str = "quoteCurrency";
const FIELD_DRIFT_BAND: &str = "driftBand";
const FIELD_FEES: &str = "fees";
const FIELD_MAX_FEE_IMPACT: &str = "maxFeeImpact";
const FIELD_FEE_STRUCTURE: &str = "feeStructure";
const FIELD_FEE_AMOUNT: &str = "feeAmount";
const FIELD_FEE_RATE: &str = "feeRate";
const FIELD_MIN_FEE: &str = "minFee";
const FIELD_MAX_FEE: &str = "maxFee";
const FIELD_ASSETS: &str = "assets";
const FIELD_ASSET: &str = "asset";
const FIELD_SYMBOL: &str = "symbol";
const FIELD_STRATEGIC_ASSET_CLASS: &str = "assetClass";
const FIELD_QUANTITY: &str = "quantity";
const FIELD_TARGET_WEIGHT: &str = "targetWeight";
const FIELD_MANUAL_PRICE: &str = "manualPrice";
const FIELD_AVERAGE_BUY_PRICE: &str = "averageBuyPrice";
const FIELD_STRATEGIC_ALLOCATION: &str = "strategicAllocation";
const FIELD_ASSET_CLASS_TARGETS: &str = "assetClassTargets";
const FIELD_ASSET_TARGETS: &str = "assetTargets";
const FIELD_RELATIVE_TARGET_WEIGHT: &str = "relativeTargetWeight";

const MESSAGE_SCHEMA_VERSION: &str = "must be 2";
const MESSAGE_DUPLICATE_PORTFOLIO_ID: &str = "duplicate Portfolio id";
const MESSAGE_DUPLICATE_DELETED_ID: &str = "duplicate Portfolio id";
const MESSAGE_PORTFOLIO_DELETED_AND_SYNCED: &str = "cannot be both synchronized and deleted";
const MESSAGE_NOT_EMPTY: &str = "must not be empty";
const MESSAGE_BETWEEN_ZERO_AND_HUNDRED: &str = "must be between 0 and 100";
const MESSAGE_AT_MOST_TWO_DECIMALS: &str = "must have at most two decimal places";
const MESSAGE_NON_NEGATIVE: &str = "must be non-negative";
const MESSAGE_POSITIVE: &str = "must be greater than zero";
const MESSAGE_DUPLICATE_IDENTITY: &str = "duplicate Provider and symbol";
const MESSAGE_SIMPLE_TARGET_REQUIRED: &str = "is required in simple allocation mode";
const MESSAGE_STRATEGIC_REQUIRED: &str = "is required in strategic allocation mode";
const MESSAGE_STRATEGIC_TARGET_NULL: &str = "must be null in strategic allocation mode";
const MESSAGE_STRATEGIC_TARGETS_EMPTY: &str = "must be empty in simple allocation mode";
const MESSAGE_ASSET_REFERENCE: &str = "must reference an Asset in the same Portfolio";
const MESSAGE_DUPLICATE_CLASS_TARGET: &str = "duplicate Asset Class target";
const MESSAGE_DUPLICATE_ASSET_TARGET: &str = "duplicate Portfolio Asset target";
/// Message for a sync that would onboard more new assets than allowed.
pub const MESSAGE_TOO_MANY_NEW_ASSETS: &str =
    "introduces too many new assets in one synchronization; sync fewer new assets at a time";
pub(super) const MESSAGE_UNRESOLVED_ASSET: &str = "is not a known asset";
const MESSAGE_MAX_AT_LEAST_MIN: &str = "must be greater than or equal to minFee";

/// One semantic validation issue. The REST boundary renders its path.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationIssue {
    pub path: FieldPath,
    pub message: &'static str,
}

/// Transport-neutral validation failure for the v2 synchronization command.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("The Portfolio aggregate is invalid.")]
pub struct ValidationError {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationError {
    /// Creates a validation failure containing one semantic issue.
    pub fn single(path: FieldPath, message: &'static str) -> Self {
        Self {
            issues: vec![ValidationIssue { path, message }],
        }
    }
}

/// A lazily-rendered field path composed of static field names and indexes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldPath {
    segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Field(&'static str),
    Index(usize),
}

impl FieldPath {
    /// Starts an empty path at the request root.
    pub fn root() -> Self {
        Self::default()
    }

    /// Appends a static field segment.
    pub fn field(mut self, field: &'static str) -> Self {
        self.segments.push(PathSegment::Field(field));
        self
    }

    /// Appends an array index segment.
    pub fn index(mut self, index: usize) -> Self {
        self.segments.push(PathSegment::Index(index));
        self
    }

    /// Captures the current path for a validation issue.
    fn issue(&self, message: &'static str) -> ValidationIssue {
        ValidationIssue {
            path: self.clone(),
            message,
        }
    }

    /// Renders the path for a transport boundary or diagnostic.
    pub fn render(&self) -> String {
        let mut rendered = String::new();
        for segment in &self.segments {
            match segment {
                PathSegment::Field(field) => {
                    if !rendered.is_empty() {
                        rendered.push('.');
                    }
                    rendered.push_str(field);
                }
                PathSegment::Index(index) => {
                    rendered.push('[');
                    rendered.push_str(&index.to_string());
                    rendered.push(']');
                }
            }
        }
        rendered
    }
}

/// The logical location of a resource that failed an ownership check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipLocation {
    /// A submitted Portfolio aggregate by its source array index.
    Portfolio { index: usize },
    /// A submitted Portfolio tombstone by its source array index.
    DeletedPortfolio { index: usize },
}

impl OwnershipLocation {
    /// Converts an ownership location into the public v2 field path.
    pub fn field_path(self) -> FieldPath {
        match self {
            Self::Portfolio { index } => FieldPath::root()
                .field("portfolios")
                .index(index)
                .field("id"),
            Self::DeletedPortfolio { index } => {
                FieldPath::root().field("deletedPortfolios").index(index)
            }
        }
    }
}

/// Validates and normalizes a complete v2 synchronization request.
pub(crate) fn validate_and_normalize(
    mut request: SyncPortfoliosV2Request,
) -> Result<SyncPortfoliosV2Request, ValidationError> {
    let mut visitor = ValidationVisitor::default();

    if request.schema_version != SCHEMA_VERSION {
        visitor.issue_field(FIELD_SCHEMA_VERSION, MESSAGE_SCHEMA_VERSION);
    }

    let mut portfolio_ids = HashSet::new();
    let mut deleted_ids = HashSet::new();

    {
        let mut deleted_path = visitor.enter_field(FIELD_DELETED_PORTFOLIOS);
        for (index, deleted_id) in request.deleted_portfolios.iter().enumerate() {
            let mut index_path = deleted_path.enter_index(index);
            if !deleted_ids.insert(*deleted_id) {
                index_path.issue_here(MESSAGE_DUPLICATE_DELETED_ID);
            }
        }
    }

    {
        let mut portfolios_path = visitor.enter_field(FIELD_PORTFOLIOS);
        for (portfolio_index, portfolio) in request.portfolios.iter_mut().enumerate() {
            let mut index_path = portfolios_path.enter_index(portfolio_index);
            validate_portfolio(portfolio, &mut index_path, &mut portfolio_ids, &deleted_ids);
        }
    }

    if visitor.issues.is_empty() {
        Ok(request)
    } else {
        Err(ValidationError {
            issues: visitor.issues,
        })
    }
}

#[derive(Default)]
struct ValidationVisitor {
    path: FieldPath,
    issues: Vec<ValidationIssue>,
}

impl ValidationVisitor {
    fn enter_field(&mut self, field: &'static str) -> PathGuard<'_> {
        self.path.segments.push(PathSegment::Field(field));
        PathGuard {
            path: &mut self.path,
            issues: &mut self.issues,
        }
    }

    fn issue_field(&mut self, field: &'static str, message: &'static str) {
        let mut field_path = self.enter_field(field);
        field_path.issue_here(message);
    }
}

/// RAII scope for one field-path segment.
struct PathGuard<'a> {
    path: &'a mut FieldPath,
    issues: &'a mut Vec<ValidationIssue>,
}

impl PathGuard<'_> {
    fn enter_field(&mut self, field: &'static str) -> PathGuard<'_> {
        self.path.segments.push(PathSegment::Field(field));
        PathGuard {
            path: self.path,
            issues: self.issues,
        }
    }

    fn enter_index(&mut self, index: usize) -> PathGuard<'_> {
        self.path.segments.push(PathSegment::Index(index));
        PathGuard {
            path: self.path,
            issues: self.issues,
        }
    }

    fn issue_here(&mut self, message: &'static str) {
        self.issues.push(self.path.issue(message));
    }

    fn issue_field(&mut self, field: &'static str, message: &'static str) {
        let mut field_path = self.enter_field(field);
        field_path.issue_here(message);
    }
}

impl Drop for PathGuard<'_> {
    fn drop(&mut self) {
        self.path.segments.pop();
    }
}

fn validate_portfolio(
    portfolio: &mut PortfolioV2,
    visitor: &mut PathGuard<'_>,
    portfolio_ids: &mut HashSet<Uuid>,
    deleted_ids: &HashSet<Uuid>,
) {
    if !portfolio_ids.insert(portfolio.id) {
        visitor.issue_field(FIELD_ID, MESSAGE_DUPLICATE_PORTFOLIO_ID);
    }
    if deleted_ids.contains(&portfolio.id) {
        visitor.issue_field(FIELD_ID, MESSAGE_PORTFOLIO_DELETED_AND_SYNCED);
    }
    if portfolio.name.trim().is_empty() {
        visitor.issue_field(FIELD_NAME, MESSAGE_NOT_EMPTY);
    }
    if portfolio.quote_currency.trim().is_empty() {
        visitor.issue_field(FIELD_QUOTE_CURRENCY, MESSAGE_NOT_EMPTY);
    }
    if let Some(value) = portfolio.drift_band
        && validate_percentage(value).is_some()
    {
        visitor.issue_field(FIELD_DRIFT_BAND, MESSAGE_BETWEEN_ZERO_AND_HUNDRED);
    }
    validate_fees(portfolio.fees.as_ref(), visitor);

    let mut asset_refs = HashSet::new();

    {
        let mut assets_path = visitor.enter_field(FIELD_ASSETS);
        for (asset_index, asset) in portfolio.assets.iter_mut().enumerate() {
            let mut asset_path = assets_path.enter_index(asset_index);
            normalize_symbol(&mut asset.asset);
            if asset.asset.symbol.is_empty() {
                let mut reference_path = asset_path.enter_field(FIELD_ASSET);
                reference_path.issue_field(FIELD_SYMBOL, MESSAGE_NOT_EMPTY);
            }
            if !asset_refs.insert(asset.asset.clone()) {
                let mut reference_path = asset_path.enter_field(FIELD_ASSET);
                reference_path.issue_here(MESSAGE_DUPLICATE_IDENTITY);
            }
            if validate_non_negative(asset.quantity).is_some() {
                asset_path.issue_field(FIELD_QUANTITY, MESSAGE_NON_NEGATIVE);
            }
            if let Some(value) = asset.manual_price
                && validate_positive(value).is_some()
            {
                asset_path.issue_field(FIELD_MANUAL_PRICE, MESSAGE_POSITIVE);
            }
            if let Some(value) = asset.average_buy_price
                && validate_positive(value).is_some()
            {
                asset_path.issue_field(FIELD_AVERAGE_BUY_PRICE, MESSAGE_POSITIVE);
            }
            if let Some(value) = asset.target_weight
                && let Some(message) = target_message(value)
            {
                asset_path.issue_field(FIELD_TARGET_WEIGHT, message);
            }
            validate_fees(asset.fees.as_ref(), &mut asset_path);
        }
    }

    if let Some(strategic) = portfolio.strategic_allocation.as_mut() {
        for target in &mut strategic.asset_targets {
            normalize_symbol(&mut target.asset);
        }
    }

    match portfolio.allocation_mode {
        AllocationMode::Simple => {
            {
                let mut assets_path = visitor.enter_field(FIELD_ASSETS);
                for (asset_index, asset) in portfolio.assets.iter().enumerate() {
                    let mut asset_path = assets_path.enter_index(asset_index);
                    if asset.target_weight.is_none() {
                        asset_path.issue_field(FIELD_TARGET_WEIGHT, MESSAGE_SIMPLE_TARGET_REQUIRED);
                    }
                }
            }
            if let Some(allocation) = portfolio.strategic_allocation.as_ref()
                && !allocation.asset_class_targets.is_empty()
            {
                let mut strategic_path = visitor.enter_field(FIELD_STRATEGIC_ALLOCATION);
                strategic_path
                    .issue_field(FIELD_ASSET_CLASS_TARGETS, MESSAGE_STRATEGIC_TARGETS_EMPTY);
            }
            if let Some(allocation) = portfolio.strategic_allocation.as_ref()
                && !allocation.asset_targets.is_empty()
            {
                let mut strategic_path = visitor.enter_field(FIELD_STRATEGIC_ALLOCATION);
                strategic_path.issue_field(FIELD_ASSET_TARGETS, MESSAGE_STRATEGIC_TARGETS_EMPTY);
            }
        }
        AllocationMode::Strategic => {
            if portfolio.strategic_allocation.is_none() {
                visitor.issue_field(FIELD_STRATEGIC_ALLOCATION, MESSAGE_STRATEGIC_REQUIRED);
            }
            {
                let mut assets_path = visitor.enter_field(FIELD_ASSETS);
                for (asset_index, asset) in portfolio.assets.iter().enumerate() {
                    let mut asset_path = assets_path.enter_index(asset_index);
                    if asset.target_weight.is_some() {
                        asset_path.issue_field(FIELD_TARGET_WEIGHT, MESSAGE_STRATEGIC_TARGET_NULL);
                    }
                }
            }
        }
    }

    if let Some(strategic) = &mut portfolio.strategic_allocation {
        validate_strategic(strategic, visitor, &asset_refs);
    }
}

/// Puts an asset reference's symbol in its canonical trimmed upper-case form.
fn normalize_symbol(reference: &mut AssetRef) {
    reference.symbol = reference.symbol.trim().to_ascii_uppercase();
}

fn validate_strategic(
    strategic: &StrategicAllocationV2,
    visitor: &mut PathGuard<'_>,
    asset_refs: &HashSet<AssetRef>,
) {
    let mut strategic_path = visitor.enter_field(FIELD_STRATEGIC_ALLOCATION);
    let mut classes = HashSet::new();
    {
        let mut class_targets_path = strategic_path.enter_field(FIELD_ASSET_CLASS_TARGETS);
        for (index, target) in strategic.asset_class_targets.iter().enumerate() {
            let mut index_path = class_targets_path.enter_index(index);
            if !classes.insert(target.asset_class) {
                index_path.issue_field(FIELD_STRATEGIC_ASSET_CLASS, MESSAGE_DUPLICATE_CLASS_TARGET);
            }
            if let Some(message) = target_message(target.target_weight) {
                index_path.issue_field(FIELD_TARGET_WEIGHT, message);
            }
        }
    }

    let mut asset_targets = HashSet::new();
    {
        let mut asset_targets_path = strategic_path.enter_field(FIELD_ASSET_TARGETS);
        for (index, target) in strategic.asset_targets.iter().enumerate() {
            let mut index_path = asset_targets_path.enter_index(index);
            if !asset_refs.contains(&target.asset) {
                index_path.issue_field(FIELD_ASSET, MESSAGE_ASSET_REFERENCE);
            }
            if !asset_targets.insert(target.asset.clone()) {
                index_path.issue_field(FIELD_ASSET, MESSAGE_DUPLICATE_ASSET_TARGET);
            }
            if let Some(message) = target_message(target.relative_target_weight) {
                index_path.issue_field(FIELD_RELATIVE_TARGET_WEIGHT, message);
            }
        }
    }
}

fn validate_fees(fees: Option<&TransactionFees>, visitor: &mut PathGuard<'_>) {
    let Some(fees) = fees else { return };
    let mut fees_path = visitor.enter_field(FIELD_FEES);
    if let Some(value) = fees.max_fee_impact
        && validate_percentage(value).is_some()
    {
        fees_path.issue_field(FIELD_MAX_FEE_IMPACT, MESSAGE_BETWEEN_ZERO_AND_HUNDRED);
    }
    {
        let mut fee_structure_path = fees_path.enter_field(FIELD_FEE_STRUCTURE);
        match fees.fee_structure {
            FeeStructure::ZeroFee => {}
            FeeStructure::Fixed { fee_amount } => {
                if validate_non_negative(fee_amount).is_some() {
                    fee_structure_path.issue_field(FIELD_FEE_AMOUNT, MESSAGE_NON_NEGATIVE);
                }
            }
            FeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => {
                if validate_percentage(fee_rate).is_some() {
                    fee_structure_path
                        .issue_field(FIELD_FEE_RATE, MESSAGE_BETWEEN_ZERO_AND_HUNDRED);
                }
                if let Some(value) = min_fee
                    && validate_non_negative(value).is_some()
                {
                    fee_structure_path.issue_field(FIELD_MIN_FEE, MESSAGE_NON_NEGATIVE);
                }
                if let Some(value) = max_fee
                    && validate_non_negative(value).is_some()
                {
                    fee_structure_path.issue_field(FIELD_MAX_FEE, MESSAGE_NON_NEGATIVE);
                }
                if let (Some(max_fee), Some(min_fee)) = (max_fee, min_fee)
                    && max_fee < min_fee
                {
                    fee_structure_path.issue_field(FIELD_MAX_FEE, MESSAGE_MAX_AT_LEAST_MIN);
                }
            }
        }
    }
}

fn target_message(value: Decimal) -> Option<&'static str> {
    validate_target_percentage(value)
}

fn validate_percentage(value: Decimal) -> Option<&'static str> {
    (value < Decimal::ZERO || value > Decimal::from(100))
        .then_some(MESSAGE_BETWEEN_ZERO_AND_HUNDRED)
}

fn validate_target_percentage(value: Decimal) -> Option<&'static str> {
    validate_percentage(value)
        .or_else(|| (value.scale() > 2).then_some(MESSAGE_AT_MOST_TWO_DECIMALS))
}

fn validate_non_negative(value: Decimal) -> Option<&'static str> {
    (value < Decimal::ZERO).then_some(MESSAGE_NON_NEGATIVE)
}

fn validate_positive(value: Decimal) -> Option<&'static str> {
    (value <= Decimal::ZERO).then_some(MESSAGE_POSITIVE)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rust_decimal::dec;

    use super::*;
    use crate::app::domain::portfolio::{
        AssetClass, PortfolioAssetV2, Provider, StrategicAssetTargetV2,
    };

    fn id(value: u8) -> Uuid {
        Uuid::parse_str(&format!("00000000-0000-7000-8000-0000000000{value:02}")).unwrap()
    }

    fn reference(symbol: &str) -> AssetRef {
        AssetRef {
            provider: Provider::YF,
            symbol: symbol.to_string(),
        }
    }

    fn asset(symbol: &str) -> PortfolioAssetV2 {
        PortfolioAssetV2 {
            asset: reference(symbol),
            asset_class_override: None,
            quantity: dec!(1.20),
            target_weight: Some(dec!(100.00)),
            manual_price: Some(dec!(120.10)),
            average_buy_price: None,
            fees: None,
        }
    }

    fn request(assets: Vec<PortfolioAssetV2>) -> SyncPortfoliosV2Request {
        SyncPortfoliosV2Request {
            schema_version: 2,
            portfolios: vec![PortfolioV2 {
                id: id(10),
                name: "Draft".to_string(),
                quote_currency: "eur".to_string(),
                allocation_mode: AllocationMode::Simple,
                drift_band: Some(dec!(1.25)),
                fees: None,
                assets,
                strategic_allocation: None,
                last_updated_at: Utc::now(),
            }],
            deleted_portfolios: Vec::new(),
        }
    }

    fn strategic(request: &mut SyncPortfoliosV2Request, targets: Vec<StrategicAssetTargetV2>) {
        let portfolio = &mut request.portfolios[0];
        portfolio.allocation_mode = AllocationMode::Strategic;
        portfolio.assets[0].target_weight = None;
        portfolio.strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: Vec::new(),
            asset_targets: targets,
        });
    }

    fn fields(error: &ValidationError) -> Vec<String> {
        error
            .issues
            .iter()
            .map(|issue| issue.path.render())
            .collect()
    }

    #[test]
    fn normalizes_symbols_without_changing_decimal_scale() {
        // GIVEN a supported aggregate whose symbol has stray whitespace and lower case,
        // WHEN validation runs,
        // THEN the symbol is canonical and exact Decimal values remain.
        let request = validate_and_normalize(request(vec![asset(" vwce ")]))
            .expect("request should be valid");
        assert_eq!(request.portfolios[0].assets[0].asset, reference("VWCE"));
        assert_eq!(request.portfolios[0].assets[0].quantity, dec!(1.20));
    }

    #[test]
    fn reports_nested_paths_for_malformed_values() {
        // GIVEN a negative quantity and an empty symbol, WHEN validation runs,
        // THEN each issue carries its real nested path.
        let mut request = request(vec![asset("  ")]);
        request.portfolios[0].assets[0].quantity = dec!(-1);
        let error = validate_and_normalize(request).expect_err("invalid request");
        let fields = fields(&error);
        assert!(fields.contains(&"portfolios[0].assets[0].asset.symbol".to_string()));
        assert!(fields.contains(&"portfolios[0].assets[0].quantity".to_string()));
    }

    #[test]
    fn rejects_duplicate_references_after_normalization() {
        // GIVEN two assets that differ only by symbol case, WHEN validation runs,
        // THEN the second one is reported as a duplicate reference.
        let error = validate_and_normalize(request(vec![asset("vwce"), asset("VWCE")]))
            .expect_err("duplicate reference");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[0].assets[1].asset"
                && issue.message == MESSAGE_DUPLICATE_IDENTITY
        }));
    }

    #[test]
    fn allows_the_same_reference_in_different_portfolios() {
        // GIVEN two Portfolios holding the same asset, WHEN validation runs,
        // THEN both pass because relationships are scoped to their Portfolio.
        let mut request = request(vec![asset("VWCE")]);
        let mut second = request.portfolios[0].clone();
        second.id = id(11);
        request.portfolios.push(second);
        validate_and_normalize(request).expect("same asset in two Portfolios");
    }

    #[test]
    fn keeps_unknown_strategic_references_for_validation() {
        // GIVEN a Strategic target naming no Asset of the Portfolio, WHEN validation runs,
        // THEN it is reported at the target's asset reference.
        let mut request = request(vec![asset("VWCE")]);
        strategic(
            &mut request,
            vec![StrategicAssetTargetV2 {
                asset: reference("MISSING"),
                relative_target_weight: dec!(100),
            }],
        );
        let error = validate_and_normalize(request).expect_err("unknown reference");
        assert!(
            fields(&error)
                .contains(&"portfolios[0].strategicAllocation.assetTargets[0].asset".to_string())
        );
    }

    #[test]
    fn matches_strategic_targets_by_normalized_reference() {
        // GIVEN a Strategic target whose symbol differs from the asset's only by case
        // and whitespace, WHEN validation runs, THEN the target resolves to the asset.
        let mut request = request(vec![asset("VWCE")]);
        strategic(
            &mut request,
            vec![StrategicAssetTargetV2 {
                asset: reference(" vwce "),
                relative_target_weight: dec!(100),
            }],
        );
        validate_and_normalize(request).expect("normalized target reference");
    }

    #[test]
    fn rejects_duplicate_strategic_asset_targets() {
        // GIVEN two targets for the same asset, WHEN validation runs,
        // THEN the second target is reported as a duplicate.
        let mut request = request(vec![asset("VWCE")]);
        let target = StrategicAssetTargetV2 {
            asset: reference("VWCE"),
            relative_target_weight: dec!(50),
        };
        strategic(&mut request, vec![target.clone(), target]);
        let error = validate_and_normalize(request).expect_err("duplicate target");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[0].strategicAllocation.assetTargets[1].asset"
                && issue.message == MESSAGE_DUPLICATE_ASSET_TARGET
        }));
    }

    #[test]
    fn preserves_empty_strategic_data_in_simple_mode() {
        // GIVEN an empty Strategic object in Simple mode, WHEN validation runs,
        // THEN the explicit object remains present so null and empty stay distinct.
        let mut request = request(vec![asset("VWCE")]);
        request.portfolios[0].strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: Vec::new(),
            asset_targets: Vec::new(),
        });
        let request = validate_and_normalize(request).expect("empty strategy is valid");
        assert!(request.portfolios[0].strategic_allocation.is_some());
    }

    #[test]
    fn rejects_strategic_class_targets_in_simple_mode() {
        // GIVEN a Simple Portfolio with a class target, WHEN validation runs, THEN it fails.
        let mut request = request(vec![asset("VWCE")]);
        request.portfolios[0].strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: vec![crate::app::domain::portfolio::StrategicAssetClassTargetV2 {
                asset_class: AssetClass::Equity,
                target_weight: dec!(100),
            }],
            asset_targets: Vec::new(),
        });
        let error = validate_and_normalize(request).expect_err("class target is invalid");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[0].strategicAllocation.assetClassTargets"
        }));
    }

    #[test]
    fn reports_duplicate_strategic_classes_using_the_wire_field_name() {
        // GIVEN duplicate Strategic Asset Class targets, WHEN validation runs,
        // THEN the issue points to the assetClass field used by the wire contract.
        let mut request = request(vec![asset("VWCE")]);
        request.portfolios[0].allocation_mode = AllocationMode::Strategic;
        request.portfolios[0].assets[0].target_weight = None;
        request.portfolios[0].strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: vec![
                crate::app::domain::portfolio::StrategicAssetClassTargetV2 {
                    asset_class: AssetClass::Equity,
                    target_weight: dec!(50),
                },
                crate::app::domain::portfolio::StrategicAssetClassTargetV2 {
                    asset_class: AssetClass::Equity,
                    target_weight: dec!(50),
                },
            ],
            asset_targets: Vec::new(),
        });

        let error = validate_and_normalize(request).expect_err("duplicate class target");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render()
                == "portfolios[0].strategicAllocation.assetClassTargets[1].assetClass"
        }));
    }

    #[test]
    fn raii_path_scopes_restore_the_parent_path() {
        // GIVEN nested scopes, WHEN child scope exits, THEN the parent path remains active.
        let mut visitor = ValidationVisitor::default();
        {
            let mut portfolios = visitor.enter_field(FIELD_PORTFOLIOS);
            {
                let mut portfolio = portfolios.enter_index(3);
                portfolio.issue_field(FIELD_ID, MESSAGE_SCHEMA_VERSION);
            }
            assert_eq!(portfolios.path.render(), FIELD_PORTFOLIOS);
        }
        assert!(visitor.path.render().is_empty());
        assert_eq!(visitor.issues[0].path.render(), "portfolios[3].id");
    }
}
