use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub budget: Decimal,
    pub pfolio_ccy: String,
    pub assets: HashMap<String, ProblemAsset>,
    pub is_buy_only: bool,
}

#[derive(Debug, Clone)]
pub struct ProblemAsset {
    pub symbol: String,
    pub shares: Decimal,
    pub current_amount: Decimal,
    pub price: Decimal,
    pub target_weight: Decimal,
    pub is_whole_shares: bool,
}
