use std::collections::HashMap;

use log::debug;
use minilp::{ComparisonOp, OptimizationDirection, Variable};
use rust_decimal::prelude::*;

use crate::{AMOUNT_DECIMALS, WEIGHT_DECIMALS};

use super::ProblemOptions;

pub struct Problem {
    pub(crate) options: ProblemOptions,
    pub(crate) problem: minilp::Problem,
    pub(crate) vars: HashMap<String, Variable>,
}

impl Problem {
    pub fn new(options: ProblemOptions) -> Self {
        // Problem:
        //    minimize sum_i(s_i)
        let mut problem = minilp::Problem::new(OptimizationDirection::Minimize);

        // Variables:
        //    a_i - invested amount for asset i
        //    s_i - slack between solution weight and target weight for asset i
        let mut vars = HashMap::new();
        let budget = options.budget.to_f64().unwrap();
        let budget_inv = 1. / budget;
        for (aid, asset) in &options.assets {
            let a_i = problem.add_var(0., (0., budget));
            let s_i_neg = problem.add_var(1., (0., f64::INFINITY));
            let s_i_pos = problem.add_var(1., (0., f64::INFINITY));

            // s_i_neg >= target_weight - a_i / budget
            // =>
            // a_i / budget + s_i_neg >= target_weight
            problem.add_constraint(
                [(a_i, budget_inv), (s_i_neg, 1.)],
                ComparisonOp::Ge,
                asset.target_weight.to_f64().unwrap(),
            );

            // s_i_pos >= a_i / budget - target_weight
            // =>
            // a_i / budget - s_i_pos <= target_weight
            problem.add_constraint(
                [(a_i, budget_inv), (s_i_pos, -1.)],
                ComparisonOp::Le,
                asset.target_weight.to_f64().unwrap(),
            );

            vars.insert(aid.clone(), a_i);
        }

        // Subject to:
        //    sum_i(a_i) = budget   -- Invest all budget
        problem.add_constraint(
            vars.values().copied().map(|a_i| (a_i, 1.0)),
            ComparisonOp::Eq,
            budget,
        );

        if options.is_buy_only {
            // Subject to:
            //    a_i >= current_amount   -- No sell
            for (aid, a_i) in &vars {
                problem.add_constraint(
                    [(*a_i, 1.0)],
                    ComparisonOp::Ge,
                    options
                        .assets
                        .get(aid)
                        .unwrap()
                        .current_amount
                        .to_f64()
                        .unwrap(),
                );
            }
        }

        Problem {
            options,
            problem,
            vars,
        }
    }
}

pub fn refine_solution(problem: &Problem, vars: &HashMap<String, f64>) -> HashMap<String, f64> {
    let options = &problem.options;

    let budget = options.budget;
    let mut assets = parse_assets(options, vars);

    debug!("assets = {:?}", assets);

    {
        let mut under_allocated = assets
            .values_mut()
            .filter_map(|a| (a.solution_weight <= a.target_weight).then_some(a))
            .collect::<Vec<&mut Asset>>();

        debug!("under_allocated = {:?}", under_allocated);

        if under_allocated.is_empty() {
            return vars.clone();
        }

        let mut leftover: Decimal = under_allocated
            .iter()
            .map(|a| a.solution_amount - a.current_amount)
            .sum();

        debug!("leftover = {:?}", leftover);

        let mut w_sum: Decimal = under_allocated.iter().map(|a| a.target_weight).sum();
        let mut adjusted_weights = under_allocated
            .iter()
            .map(|a| (a.target_weight / w_sum).round_dp(WEIGHT_DECIMALS))
            .collect::<Vec<Decimal>>();

        debug!(
            "w_sum = {:?} adjusted_weights = {:?}",
            leftover, adjusted_weights
        );

        while leftover > Decimal::zero() {
            let mut leftover_next = Decimal::zero();
            for (i, asset) in under_allocated.iter_mut().enumerate() {
                let w_i = adjusted_weights[i];
                let allocated_amount =
                    Decimal::min(w_i * leftover, asset.target_amount - asset.current_amount)
                        .round_dp(AMOUNT_DECIMALS);

                asset.current_amount += allocated_amount;
                asset.current_weight = (asset.current_amount / budget).round_dp(WEIGHT_DECIMALS);
                leftover_next += w_i * leftover - allocated_amount;
            }

            leftover = leftover_next.round_dp(AMOUNT_DECIMALS);

            debug!("under_allocated = {:?}", under_allocated);
            debug!("leftover = {:?}", leftover_next);

            // Remove fully allocated assets
            let mut i = 0;
            while i < under_allocated.len() {
                if under_allocated[i].current_weight >= under_allocated[i].target_weight {
                    under_allocated.remove(i);
                } else {
                    i += 1;
                }
            }

            w_sum = under_allocated.iter().map(|a| a.target_weight).sum();
            adjusted_weights = under_allocated
                .iter()
                .map(|a| (a.target_weight / w_sum).round_dp(WEIGHT_DECIMALS))
                .collect::<Vec<Decimal>>();

            debug!(
                "w_sum = {:?} adjusted_weights = {:?}",
                leftover, adjusted_weights
            );
        }
    }

    debug!("assets = {:?}", assets);

    assets
        .into_iter()
        .map(|(aid, asset)| (aid, asset.current_amount.to_f64().unwrap()))
        .collect()
}

#[derive(Debug)]
struct Asset {
    current_amount: Decimal,
    solution_amount: Decimal,
    target_amount: Decimal,
    current_weight: Decimal,
    solution_weight: Decimal,
    target_weight: Decimal,
}

impl Asset {
    fn new(
        current_amount: Decimal,
        solution_amount: Decimal,
        target_amount: Decimal,
        current_weight: Decimal,
        solution_weight: Decimal,
        target_weight: Decimal,
    ) -> Self {
        Self {
            current_amount,
            solution_amount,
            target_amount,
            current_weight,
            solution_weight,
            target_weight,
        }
    }
}

fn parse_assets(options: &ProblemOptions, vars: &HashMap<String, f64>) -> HashMap<String, Asset> {
    let budget = &options.budget;

    vars.iter()
        .map(|(aid, solution_amount)| {
            let asset = options.assets.get(aid).unwrap();
            let current_amount = asset.current_amount;
            let target_weight = asset.target_weight;
            let target_amount = (target_weight * budget).round_dp(AMOUNT_DECIMALS);
            let current_weight = (current_amount / budget).round_dp(WEIGHT_DECIMALS);
            let solution_amount = parse_amount(*solution_amount);
            let solution_weight = (solution_amount / budget).round_dp(WEIGHT_DECIMALS);

            (
                aid.clone(),
                Asset::new(
                    current_amount,
                    solution_amount,
                    target_amount,
                    current_weight,
                    solution_weight,
                    target_weight,
                ),
            )
        })
        .collect()
}

fn parse_amount(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap().round_dp(AMOUNT_DECIMALS)
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::optimize::ProblemAsset;

    use super::*;

    fn build_60_40_portfolio_no_allocation(is_buy_only: bool) -> (Problem, Vec<String>) {
        let budget = dec!(100.);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: dec!(0.),
                    target_weight: dec!(0.6),
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: dec!(0.),
                    target_weight: dec!(0.4),
                },
            ),
        ]);

        let options = ProblemOptions {
            budget,
            assets,
            is_buy_only,
        };

        (Problem::new(options), vec![vwce, aggh])
    }

    #[test]
    fn it_solves_60_40_portfolio_buy_only() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_60_40_portfolio_no_allocation(true);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 60.);
        assert_eq!(solution[problem.vars[&aggh]], 40.);
        Ok(())
    }

    #[test]
    fn it_solves_60_40_portfolio_buy_and_sell() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_60_40_portfolio_no_allocation(false);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 60.);
        assert_eq!(solution[problem.vars[&aggh]], 40.);
        Ok(())
    }

    fn build_60_40_portfolio_unbalanced(is_buy_only: bool) -> (Problem, Vec<String>) {
        let budget = dec!(100.);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: dec!(65.),
                    target_weight: dec!(0.6),
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: dec!(25.),
                    target_weight: dec!(0.4),
                },
            ),
        ]);
        let options = ProblemOptions {
            budget,
            assets,
            is_buy_only,
        };

        (Problem::new(options), vec![vwce, aggh])
    }

    #[test]
    fn it_solves_60_40_unbalanced_portfolio_buy_only() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_60_40_portfolio_unbalanced(true);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 65.);
        assert_eq!(solution[problem.vars[&aggh]], 35.);
        Ok(())
    }

    #[test]
    fn it_solves_60_40_unbalanced_portfolio_buy_and_sell() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_60_40_portfolio_unbalanced(false);
        let [vwce, aggh] = <[String; 2]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 60.);
        assert_eq!(solution[problem.vars[&aggh]], 40.);
        Ok(())
    }

    fn build_three_assets_portfolio(is_buy_only: bool) -> (Problem, Vec<String>) {
        let budget = dec!(7706.12);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let reit = "EPRU".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: dec!(5420.10),
                    target_weight: dec!(0.8),
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: dec!(680.93),
                    target_weight: dec!(0.1),
                },
            ),
            (
                reit.clone(),
                ProblemAsset {
                    symbol: reit.clone(),
                    current_amount: dec!(605.48),
                    target_weight: dec!(0.1),
                },
            ),
        ]);
        let options = ProblemOptions {
            budget,
            assets,
            is_buy_only,
        };

        (Problem::new(options), vec![vwce, aggh, reit])
    }

    #[test]
    fn it_solves_three_assets_portfolio_buy_only() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_three_assets_portfolio(true);
        let [vwce, aggh, reit] = <[String; 3]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        println!("{}: {}", &vwce, solution[problem.vars[&vwce]]);
        println!("{}: {}", &aggh, solution[problem.vars[&aggh]]);
        println!("{}: {}", &reit, solution[problem.vars[&reit]]);
        Ok(())
    }

    #[test]
    fn it_solves_three_assets_portfolio_buy_and_sell() -> anyhow::Result<()> {
        // Given
        let (problem, assets) = build_three_assets_portfolio(false);
        let [vwce, aggh, reit] = <[String; 3]>::try_from(assets).ok().unwrap();

        // When
        let solution = problem.problem.solve()?;

        // Expect
        println!("{}: {}", &vwce, solution[problem.vars[&vwce]]);
        println!("{}: {}", &aggh, solution[problem.vars[&aggh]]);
        println!("{}: {}", &reit, solution[problem.vars[&reit]]);
        Ok(())
    }

    #[test]
    fn it_solves_my_problem() -> anyhow::Result<()> {
        let budget = dec!(17_069.12);
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let epra = "EPRA".to_string();
        let btc = "BTC".to_string();
        let eth = "ETH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: dec!(10_193.68),
                    target_weight: dec!(0.7),
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: dec!(2_646.31),
                    target_weight: dec!(0.2),
                },
            ),
            (
                epra.clone(),
                ProblemAsset {
                    symbol: epra.clone(),
                    current_amount: dec!(775.75),
                    target_weight: dec!(0.02),
                },
            ),
            (
                btc.clone(),
                ProblemAsset {
                    symbol: btc.clone(),
                    current_amount: dec!(920.55),
                    target_weight: dec!(0.055),
                },
            ),
            (
                eth.clone(),
                ProblemAsset {
                    symbol: eth.clone(),
                    current_amount: dec!(532.83),
                    target_weight: dec!(0.025),
                },
            ),
        ]);
        let options = ProblemOptions {
            budget,
            assets,
            is_buy_only: true,
        };
        let problem = Problem::new(options);

        // When
        let solution = problem.problem.solve()?;
        let vars = problem
            .vars
            .iter()
            .map(|(aid, v)| (aid.clone(), solution[*v]))
            .collect();

        let vars = refine_solution(&problem, &vars);

        // Expect
        println!("{}: {}", &vwce, vars[&vwce]);
        println!("{}: {}", &aggh, vars[&aggh]);
        println!("{}: {}", &epra, vars[&epra]);
        println!("{}: {}", &btc, vars[&btc]);
        println!("{}: {}", &eth, vars[&eth]);
        Ok(())
    }
}
