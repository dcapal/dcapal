mod optimize;
mod utils;

use optimize::{advanced, basic};

use rand::{distributions, Rng};
use rust_decimal::{
    prelude::{One, ToPrimitive},
    Decimal,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};
use utils::{parse_amount, parse_shares, parse_weight};
use wasm_bindgen::prelude::*;

#[macro_use]
extern crate lazy_static;

const AMOUNT_DECIMALS: u32 = 4;
const WEIGHT_DECIMALS: u32 = 6;
const SHARES_DECIMALS: u32 = 8;

lazy_static! {
    static ref BASIC_PROBLEMS: Mutex<HashMap<String, optimize::basic::Problem>> =
        Mutex::new(HashMap::new());
    static ref ADVANCED_PROBLEMS: Mutex<HashMap<String, optimize::advanced::Problem>> =
        Mutex::new(HashMap::new());
    static ref NUMERIC_DIST: distributions::Uniform<u8> =
        distributions::Uniform::new_inclusive(0, 9);
}

#[wasm_bindgen]
pub struct Solver {}

#[wasm_bindgen]
impl Solver {
    pub fn build_problem(options: JsValue) -> Result<ProblemHandle, JsValue> {
        utils::require_init();

        let options: JsProblemOptions =
            serde_wasm_bindgen::from_value(options).map_err(|e| e.to_string())?;

        let id = generate_problem_id();

        match options {
            JsProblemOptions::Advanced(options) => {
                let options = advanced::ProblemOptions::try_from(options)?;
                let problem = optimize::advanced::Problem::new(options);

                let mut problems = ADVANCED_PROBLEMS.lock().unwrap();
                problems.insert(id.clone(), problem);

                Ok(ProblemHandle {
                    id,
                    kind: ProblemKind::Advanced,
                })
            }
            JsProblemOptions::Basic(options) => {
                let options = basic::ProblemOptions::try_from(options)?;
                let problem = optimize::basic::Problem::new(options);

                BASIC_PROBLEMS.lock().unwrap().insert(id.clone(), problem);

                Ok(ProblemHandle {
                    id,
                    kind: ProblemKind::Basic,
                })
            }
        }
    }

    pub fn solve(handle: &ProblemHandle) -> Result<JsValue, JsValue> {
        utils::require_init();

        match handle.kind {
            ProblemKind::Advanced => Self::solve_advanced(&handle.id),
            ProblemKind::Basic => Self::solve_basic(&handle.id),
        }
    }

    fn solve_basic(id: &str) -> Result<JsValue, JsValue> {
        let problems = BASIC_PROBLEMS.lock().unwrap();
        let problem = problems
            .get(id)
            .ok_or_else(|| format!("Invalid problem id {}", id))?;

        let solution = problem.problem.solve().map_err(|e| e.to_string())?;
        let objective = solution.objective();
        let vars = problem
            .vars
            .iter()
            .map(|(aid, v)| (aid.clone(), solution[*v]))
            .collect();

        let amounts = if problem.options.is_buy_only {
            basic::refine_solution(problem, &vars)
        } else {
            vars
        };

        let js_solution = JsBasicSolution { objective, amounts };
        Ok(serde_wasm_bindgen::to_value(&js_solution).unwrap())
    }

    fn solve_advanced(id: &str) -> Result<JsValue, JsValue> {
        let problems = ADVANCED_PROBLEMS.lock().unwrap();
        let problem = problems
            .get(id)
            .ok_or_else(|| format!("Invalid problem id {}", id))?;

        let solution = problem.solve();

        if !solution.is_solved {
            let js_solution = JsAdvancedSolution::default();
            return Ok(serde_wasm_bindgen::to_value(&js_solution).unwrap());
        }

        let budget_left = solution.budget_left.to_f64().unwrap();

        let amounts = solution
            .assets
            .iter()
            .map(|(aid, v)| (aid.clone(), v.amount.to_f64().unwrap()))
            .collect();

        let shares = solution
            .assets
            .iter()
            .map(|(aid, v)| (aid.clone(), v.shares.to_f64().unwrap()))
            .collect();

        let js_solution = JsAdvancedSolution {
            budget_left,
            amounts,
            shares,
        };
        Ok(serde_wasm_bindgen::to_value(&js_solution).unwrap())
    }
}

#[wasm_bindgen]
pub enum ProblemKind {
    Advanced,
    Basic,
}

#[wasm_bindgen]
pub struct ProblemHandle {
    id: String,
    kind: ProblemKind,
}

#[wasm_bindgen]
impl ProblemHandle {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}

fn generate_problem_id() -> String {
    rand::thread_rng()
        .sample_iter(&*NUMERIC_DIST)
        .take(10)
        .map(|n| n.to_string())
        .collect()
}

#[derive(Serialize)]
pub struct JsBasicSolution {
    pub objective: f64,
    pub amounts: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JsAdvancedSolution {
    pub budget_left: f64,
    pub amounts: HashMap<String, f64>,
    pub shares: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsProblemOptions {
    Advanced(JsAdvancedOptions),
    Basic(JsBasicOptions),
}

#[derive(Serialize, Deserialize)]
pub struct JsAdvancedOptions {
    budget: f64,
    pfolio_ccy: String,
    assets: HashMap<String, JsAdvancedAsset>,
    is_buy_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct JsAdvancedAsset {
    symbol: String,
    shares: f64,
    price: f64,
    target_weight: f64,
    is_whole_shares: bool,
}

impl TryFrom<JsAdvancedOptions> for advanced::ProblemOptions {
    type Error = String;

    fn try_from(options: JsAdvancedOptions) -> Result<Self, Self::Error> {
        if options.budget < 0. {
            return Err(format!(
                "Invalid budget ({}). Must be positive",
                options.budget
            ));
        }

        let assets = options
            .assets
            .into_iter()
            .map(|(aid, a)| advanced::ProblemAsset::try_from(a).map(|a| (aid, a)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        let target_total = assets
            .values()
            .map(|a| a.target_weight)
            .sum::<Decimal>()
            .round_dp(AMOUNT_DECIMALS);

        if target_total != Decimal::one() {
            return Err(format!(
                "Invalid target weights. Sum must be equal to 1 ({} instead)",
                target_total
            ));
        }

        let budget = parse_amount(options.budget);
        let current_total = assets
            .values()
            .map(|a| a.price * a.shares)
            .sum::<Decimal>()
            .round_dp(AMOUNT_DECIMALS);

        Ok(advanced::ProblemOptions {
            pfolio_ccy: options.pfolio_ccy,
            current_pfolio_amount: current_total,
            assets,
            budget,
            is_buy_only: options.is_buy_only,
        })
    }
}

impl TryFrom<JsAdvancedAsset> for advanced::ProblemAsset {
    type Error = String;

    fn try_from(asset: JsAdvancedAsset) -> Result<Self, Self::Error> {
        let JsAdvancedAsset {
            symbol,
            shares,
            price,
            target_weight,
            is_whole_shares,
        } = asset;

        if symbol.is_empty() {
            return Err("Invalid symbol. Must not be empty".to_string());
        }

        if shares < 0. {
            return Err(format!(
                "Invalid shares ({}). Must be zero or positive",
                shares
            ));
        }

        if price < 0. {
            return Err(format!(
                "Invalid price ({}). Must be zero or positive",
                price
            ));
        }

        if target_weight < 0. {
            return Err(format!(
                "Invalid target weight ({}). Must be zero or positive",
                target_weight
            ));
        }

        if target_weight > 1. {
            return Err(format!(
                "Invalid target weight ({}). Must be less then or equal to 1.",
                target_weight
            ));
        }

        let shares = if is_whole_shares {
            shares.trunc()
        } else {
            shares
        };

        Ok(advanced::ProblemAsset {
            symbol,
            shares: parse_shares(shares),
            price: parse_amount(price),
            target_weight: parse_weight(target_weight),
            is_whole_shares,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsBasicOptions {
    budget: f64,
    assets: HashMap<String, JsProblemAsset>,
    is_buy_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct JsProblemAsset {
    symbol: String,
    target_weight: f64,
    current_amount: f64,
}

impl TryFrom<JsBasicOptions> for basic::ProblemOptions {
    type Error = String;

    fn try_from(options: JsBasicOptions) -> Result<Self, Self::Error> {
        if options.budget <= 0. {
            return Err(format!(
                "Invalid budget ({}). Must be positive",
                options.budget
            ));
        }

        let assets = options
            .assets
            .into_iter()
            .map(|(aid, a)| basic::ProblemAsset::try_from(a).map(|a| (aid, a)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        let target_total = assets
            .values()
            .map(|a| a.target_weight)
            .sum::<Decimal>()
            .round_dp(AMOUNT_DECIMALS);

        if target_total != Decimal::one() {
            return Err(format!(
                "Invalid target weights. Sum must be equal to 1 ({} instead)",
                target_total
            ));
        }

        let budget = parse_amount(options.budget);
        let current_total = assets
            .values()
            .map(|a| a.current_amount)
            .sum::<Decimal>()
            .round_dp(AMOUNT_DECIMALS);

        if current_total > budget {
            return Err(format!(
                "Invalid current amounts. Sum must be less than or equal to budget: {} ({} instead)",
                budget,
                current_total
            ));
        }

        Ok(basic::ProblemOptions {
            budget,
            assets,
            is_buy_only: options.is_buy_only,
        })
    }
}

impl TryFrom<JsProblemAsset> for basic::ProblemAsset {
    type Error = String;

    fn try_from(asset: JsProblemAsset) -> Result<Self, Self::Error> {
        let JsProblemAsset {
            symbol,
            target_weight,
            current_amount,
        } = asset;

        if symbol.is_empty() {
            return Err("Invalid symbol. Must not be empty".to_string());
        }

        if target_weight < 0. {
            return Err(format!(
                "Invalid target weight ({}). Must be zero or positive",
                target_weight
            ));
        }

        if target_weight > 1. {
            return Err(format!(
                "Invalid target weight ({}). Must be less then or equal to 1.",
                target_weight
            ));
        }

        if current_amount < 0. {
            return Err(format!(
                "Invalid current amount ({}). Must be zero or positive",
                current_amount
            ));
        }

        Ok(basic::ProblemAsset {
            symbol,
            target_weight: parse_weight(target_weight),
            current_amount: parse_amount(current_amount),
        })
    }
}
