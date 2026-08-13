pub mod advanced;
pub mod basic;
pub mod suggestions;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::AMOUNT_DECIMALS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFees {
    #[serde(
        with = "rust_decimal::serde::float",
        default = "TransactionFees::default_max_fee_impact"
    )]
    /// Maximum acceptable fee impact (as a rate, in [0..1] range)
    pub max_fee_impact: Decimal,
    #[serde(default)]
    pub fee_structure: FeeStructure,
}

impl Default for TransactionFees {
    fn default() -> Self {
        Self {
            max_fee_impact: Self::default_max_fee_impact(),
            fee_structure: Default::default(),
        }
    }
}

impl TransactionFees {
    pub fn default_max_fee_impact() -> Decimal {
        Decimal::MAX
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FeeStructure {
    Fixed(FeeStructureFixed),
    Variable(FeeStructureVariable),
}

impl FeeStructure {
    pub fn compute_fee(&self, amount: &Decimal) -> Decimal {
        match self {
            FeeStructure::Fixed(fee) => {
                if *amount > Decimal::ZERO {
                    fee.fee_amount
                } else {
                    Decimal::ZERO
                }
            }
            FeeStructure::Variable(fee) => fee.compute_fee(amount),
        }
    }
}

impl Default for FeeStructure {
    fn default() -> Self {
        FeeStructure::Fixed(FeeStructureFixed {
            fee_amount: Decimal::ZERO,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeStructureFixed {
    #[serde(with = "rust_decimal::serde::float")]
    pub fee_amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeStructureVariable {
    #[serde(with = "rust_decimal::serde::float_option")]
    pub min_fee: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub max_fee: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float")]
    pub fee_rate: Decimal,
}

impl FeeStructureVariable {
    pub fn compute_fee(&self, amount: &Decimal) -> Decimal {
        (self.fee_rate * amount)
            .clamp(
                self.min_fee.unwrap_or(Decimal::ZERO),
                self.max_fee.unwrap_or(Decimal::MAX),
            )
            .round_dp(AMOUNT_DECIMALS)
    }
}
