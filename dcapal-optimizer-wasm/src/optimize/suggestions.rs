use std::cmp::Ordering;
use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{AMOUNT_DECIMALS, PERCENTAGE_DECIMALS};

pub struct Problem {
    pub(crate) options: ProblemOptions,
}

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub current_pfolio_amount: Decimal,
    pub assets: HashMap<String, ProblemAsset>,
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
pub struct Solution {
    pub is_solved: bool,
    pub assets: HashMap<String, Asset>,
    pub budget_left: Decimal,
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
    pub fn new(asset: ProblemAsset, current_pfolio_amount: Decimal) -> Self {
        let ProblemAsset {
            symbol,
            shares,
            price,
            target_weight,
            is_whole_shares,
        } = asset;

        let pfolio_amount = current_pfolio_amount;

        let current_shares = shares;
        let current_amount = (price * shares).round_dp(AMOUNT_DECIMALS);
        let current_weight = if current_pfolio_amount > Decimal::ZERO {
            (current_amount / current_pfolio_amount).round_dp(PERCENTAGE_DECIMALS)
        } else {
            Decimal::ZERO
        };
        let target_amount = (target_weight * pfolio_amount).round_dp(AMOUNT_DECIMALS);
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
    pub fn get_allocated_amount(&self) -> Decimal {
        self.amount - self.current_amount
    }
}

impl Solution {
    pub fn new(options: ProblemOptions) -> Self {
        let pfolio_amount = options.current_pfolio_amount;

        let assets = options
            .assets
            .into_iter()
            .map(|(aid, asset)| (aid, Asset::new(asset, pfolio_amount)))
            .collect::<HashMap<_, _>>();

        Self {
            is_solved: false,
            assets,
            budget_left: Decimal::ZERO,
        }
    }
}

impl Problem {
    pub fn new(options: ProblemOptions) -> Self {
        Self { options }
    }

    pub fn suggest_invest_amount(&self) -> Decimal {
        let mut solution = Solution::new(self.options.clone());

        let assets = solution.assets.values_mut().collect::<Vec<_>>();

        calculate_allocation_amount(assets)
    }
}

pub fn calculate_allocation_amount(assets: Vec<&mut Asset>) -> Decimal {
    let overweight_assets = assets
        .iter()
        .filter(|&a| a.current_weight > a.target_weight)
        .collect::<Vec<&&mut Asset>>();

    let max_asset = overweight_assets.iter().max_by(|&&a, &&b| {
        a.current_weight
            .partial_cmp(&b.current_weight)
            .unwrap_or(Ordering::Equal)
    });

    let total_current_value = assets.iter().fold(Decimal::ZERO, |acc, asset| {
        acc + asset.current_shares * asset.price
    });

    match max_asset {
        Some(&asset) => {
            (asset.price * asset.current_shares)
                * (Decimal::new(100, 0) / (asset.target_weight * Decimal::new(100, 0)))
                - total_current_value
        }
        None => Decimal::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal_macros::dec;

    use crate::optimize::suggestions::{Problem, ProblemAsset, ProblemOptions};
    use crate::AMOUNT_DECIMALS;

    #[test_log::test]
    fn it_solves_60_40_portfolio_buy_only() {
        // Given
        let (problem, _assets) = build_60_40_portfolio_no_allocation();

        // When
        let solution = problem.suggest_invest_amount();

        // Expect
        assert_eq!(solution, dec!(72.5));
    }

    fn build_60_40_portfolio_no_allocation() -> (Problem, Vec<String>) {
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    shares: dec!(1.0),
                    price: dec!(100.0),
                    target_weight: dec!(0.6),
                    is_whole_shares: true,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    shares: dec!(23.0),
                    price: dec!(5.0),
                    target_weight: dec!(0.4),
                    is_whole_shares: true,
                },
            ),
        ]);

        let current_pfolio_amount = assets
            .values()
            .map(|a| (a.shares * a.price).round_dp(AMOUNT_DECIMALS))
            .sum();

        let options = ProblemOptions {
            current_pfolio_amount,
            assets,
        };

        (Problem::new(options), vec![vwce, aggh])
    }
}
