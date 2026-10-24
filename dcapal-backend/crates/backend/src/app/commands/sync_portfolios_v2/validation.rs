use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::app::domain::portfolio::{
    AllocationMode, FeeStructure, PortfolioV2, Provider, StrategicAllocationV2,
    SyncPortfoliosV2Request, TransactionFees,
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
const FIELD_ASSET_NAME: &str = "name";
const FIELD_CURRENCY: &str = "currency";
const FIELD_STRATEGIC_ASSET_CLASS: &str = "assetClass";
const FIELD_QUANTITY: &str = "quantity";
const FIELD_TARGET_WEIGHT: &str = "targetWeight";
const FIELD_MANUAL_PRICE: &str = "manualPrice";
const FIELD_AVERAGE_BUY_PRICE: &str = "averageBuyPrice";
const FIELD_STRATEGIC_ALLOCATION: &str = "strategicAllocation";
const FIELD_ASSET_CLASS_TARGETS: &str = "assetClassTargets";
const FIELD_ASSET_TARGETS: &str = "assetTargets";
const FIELD_PORTFOLIO_ASSET_ID: &str = "portfolioAssetId";
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
const MESSAGE_DUPLICATE_ASSET_ID: &str = "duplicate Portfolio Asset id";
const MESSAGE_DUPLICATE_ASSET_ID_OTHER_PORTFOLIO: &str =
    "Portfolio Asset id is used by another Portfolio in this request";
const MESSAGE_DUPLICATE_IDENTITY: &str = "duplicate Provider and symbol";
const MESSAGE_UUIDV7: &str = "must be a UUIDv7";
const MESSAGE_SIMPLE_TARGET_REQUIRED: &str = "is required in simple allocation mode";
const MESSAGE_STRATEGIC_REQUIRED: &str = "is required in strategic allocation mode";
const MESSAGE_STRATEGIC_TARGET_NULL: &str = "must be null in strategic allocation mode";
const MESSAGE_STRATEGIC_TARGETS_EMPTY: &str = "must be empty in simple allocation mode";
const MESSAGE_ASSET_REFERENCE: &str = "must reference an Asset in the same Portfolio";
const MESSAGE_DUPLICATE_CLASS_TARGET: &str = "duplicate Asset Class target";
const MESSAGE_DUPLICATE_ASSET_TARGET: &str = "duplicate Portfolio Asset target";
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
    /// A submitted Portfolio Asset by its source array indexes.
    PortfolioAsset {
        portfolio_index: usize,
        asset_index: usize,
    },
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
            Self::PortfolioAsset {
                portfolio_index,
                asset_index,
            } => FieldPath::root()
                .field("portfolios")
                .index(portfolio_index)
                .field("assets")
                .index(asset_index)
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
    let mut asset_id_portfolios = HashMap::new();

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
            validate_portfolio(
                portfolio,
                portfolio_index,
                &mut index_path,
                &mut portfolio_ids,
                &deleted_ids,
                &mut asset_id_portfolios,
            );
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
    portfolio_index: usize,
    visitor: &mut PathGuard<'_>,
    portfolio_ids: &mut HashSet<Uuid>,
    deleted_ids: &HashSet<Uuid>,
    asset_id_portfolios: &mut HashMap<Uuid, usize>,
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

    let mut asset_ids = HashSet::new();
    let mut submitted_asset_ids = HashSet::new();
    let mut asset_identity = HashSet::new();
    let mut unsupported_asset_ids = HashSet::new();
    let mut retained_assets = Vec::with_capacity(portfolio.assets.len());
    let mut retained_indices = Vec::with_capacity(portfolio.assets.len());

    {
        let mut assets_path = visitor.enter_field(FIELD_ASSETS);
        for (asset_index, mut asset) in portfolio.assets.drain(..).enumerate() {
            let mut asset_path = assets_path.enter_index(asset_index);
            if !submitted_asset_ids.insert(asset.id) {
                asset_path.issue_field(FIELD_ID, MESSAGE_DUPLICATE_ASSET_ID);
            }
            if let Some(first_portfolio) = asset_id_portfolios.insert(asset.id, portfolio_index)
                && first_portfolio != portfolio_index
            {
                asset_path.issue_field(FIELD_ID, MESSAGE_DUPLICATE_ASSET_ID_OTHER_PORTFOLIO);
            }
            if matches!(&asset.asset.provider, Provider::Unsupported(_)) {
                unsupported_asset_ids.insert(asset.id);
                continue;
            }
            if asset.id.get_version_num() != 7 {
                asset_path.issue_field(FIELD_ID, MESSAGE_UUIDV7);
            }
            asset.asset.symbol = asset.asset.symbol.trim().to_ascii_uppercase();
            if asset.asset.symbol.is_empty() {
                let mut asset_data_path = asset_path.enter_field(FIELD_ASSET);
                asset_data_path.issue_field(FIELD_SYMBOL, MESSAGE_NOT_EMPTY);
            }
            if asset.asset.name.trim().is_empty() {
                let mut asset_data_path = asset_path.enter_field(FIELD_ASSET);
                asset_data_path.issue_field(FIELD_ASSET_NAME, MESSAGE_NOT_EMPTY);
            }
            if asset.asset.currency.trim().is_empty() {
                let mut asset_data_path = asset_path.enter_field(FIELD_ASSET);
                asset_data_path.issue_field(FIELD_CURRENCY, MESSAGE_NOT_EMPTY);
            }
            asset_ids.insert(asset.id);
            if !asset_identity.insert((asset.asset.provider.clone(), asset.asset.symbol.clone())) {
                let mut asset_data_path = asset_path.enter_field(FIELD_ASSET);
                asset_data_path.issue_here(MESSAGE_DUPLICATE_IDENTITY);
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

            retained_indices.push(asset_index);
            retained_assets.push(asset);
        }
    }
    portfolio.assets = retained_assets;

    let retained_strategic_target_indices =
        portfolio.strategic_allocation.as_mut().map(|strategic| {
            let mut retained_targets = Vec::with_capacity(strategic.asset_targets.len());
            let mut retained_indices = Vec::with_capacity(strategic.asset_targets.len());
            for (index, target) in strategic.asset_targets.drain(..).enumerate() {
                if unsupported_asset_ids.contains(&target.portfolio_asset_id) {
                    continue;
                }
                retained_indices.push(index);
                retained_targets.push(target);
            }
            strategic.asset_targets = retained_targets;
            retained_indices
        });

    match portfolio.allocation_mode {
        AllocationMode::Simple => {
            {
                let mut assets_path = visitor.enter_field(FIELD_ASSETS);
                for (asset_index, asset) in portfolio.assets.iter().enumerate() {
                    let mut asset_path = assets_path.enter_index(retained_indices[asset_index]);
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
                    let mut asset_path = assets_path.enter_index(retained_indices[asset_index]);
                    if asset.target_weight.is_some() {
                        asset_path.issue_field(FIELD_TARGET_WEIGHT, MESSAGE_STRATEGIC_TARGET_NULL);
                    }
                }
            }
        }
    }

    if let Some(strategic) = &mut portfolio.strategic_allocation {
        validate_strategic(
            strategic,
            visitor,
            &asset_ids,
            retained_strategic_target_indices
                .as_deref()
                .unwrap_or_default(),
        );
    }
}

fn validate_strategic(
    strategic: &mut StrategicAllocationV2,
    visitor: &mut PathGuard<'_>,
    asset_ids: &HashSet<Uuid>,
    asset_target_indices: &[usize],
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
            let mut index_path = asset_targets_path
                .enter_index(asset_target_indices.get(index).copied().unwrap_or(index));
            if !asset_ids.contains(&target.portfolio_asset_id) {
                index_path.issue_field(FIELD_PORTFOLIO_ASSET_ID, MESSAGE_ASSET_REFERENCE);
            }
            if !asset_targets.insert(target.portfolio_asset_id) {
                index_path.issue_field(FIELD_PORTFOLIO_ASSET_ID, MESSAGE_DUPLICATE_ASSET_TARGET);
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
        AssetClass, PortfolioAssetV2, SharedAssetV2, StrategicAssetTargetV2,
    };

    fn id(value: u8) -> Uuid {
        Uuid::parse_str(&format!("00000000-0000-7000-8000-0000000000{value:02}")).unwrap()
    }

    fn asset(asset_id: Uuid, provider: &str) -> PortfolioAssetV2 {
        PortfolioAssetV2 {
            id: asset_id,
            asset: SharedAssetV2 {
                provider: Provider::parse(provider)
                    .unwrap_or_else(|| Provider::Unsupported(provider.to_string())),
                symbol: " vwce ".to_string(),
                name: "ETF".to_string(),
                currency: "EUR".to_string(),
                asset_class: AssetClass::Equity,
            },
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

    #[test]
    fn normalizes_supported_values_without_changing_decimal_scale() {
        // GIVEN a supported aggregate, WHEN validation runs, THEN exact Decimal values remain.
        let mut request = request(vec![asset(id(1), "YF")]);
        request.portfolios[0].assets[0].asset.symbol = " vwce ".to_string();
        let request = validate_and_normalize(request).expect("request should be valid");
        assert_eq!(request.portfolios[0].assets[0].asset.provider, Provider::YF);
        assert_eq!(request.portfolios[0].assets[0].asset.symbol, "VWCE");
        assert_eq!(request.portfolios[0].assets[0].quantity, dec!(1.20));
    }

    #[test]
    fn reports_nested_paths_and_uuid_policy() {
        // GIVEN malformed values, WHEN validation runs, THEN nested paths are retained.
        let mut request = request(vec![asset(id(1), "YF")]);
        request.portfolios[0].assets[0].id = Uuid::new_v4();
        request.portfolios[0].assets[0].quantity = dec!(-1);
        let error = validate_and_normalize(request).expect_err("invalid request");
        let fields: Vec<_> = error
            .issues
            .iter()
            .map(|issue| issue.path.render())
            .collect();
        assert!(fields.contains(&"portfolios[0].assets[0].id".to_string()));
        assert!(fields.contains(&"portfolios[0].assets[0].quantity".to_string()));
    }

    #[test]
    fn rejects_cross_portfolio_asset_id_reuse() {
        // GIVEN two Portfolios reuse an Asset id, WHEN validation runs, THEN the second path fails.
        let shared_id = id(1);
        let mut request = request(vec![asset(shared_id, "yf")]);
        let mut second = request.portfolios[0].clone();
        second.id = id(11);
        second.assets = vec![asset(shared_id, "yf")];
        request.portfolios.push(second);
        let error = validate_and_normalize(request).expect_err("duplicate id");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[1].assets[0].id"
                && issue.message == MESSAGE_DUPLICATE_ASSET_ID_OTHER_PORTFOLIO
        }));
    }

    #[test]
    fn rejects_cross_portfolio_asset_id_reuse_when_first_provider_is_unsupported() {
        // GIVEN an unsupported Asset in one Portfolio and a supported Asset with the same id
        // in another, WHEN validation runs, THEN the second relationship is rejected.
        let shared_id = id(1);
        let mut request = request(vec![asset(shared_id, "ibkr")]);
        let mut second = request.portfolios[0].clone();
        second.id = id(11);
        second.assets = vec![asset(shared_id, "yf")];
        request.portfolios.push(second);

        let error = validate_and_normalize(request).expect_err("duplicate id");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[1].assets[0].id"
                && issue.message == MESSAGE_DUPLICATE_ASSET_ID_OTHER_PORTFOLIO
        }));
    }

    #[test]
    fn rejects_same_portfolio_asset_id_reuse_across_provider_filtering() {
        // GIVEN an unsupported Asset followed by a supported Asset with the
        // same relationship id, WHEN validation runs, THEN the duplicate is
        // reported instead of being hidden by provider filtering.
        let shared_id = id(1);
        let request = request(vec![asset(shared_id, "ibkr"), asset(shared_id, "yf")]);

        let error = validate_and_normalize(request).expect_err("duplicate id");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render() == "portfolios[0].assets[1].id"
                && issue.message == MESSAGE_DUPLICATE_ASSET_ID
        }));
    }

    #[test]
    fn keeps_unknown_strategic_references_for_validation() {
        // GIVEN a Strategic target naming no Asset, WHEN validation runs, THEN it is reported.
        let mut request = request(vec![asset(id(1), "yf")]);
        let portfolio = &mut request.portfolios[0];
        portfolio.allocation_mode = AllocationMode::Strategic;
        portfolio.assets[0].target_weight = None;
        portfolio.strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: Vec::new(),
            asset_targets: vec![StrategicAssetTargetV2 {
                portfolio_asset_id: id(99),
                relative_target_weight: dec!(100),
            }],
        });
        let error = validate_and_normalize(request).expect_err("unknown reference");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render()
                == "portfolios[0].strategicAllocation.assetTargets[0].portfolioAssetId"
        }));
    }

    #[test]
    fn drops_strategic_targets_for_unsupported_assets() {
        // GIVEN an unsupported Asset and dependent target, WHEN validation runs, THEN both drop.
        let unsupported_id = id(2);
        let mut request = request(vec![asset(id(1), "yf"), asset(unsupported_id, "ibkr")]);
        let portfolio = &mut request.portfolios[0];
        portfolio.allocation_mode = AllocationMode::Strategic;
        portfolio.assets[0].target_weight = None;
        portfolio.strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: Vec::new(),
            asset_targets: vec![StrategicAssetTargetV2 {
                portfolio_asset_id: unsupported_id,
                relative_target_weight: dec!(100),
            }],
        });
        let request = validate_and_normalize(request).expect("unsupported dependency");
        assert!(
            request.portfolios[0]
                .assets
                .iter()
                .all(|asset| asset.id != unsupported_id)
        );
        assert!(
            request.portfolios[0]
                .strategic_allocation
                .as_ref()
                .expect("strategic allocation")
                .asset_targets
                .is_empty()
        );
    }

    #[test]
    fn preserves_original_strategic_target_indexes_after_dependency_filtering() {
        // GIVEN a dropped target before an invalid target, WHEN validation runs, THEN source index remains.
        let unsupported_id = id(2);
        let mut request = request(vec![asset(id(1), "yf"), asset(unsupported_id, "ibkr")]);
        let portfolio = &mut request.portfolios[0];
        portfolio.allocation_mode = AllocationMode::Strategic;
        portfolio.assets[0].target_weight = None;
        portfolio.strategic_allocation = Some(StrategicAllocationV2 {
            asset_class_targets: Vec::new(),
            asset_targets: vec![
                StrategicAssetTargetV2 {
                    portfolio_asset_id: unsupported_id,
                    relative_target_weight: dec!(100),
                },
                StrategicAssetTargetV2 {
                    portfolio_asset_id: id(99),
                    relative_target_weight: dec!(100),
                },
            ],
        });
        let error = validate_and_normalize(request).expect_err("unknown reference");
        assert!(error.issues.iter().any(|issue| {
            issue.path.render()
                == "portfolios[0].strategicAllocation.assetTargets[1].portfolioAssetId"
        }));
    }

    #[test]
    fn preserves_empty_strategic_data_in_simple_mode() {
        // GIVEN an empty Strategic object in Simple mode, WHEN validation runs,
        // THEN the explicit object remains present so null and empty stay distinct.
        let mut request = request(vec![asset(id(1), "yf")]);
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
        let mut request = request(vec![asset(id(1), "yf")]);
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
        let mut request = request(vec![asset(id(1), "yf")]);
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
