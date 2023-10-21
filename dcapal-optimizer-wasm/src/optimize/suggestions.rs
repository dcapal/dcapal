extern crate nalgebra as na;

use std::cmp::Ordering;

use rust_decimal::Decimal;

use crate::optimize::advanced::{Asset, ProblemOptions, Solution};

pub struct Problem {
    pub(crate) options: ProblemOptions,
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
