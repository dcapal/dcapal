use std::cmp::Ordering;
use std::collections::HashMap;

use log::debug;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use crate::{AMOUNT_DECIMALS, PERCENTAGE_DECIMALS, SHARES_DECIMALS};

use super::TransactionFees;

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub pfolio_ccy: String,
    pub current_pfolio_amount: Decimal,
    pub assets: HashMap<String, ProblemAsset>,
    pub budget: Decimal,
    pub is_buy_only: bool,
    pub fees: TransactionFees,
}

#[derive(Debug, Clone)]
pub struct ProblemAsset {
    pub symbol: String,
    pub shares: Decimal,
    pub price: Decimal,
    pub target_weight: Decimal,
    pub is_whole_shares: bool,
    pub fees: Option<TransactionFees>,
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
    pub fees: Option<TransactionFees>,
    pub theo_alloc: Option<TheoreticalAllocation>,
    pub state: SolutionState,
}

#[derive(Debug, Clone)]
pub struct TheoreticalAllocation {
    pub shares: Decimal,
    pub amount: Decimal,
    pub fees: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolutionState {
    Open,
    Overallocated,
    FullyAllocated,
    PriceTooHigh,
    FeesTooHigh,
    TieBreaker,
}

impl Asset {
    pub fn new(asset: ProblemAsset, current_pfolio_amount: Decimal, budget: Decimal) -> Self {
        let ProblemAsset {
            symbol,
            shares,
            price,
            target_weight,
            is_whole_shares,
            fees,
        } = asset;

        let pfolio_amount = current_pfolio_amount + budget;

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
            fees,
            theo_alloc: None,
            state: SolutionState::Open,
        }
    }

    pub fn compute_fee(&self, amount: &Decimal, general_fees: &TransactionFees) -> Decimal {
        if let Some(ref fee) = self.fees {
            fee.fee_structure.compute_fee(amount)
        } else {
            general_fees.fee_structure.compute_fee(amount)
        }
    }

    pub fn get_max_fee_impact(&self, general_fees: &TransactionFees) -> Decimal {
        if let Some(ref fee) = self.fees {
            fee.max_fee_impact
        } else {
            general_fees.max_fee_impact
        }
    }

    pub fn get_allocated_amount(&self) -> Decimal {
        self.amount - self.current_amount
    }
}

pub struct Problem {
    pub(crate) options: ProblemOptions,
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub is_solved: bool,
    pub assets: HashMap<String, Asset>,
    pub budget_left: Decimal,
}

impl Solution {
    pub fn new(options: ProblemOptions) -> Self {
        let (pfolio_amount, budget) = (options.current_pfolio_amount, options.budget);

        let assets = options
            .assets
            .into_iter()
            .map(|(aid, asset)| (aid, Asset::new(asset, pfolio_amount, budget)))
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

    pub fn solve(&self) -> Solution {
        let mut solution = Solution::new(self.options.clone());

        // New portfolio amount
        let pfolio_amount = self.options.current_pfolio_amount + self.options.budget;

        let sold_amount = if self.options.is_buy_only {
            close_over_allocated_assets(&mut solution.assets);
            Decimal::ZERO
        } else {
            sell_over_allocated_assets(&mut solution, pfolio_amount)
        };

        let pfolio_fees = &self.options.fees;

        // Budget available to allocate
        let mut budget_left = self.options.budget + sold_amount;

        debug!("[Init] solution={solution:?} pfolio_amount={pfolio_amount} sold_amount={sold_amount} budget_left={budget_left}");

        // Get under allocated assets
        let mut open_assets = under_allocated_view(&mut solution.assets);

        // Rescale target weights for open assets only
        let mut w_sum: Decimal = open_assets.iter().map(|a| a.target_weight).sum();
        let mut adjusted_weights = open_assets
            .iter()
            .map(|a| (a.target_weight / w_sum).round_dp(PERCENTAGE_DECIMALS))
            .collect::<Vec<Decimal>>();

        let mut step = 0;
        while !open_assets.is_empty() {
            debug!(
                "[Step {}] budget_left={} open_assets={:?} w_sum={} adjusted_weights={:?}",
                step, budget_left, open_assets, w_sum, adjusted_weights
            );

            let mut budget_left_next = budget_left;
            let mut is_all_unallocated = true;

            if !is_negligible(&budget_left) {
                // Allocate budget depending on target weight
                for (i, asset) in open_assets.iter_mut().enumerate() {
                    let w_i = adjusted_weights[i];
                    let allocated = amount_to_allocate(asset, budget_left, w_i);
                    let allocated_shares = shares_to_allocate(asset, allocated);

                    if allocated_shares > Decimal::ZERO {
                        is_all_unallocated = false;
                    }

                    // Update allocated amount depending on shares allocated
                    let allocated = (allocated_shares * asset.price).round_dp(AMOUNT_DECIMALS);

                    // Update solution values
                    asset.amount += allocated;
                    asset.shares += allocated_shares;
                    asset.weight = (asset.amount / pfolio_amount).round_dp(PERCENTAGE_DECIMALS);
                    budget_left_next -= allocated;
                }
            }

            // Check asset constraints
            let mut freed_budget =
                check_fully_allocated_assets(&mut open_assets, &budget_left, pfolio_fees);

            // If previous step freed some budget, do another round of allocation unconditionally.
            // Otherwise, try to break ties or exit
            if freed_budget == Decimal::ZERO {
                // Unblock ties if necessary
                if !is_negligible(&budget_left) && is_all_unallocated {
                    freed_budget += unblock_ties(&mut open_assets, pfolio_fees);
                }

                // Check remaining budget
                let was_negligible = is_negligible(&budget_left);
                budget_left = budget_left_next + freed_budget;
                if was_negligible && is_negligible(&budget_left) {
                    // Check fee impact for open assets
                    freed_budget = Decimal::ZERO;
                    for asset in &mut open_assets {
                        if is_fee_impact_too_high(asset, pfolio_fees) {
                            freed_budget += deallocate_asset_fee_too_high(asset, pfolio_fees);
                        }
                    }

                    budget_left += freed_budget;
                    if is_negligible(&budget_left) {
                        for asset in &mut open_assets {
                            asset.state = SolutionState::FullyAllocated;
                        }
                        break;
                    }
                }
            } else {
                budget_left = budget_left_next + freed_budget
            }

            // Refresh open_assets
            if freed_budget > Decimal::ZERO {
                debug!(
                    "[Step {step}] Before refresh: budget_left={budget_left} freed_budget={freed_budget} count={} open_assets={open_assets:?}",
                    open_assets.len()
                );
                open_assets = refresh_open_assets(&mut solution.assets, &budget_left);
                debug!(
                    "[Step {step}] After refresh: budget_left={budget_left} freed_budget={freed_budget} count={} open_assets={open_assets:?}",
                    open_assets.len()
                );
            }

            // Update open_assets rescaled weights
            w_sum = open_assets.iter().map(|a| a.target_weight).sum();
            adjusted_weights = open_assets
                .iter()
                .map(|a| (a.target_weight / w_sum).round_dp(PERCENTAGE_DECIMALS))
                .collect::<Vec<Decimal>>();

            step += 1;
        }

        debug!(
            "[Solution] budget_left={} open_assets={:?} w_sum={} adjusted_weights={:?}",
            budget_left, open_assets, w_sum, adjusted_weights
        );

        // Reconcile solution weights
        for asset in solution.assets.values_mut() {
            asset.weight = (asset.amount / pfolio_amount).round_dp(PERCENTAGE_DECIMALS);
        }

        solution.is_solved = true;
        solution.budget_left = budget_left;

        debug!("[Solution] solution={solution:?}");

        solution
    }

    pub fn suggest_invest_amount(&self) -> Decimal {
        let mut solution = Solution::new(self.options.clone());

        let assets = solution.assets.values_mut().collect::<Vec<_>>();

        //find max current weight in the solution's assets
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

fn is_negligible(budget: &Decimal) -> bool {
    *budget < dec!(0.01)
}

fn refresh_open_assets<'a>(
    assets: &'a mut HashMap<String, Asset>,
    budget_left: &Decimal,
) -> Vec<&'a mut Asset> {
    assets
        .values_mut()
        .filter(|a| {
            a.state == SolutionState::Open
                || a.state == SolutionState::TieBreaker
                || (a.state == SolutionState::PriceTooHigh
                    && (!a.is_whole_shares || a.price <= *budget_left))
        })
        .map(|a| {
            a.state = SolutionState::Open;
            a
        })
        .collect::<_>()
}

fn close_over_allocated_assets(assets: &mut HashMap<String, Asset>) {
    for asset in assets.values_mut() {
        if asset.current_amount > asset.target_amount {
            asset.state = SolutionState::Overallocated;
        }
    }
}

fn sell_over_allocated_assets(solution: &mut Solution, pfolio_amount: Decimal) -> Decimal {
    let mut sold_amount = Decimal::ZERO;
    for asset in solution.assets.values_mut() {
        if asset.current_amount <= asset.target_amount {
            continue;
        }

        asset.state = SolutionState::Overallocated;

        let overallocated = asset.current_amount - asset.target_amount;
        let sell_shares = shares_to_allocate(asset, overallocated);
        if sell_shares.is_zero() {
            debug!("[Rebalance] Cannot sell over allocated asset: {:?} (overallocated={overallocated}, sell_shares={sell_shares}", asset);
            continue; // If cannot sell a single share, do nothing -- Better slightly overbalanced
        }

        let sell_amount = (sell_shares * asset.price).round_dp(AMOUNT_DECIMALS);

        // Update solution values
        asset.amount -= sell_amount;
        asset.shares -= sell_shares;
        asset.weight = (asset.amount / pfolio_amount).round_dp(PERCENTAGE_DECIMALS);
        sold_amount += sell_amount;
    }

    sold_amount.round_dp(AMOUNT_DECIMALS)
}

/// Get a view over under allocated assets i.e. assets with `current_weight` less than `target_weight`
fn under_allocated_view(assets: &mut HashMap<String, Asset>) -> Vec<&mut Asset> {
    assets
        .values_mut()
        .filter_map(|a| (a.state == SolutionState::Open).then_some(a))
        .collect::<Vec<&mut Asset>>()
}

/// Get amount to allocate
fn amount_to_allocate(asset: &Asset, budget: Decimal, w_i: Decimal) -> Decimal {
    Decimal::min(w_i * budget, asset.target_amount - asset.amount)
}

/// Get number of shares to allocate
fn shares_to_allocate(asset: &Asset, allocated_amount: Decimal) -> Decimal {
    let shares = allocated_amount / asset.price;

    if asset.is_whole_shares {
        shares.trunc()
    } else {
        shares.round_dp(SHARES_DECIMALS)
    }
}

/// Remove fully allocated assets. Returns budget freed from assets with too high fee impact.
fn check_fully_allocated_assets(
    open_assets: &mut Vec<&mut Asset>,
    budget_left: &Decimal,
    fees: &TransactionFees,
) -> Decimal {
    let mut freed_budget = Decimal::ZERO;

    // Remove fully-allocated assets
    let mut i = 0;
    while i < open_assets.len() {
        let asset = &mut open_assets[i];

        let to_remove = || -> bool {
            // Fully allocated
            if asset.amount >= asset.target_amount {
                asset.state = SolutionState::FullyAllocated;
                return true;
            }

            // Cannot allocate more -- Budget left higher than asset price
            if asset.is_whole_shares && asset.price > *budget_left {
                asset.state = SolutionState::PriceTooHigh;
                return true;
            }

            false
        }();

        if to_remove {
            if is_fee_impact_too_high(asset, fees) {
                freed_budget += deallocate_asset_fee_too_high(asset, fees);
            }
            open_assets.remove(i);
        } else {
            i += 1;
        }
    }

    freed_budget
}

fn unblock_ties(open_assets: &mut Vec<&mut Asset>, fees: &TransactionFees) -> Decimal {
    let mut freed_budget = Decimal::ZERO;
    let mut not_enough_budget_idx = None;
    let mut min_target_distance = Decimal::MAX;

    // Otherwise, just find the asset closest to target allocation to remove
    for (i, asset) in open_assets.iter().enumerate() {
        let target_distance = asset.target_amount - asset.amount;
        if target_distance < min_target_distance {
            // Cannot allocate more to this asset -- not enough budget
            min_target_distance = target_distance;
            not_enough_budget_idx = Some(i);
        }
    }

    if let Some(idx) = not_enough_budget_idx {
        let asset = &mut open_assets[idx];
        asset.state = SolutionState::TieBreaker;

        if is_fee_impact_too_high(asset, fees) {
            freed_budget = deallocate_asset_fee_too_high(asset, fees);
        }

        open_assets.remove(idx);
    }

    freed_budget
}

fn is_fee_impact_too_high(asset: &Asset, general_fees: &TransactionFees) -> bool {
    let allocated = asset.get_allocated_amount();
    if allocated <= Decimal::ZERO {
        return false;
    }

    let fees = asset.compute_fee(&allocated, general_fees);
    let impact = fees / allocated;

    debug!(
        "allocated={allocated} fees={fees} impact={impact} max_fee_impact={}",
        asset.get_max_fee_impact(general_fees)
    );

    impact > asset.get_max_fee_impact(general_fees)
}

fn deallocate_asset_fee_too_high(asset: &mut Asset, general_fees: &TransactionFees) -> Decimal {
    let allocated = asset.get_allocated_amount();

    asset.theo_alloc.replace(TheoreticalAllocation {
        shares: asset.shares,
        amount: asset.amount,
        fees: asset.compute_fee(&allocated, general_fees),
    });

    asset.amount = asset.current_amount;
    asset.shares = asset.current_shares;
    asset.weight = asset.current_weight;

    asset.state = SolutionState::FeesTooHigh;

    allocated
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test_log::test]
    fn it_solves_60_40_portfolio_buy_only() {
        // Given
        let (problem, assets) = build_60_40_portfolio_no_allocation(true);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.solve();

        // Expect
        assert!(solution.is_solved);
        {
            let sol = &solution.assets[&vwce];
            assert_eq!(sol.shares, dec!(11));
            assert_eq!(sol.amount, dec!(58.3));
            assert_eq!(sol.weight, dec!(0.583));
        }
        {
            let sol = &solution.assets[&aggh];
            assert_eq!(sol.shares, dec!(30));
            assert_eq!(sol.amount, dec!(39));
            assert_eq!(sol.weight, dec!(0.39));
        }
        assert_eq!(solution.budget_left, dec!(2.7));
    }

    #[test_log::test]
    fn it_solves_60_40_portfolio_buy_and_sell() {
        // Given
        let (problem, assets) = build_60_40_portfolio_no_allocation(false);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.solve();

        // Expect
        assert!(solution.is_solved);
        {
            let sol = &solution.assets[&vwce];
            assert_eq!(sol.shares, dec!(11));
            assert_eq!(sol.amount, dec!(58.3));
            assert_eq!(sol.weight, dec!(0.583));
        }
        {
            let sol = &solution.assets[&aggh];
            assert_eq!(sol.shares, dec!(30));
            assert_eq!(sol.amount, dec!(39));
            assert_eq!(sol.weight, dec!(0.39));
        }
        assert_eq!(solution.budget_left, dec!(2.7));
    }

    #[test_log::test]
    fn it_solves_60_40_unbalanced_portfolio_buy_only() {
        // Given
        let (problem, assets) = build_60_40_portfolio_unbalanced(true, false);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.solve();

        // Expect
        assert!(solution.is_solved);
        {
            let sol = &solution.assets[&vwce];
            assert_eq!(sol.shares, dec!(6));
            assert_eq!(sol.amount, dec!(63.6));
            assert_eq!(sol.weight, dec!(0.636));
        }
        {
            let sol = &solution.assets[&aggh];
            assert_eq!(sol.shares, dec!(28));
            assert_eq!(sol.amount, dec!(36.4));
            assert_eq!(sol.weight, dec!(0.364));
        }
        assert_eq!(solution.budget_left, Decimal::ZERO);
    }

    #[test_log::test]
    fn it_solves_60_40_unbalanced_portfolio_buy_and_sell_price_too_high_to_sell() {
        // Given
        let (problem, assets) = build_60_40_portfolio_unbalanced(false, false);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.solve();

        // Expect
        assert!(solution.is_solved);
        {
            let sol = &solution.assets[&vwce];
            assert_eq!(sol.shares, dec!(6));
            assert_eq!(sol.amount, dec!(63.6));
            assert_eq!(sol.weight, dec!(0.636));
        }
        {
            let sol = &solution.assets[&aggh];
            assert_eq!(sol.shares, dec!(28));
            assert_eq!(sol.amount, dec!(36.4));
            assert_eq!(sol.weight, dec!(0.364));
        }
        assert_eq!(solution.budget_left, Decimal::ZERO);
    }

    #[test_log::test]
    fn it_solves_60_40_unbalanced_portfolio_buy_and_sell() {
        // Given
        let (problem, assets) = build_60_40_portfolio_unbalanced(false, true);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.solve();

        // Expect
        assert!(solution.is_solved);
        {
            let sol = &solution.assets[&vwce];
            assert_eq!(sol.shares, dec!(57));
            assert_eq!(sol.amount, dec!(60.42));
            assert_eq!(sol.weight, dec!(0.6042));
        }
        {
            let sol = &solution.assets[&aggh];
            assert_eq!(sol.shares, dec!(30.));
            assert_eq!(sol.amount, dec!(39.));
            assert_eq!(sol.weight, dec!(0.39));
        }
        assert_eq!(solution.budget_left, dec!(0.58));
    }

    fn build_60_40_portfolio_no_allocation(is_buy_only: bool) -> (Problem, Vec<String>) {
        let budget = dec!(100.);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    shares: dec!(0.),
                    price: dec!(5.3),
                    target_weight: dec!(0.6),
                    is_whole_shares: true,
                    fees: None,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    shares: dec!(0.),
                    price: dec!(1.3),
                    target_weight: dec!(0.4),
                    is_whole_shares: true,
                    fees: None,
                },
            ),
        ]);

        let current_pfolio_amount = assets
            .values()
            .map(|a| (a.shares * a.price).round_dp(AMOUNT_DECIMALS))
            .sum();

        let options = ProblemOptions {
            pfolio_ccy: "eur".into(),
            current_pfolio_amount,
            assets,
            budget,
            is_buy_only,
            fees: TransactionFees::default(),
        };

        (Problem::new(options), vec![vwce, aggh])
    }

    fn build_60_40_portfolio_unbalanced(
        is_buy_only: bool,
        is_low_price: bool,
    ) -> (Problem, Vec<String>) {
        let budget = dec!(10.4);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();

        let (vwce_shares, vwce_price) = if is_low_price {
            (dec!(60), dec!(1.06))
        } else {
            (dec!(6), dec!(10.6))
        };

        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    shares: vwce_shares,
                    price: vwce_price,
                    target_weight: dec!(0.6),
                    is_whole_shares: true,
                    fees: None,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    shares: dec!(20),
                    price: dec!(1.3),
                    target_weight: dec!(0.4),
                    is_whole_shares: true,
                    fees: None,
                },
            ),
        ]);

        let current_pfolio_amount = assets
            .values()
            .map(|a| (a.shares * a.price).round_dp(AMOUNT_DECIMALS))
            .sum();

        let options = ProblemOptions {
            pfolio_ccy: "eur".into(),
            current_pfolio_amount,
            assets,
            budget,
            is_buy_only,
            fees: TransactionFees::default(),
        };

        (Problem::new(options), vec![vwce, aggh])
    }
}
