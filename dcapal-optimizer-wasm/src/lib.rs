mod optimize;
mod utils;

use std::{collections::HashMap, sync::Mutex};

use optimize::{ProblemAsset, ProblemOptions};
use rand::{distributions, Rng};
use serde::{Deserialize, Serialize};
use utils::Round;
use wasm_bindgen::prelude::*;

use crate::optimize::Problem;

#[macro_use]
extern crate lazy_static;

const AMOUNT_DECIMALS: i32 = 4;

lazy_static! {
    static ref PROBLEMS: Mutex<HashMap<String, Problem>> = Mutex::new(HashMap::new());
    static ref NUMERIC_DIST: distributions::Uniform<u8> =
        distributions::Uniform::new_inclusive(0, 9);
}

#[wasm_bindgen]
pub struct Solver {}

#[wasm_bindgen]
impl Solver {
    pub fn build_problem(options: JsValue) -> Result<ProblemHandle, JsValue> {
        utils::set_panic_hook();

        let options: JsProblemOptions =
            serde_wasm_bindgen::from_value(options).map_err(|e| e.to_string())?;
        let options = ProblemOptions::try_from(options)?;
        let problem = optimize::Problem::new(options);
        let id = generate_problem_id();

        PROBLEMS.lock().unwrap().insert(id.clone(), problem);
        Ok(ProblemHandle { id })
    }

    pub fn solve(handle: &ProblemHandle) -> Result<JsValue, JsValue> {
        utils::set_panic_hook();

        let problems = PROBLEMS.lock().unwrap();
        let problem = problems
            .get(&handle.id)
            .ok_or_else(|| format!("Invalid problem id {}", &handle.id))?;

        let solution = problem.problem.solve().map_err(|e| e.to_string())?;
        let objective = solution.objective();
        let vars = problem
            .vars
            .iter()
            .map(|(aid, v)| (aid.clone(), solution[*v]))
            .collect();

        let js_solution = JsSolution { objective, vars };
        Ok(serde_wasm_bindgen::to_value(&js_solution).unwrap())
    }
}

#[wasm_bindgen]
pub struct ProblemHandle {
    id: String,
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
pub struct JsSolution {
    pub objective: f64,
    pub vars: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize)]
pub struct JsProblemOptions {
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

impl TryFrom<JsProblemOptions> for ProblemOptions {
    type Error = String;

    fn try_from(options: JsProblemOptions) -> Result<Self, Self::Error> {
        if options.budget <= 0. {
            return Err(format!(
                "Invalid budget ({}). Must be positive",
                options.budget
            ));
        }

        let assets = options
            .assets
            .into_iter()
            .map(|(aid, a)| ProblemAsset::try_from(a).map(|a| (aid, a)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        let target_total: f64 = assets
            .values()
            .map(|a| a.target_weight)
            .sum::<f64>()
            .round_n(AMOUNT_DECIMALS);
        if target_total != 1. {
            return Err(format!(
                "Invalid target weights. Sum must be equal to 1 ({} instead)",
                target_total
            ));
        }

        let budget = options.budget.round_n(AMOUNT_DECIMALS);
        let current_total = assets
            .values()
            .map(|a| a.current_amount)
            .sum::<f64>()
            .round_n(AMOUNT_DECIMALS);

        if current_total > budget {
            return Err(format!(
                "Invalid current amounts. Sum must be less than or equal to budget: {} ({} instead)",
                budget,
                current_total
            ));
        }

        Ok(ProblemOptions {
            budget,
            assets,
            is_buy_only: options.is_buy_only,
        })
    }
}

impl TryFrom<JsProblemAsset> for ProblemAsset {
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

        Ok(ProblemAsset {
            symbol,
            target_weight,
            current_amount: current_amount.round_n(AMOUNT_DECIMALS),
        })
    }
}
