pub mod basic;

use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub budget: Decimal,
    pub assets: HashMap<String, ProblemAsset>,
    pub is_buy_only: bool,
}

#[derive(Debug, Clone)]
pub struct ProblemAsset {
    pub symbol: String,
    pub target_weight: Decimal,
    pub current_amount: Decimal,
}
