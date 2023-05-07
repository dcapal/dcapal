use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{AMOUNT_DECIMALS, WEIGHT_DECIMALS};

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub budget: Decimal,
    pub pfolio_ccy: String,
    pub assets: HashMap<String, ProblemAsset>,
    pub total_amount: Decimal,
    pub is_buy_only: bool,
}

#[derive(Debug, Clone)]
pub struct ProblemAsset {
    pub symbol: String,
    pub shares: Decimal,
    pub price: Decimal,
    pub target_weight: Decimal,
    pub is_whole_shares: bool,
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub symbol: String,
    pub price: Decimal,
    pub current_shares: Decimal,
    pub current_amount: Decimal,
    pub current_weight: Decimal,
    pub target_amount: Decimal,
    pub target_weight: Decimal,
    pub is_whole_shares: bool,
    pub shares: Decimal,
    pub amount: Decimal,
    pub weight: Decimal,
}

impl Asset {
    pub fn new(asset: ProblemAsset, total_amount: &Decimal, budget: &Decimal) -> Self {
        let ProblemAsset {
            symbol,
            shares,
            price,
            target_weight,
            is_whole_shares,
        } = asset;

        let current_shares = shares;
        let current_amount = (price * shares).round_dp(AMOUNT_DECIMALS);
        let current_weight = (current_amount / total_amount).round_dp(WEIGHT_DECIMALS);
        let target_amount = (target_weight * budget).round_dp(AMOUNT_DECIMALS);
        let amount = current_amount;
        let weight = current_weight;

        Asset {
            symbol,
            price,
            current_shares,
            current_amount,
            current_weight,
            target_amount,
            target_weight,
            is_whole_shares,
            shares,
            amount,
            weight,
        }
    }
}

pub struct Problem {
    pub(crate) options: ProblemOptions,
    pub(crate) solution: HashMap<String, Asset>,
}

impl Problem {
    pub fn new(options: ProblemOptions) -> Self {
        let solution = options
            .assets
            .iter()
            .map(|(aid, asset)| {
                (
                    aid.clone(),
                    Asset::new(asset.clone(), &options.total_amount, &options.budget),
                )
            })
            .collect::<HashMap<_, _>>();

        Problem { options, solution }
    }
}
