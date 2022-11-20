use std::collections::HashMap;

use minilp::{ComparisonOp, OptimizationDirection, Variable};

#[derive(Debug, Clone)]
pub struct ProblemOptions {
    pub budget: f64,
    pub assets: HashMap<String, ProblemAsset>,
}

#[derive(Debug, Clone)]
pub struct ProblemAsset {
    pub symbol: String,
    pub target_weight: f64,
    pub current_amount: f64,
}

pub struct Problem {
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
        let budget_inv = 1. / options.budget;
        for (aid, asset) in &options.assets {
            let a_i = problem.add_var(0., (0., options.budget));
            let s_i_pos = problem.add_var(1., (0., f64::INFINITY));
            let s_i_neg = problem.add_var(1., (0., f64::INFINITY));

            // s_i_pos >= a_i / budget - target_weight
            // =>
            // a_i / budget - s_i_pos <= target_weight
            problem.add_constraint(
                &[(a_i, budget_inv), (s_i_pos, -1.)],
                ComparisonOp::Le,
                asset.target_weight,
            );
            // s_i_neg >= target_weight - a_i / budget
            // =>
            // a_i / budget + s_i_neg >= target_weight
            problem.add_constraint(
                &[(a_i, budget_inv), (s_i_neg, 1.)],
                ComparisonOp::Ge,
                asset.target_weight,
            );

            vars.insert(aid.clone(), a_i);
        }

        // Subject to:
        //    sum_i(a_i) = budget   -- Invest all budget
        problem.add_constraint(
            vars.values().copied().map(|a_i| (a_i, 1.0)),
            ComparisonOp::Eq,
            options.budget,
        );
        //    a_i >= current_amount   -- No sell
        for (aid, a_i) in &vars {
            problem.add_constraint(
                &[(*a_i, 1.0)],
                ComparisonOp::Ge,
                options.assets.get(aid).unwrap().current_amount,
            );
        }

        Problem { problem, vars }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_solves_60_40_portfolio() -> anyhow::Result<()> {
        // Given
        let budget = 100.;
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: 0.,
                    target_weight: 0.6,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: 0.,
                    target_weight: 0.4,
                },
            ),
        ]);
        let options = ProblemOptions { budget, assets };

        // When
        let problem = Problem::new(options);
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 60.);
        assert_eq!(solution[problem.vars[&aggh]], 40.);
        Ok(())
    }

    #[test]
    fn it_solves_60_40_unbalanced_portfolio() -> anyhow::Result<()> {
        // Given
        let budget = 100.;
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: 65.,
                    target_weight: 0.6,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: 25.,
                    target_weight: 0.4,
                },
            ),
        ]);
        let options = ProblemOptions { budget, assets };

        // When
        let problem = Problem::new(options);
        let solution = problem.problem.solve()?;

        // Expect
        assert_eq!(solution[problem.vars[&vwce]], 65.);
        assert_eq!(solution[problem.vars[&aggh]], 35.);
        Ok(())
    }

    #[test]
    fn it_solves_my_portfolio() -> anyhow::Result<()> {
        // Given
        let budget = 7706.12;
        let vwce = "VWCE".to_string();
        let aggh = "AGGH".to_string();
        let reit = "EPRU".to_string();
        let assets = HashMap::from([
            (
                vwce.clone(),
                ProblemAsset {
                    symbol: vwce.clone(),
                    current_amount: 5420.10,
                    target_weight: 0.8,
                },
            ),
            (
                aggh.clone(),
                ProblemAsset {
                    symbol: aggh.clone(),
                    current_amount: 680.93,
                    target_weight: 0.1,
                },
            ),
            (
                reit.clone(),
                ProblemAsset {
                    symbol: reit.clone(),
                    current_amount: 605.48,
                    target_weight: 0.1,
                },
            ),
        ]);
        let options = ProblemOptions { budget, assets };

        // When
        let problem = Problem::new(options);
        let solution = problem.problem.solve()?;

        // Expect
        println!("{}: {}", &vwce, solution[problem.vars[&vwce]]);
        println!("{}: {}", &aggh, solution[problem.vars[&aggh]]);
        println!("{}: {}", &reit, solution[problem.vars[&reit]]);
        Ok(())
    }
}
