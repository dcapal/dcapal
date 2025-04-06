#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, sync::Mutex};

use rand::{Rng, distributions};
use rust_decimal::{
    Decimal,
    prelude::{One, ToPrimitive},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use optimize::{
    FeeStructure, FeeStructureFixed, FeeStructureVariable, TransactionFees,
    advanced::{self, TheoreticalAllocation},
    basic,
};
use utils::{parse_amount, parse_percentage, parse_shares};

use crate::optimize::suggestions;

pub mod optimize;
mod utils;

const AMOUNT_DECIMALS: u32 = 4;
const PERCENTAGE_DECIMALS: u32 = 6;
const SHARES_DECIMALS: u32 = 8;

lazy_static! {
    static ref BASIC_PROBLEMS: Mutex<HashMap<String, optimize::basic::Problem>> =
        Mutex::new(HashMap::new());
    static ref ADVANCED_PROBLEMS: Mutex<HashMap<String, optimize::advanced::Problem>> =
        Mutex::new(HashMap::new());
    static ref SUGGESTION_PROBLEMS: Mutex<HashMap<String, optimize::suggestions::Problem>> =
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
            JsProblemOptions::Analyze(options) => {
                let options = suggestions::ProblemOptions::try_from(options)?;
                let problem = optimize::suggestions::Problem::new(options);

                let mut problems = SUGGESTION_PROBLEMS.lock().unwrap();
                problems.insert(id.clone(), problem);

                Ok(ProblemHandle {
                    id,
                    kind: ProblemKind::Analyze,
                })
            }
        }
    }

    pub fn solve(handle: &ProblemHandle) -> Result<JsValue, JsValue> {
        utils::require_init();

        match handle.kind {
            ProblemKind::Advanced => Self::solve_advanced(&handle.id),
            ProblemKind::Basic => Self::solve_basic(&handle.id),
            ProblemKind::Analyze => Self::suggest_amount_to_invest(&handle.id),
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

        let theo_allocs = solution
            .assets
            .iter()
            .filter_map(|(aid, v)| v.theo_alloc.clone().map(|t| (aid.clone(), t.into())))
            .collect();

        let js_solution = JsAdvancedSolution {
            budget_left,
            amounts,
            shares,
            theo_allocs,
        };
        Ok(serde_wasm_bindgen::to_value(&js_solution).unwrap())
    }

    fn suggest_amount_to_invest(id: &str) -> Result<JsValue, JsValue> {
        let problems = SUGGESTION_PROBLEMS.lock().unwrap();
        let problem = problems
            .get(id)
            .ok_or_else(|| format!("Invalid problem id {}", id))?;

        let solution = problem.suggest_invest_amount().round_dp(AMOUNT_DECIMALS);

        Ok(serde_wasm_bindgen::to_value(&solution).unwrap())
    }

    pub fn delete_problem(handle: &ProblemHandle) -> Result<bool, JsValue> {
        utils::require_init();

        Ok(match handle.kind {
            ProblemKind::Advanced => delete_problem(&ADVANCED_PROBLEMS, &handle.id),
            ProblemKind::Basic => delete_problem(&BASIC_PROBLEMS, &handle.id),
            ProblemKind::Analyze => delete_problem(&SUGGESTION_PROBLEMS, &handle.id),
        })
    }
}

fn delete_problem<P>(problems: &Mutex<HashMap<String, P>>, id: &str) -> bool {
    problems.lock().unwrap().remove(id).is_some()
}

#[wasm_bindgen]
pub enum ProblemKind {
    Advanced,
    Basic,
    Analyze,
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
    pub theo_allocs: HashMap<String, JsTheoreticalAllocation>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JsTheoreticalAllocation {
    pub shares: f64,
    pub amount: f64,
    pub fees: f64,
}

impl From<TheoreticalAllocation> for JsTheoreticalAllocation {
    fn from(value: TheoreticalAllocation) -> Self {
        Self {
            shares: value.shares.to_f64().unwrap(),
            amount: value.amount.to_f64().unwrap(),
            fees: value.fees.to_f64().unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsProblemOptions {
    Advanced(JsAdvancedOptions),
    Basic(JsBasicOptions),
    Analyze(JsAnalyzeOptions),
}

#[derive(Serialize, Deserialize)]
pub struct JsAdvancedOptions {
    pub budget: f64,
    pub pfolio_ccy: String,
    pub assets: HashMap<String, JsAdvancedAsset>,
    pub fees: Option<JsTransactionFees>,
    pub is_buy_only: bool,
    #[serde(default)]
    pub use_all_budget: bool,
}

#[derive(Serialize, Deserialize)]
pub struct JsAnalyzeOptions {
    pub assets: HashMap<String, JsAnalyzeAsset>,
}

#[derive(Serialize, Deserialize)]
pub struct JsAdvancedAsset {
    pub symbol: String,
    pub shares: f64,
    pub price: f64,
    pub target_weight: f64,
    pub is_whole_shares: bool,
    pub fees: Option<JsTransactionFees>,
}

#[derive(Serialize, Deserialize)]
pub struct JsAnalyzeAsset {
    pub symbol: String,
    pub shares: f64,
    pub price: f64,
    pub target_weight: f64,
    pub is_whole_shares: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsTransactionFees {
    /// Maximum acceptable fee impact (as a rate, in [0..1] range)
    pub max_fee_impact: Option<f64>,
    pub fee_structure: JsFeeStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsFeeStructure {
    ZeroFee,
    Fixed(JsFeeStructureFixed),
    Variable(JsFeeStructureVariable),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsFeeStructureFixed {
    pub fee_amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsFeeStructureVariable {
    pub min_fee: Option<f64>,
    pub max_fee: Option<f64>,
    pub fee_rate: Option<f64>,
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

        if budget + current_total <= Decimal::ZERO {
            return Err(format!(
                "Invalid input. Budget + portfolio value must be positive. (budget={budget} portfolio_value={current_total})"
            ));
        }

        let fees = options
            .fees
            .map(TransactionFees::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(advanced::ProblemOptions {
            pfolio_ccy: options.pfolio_ccy,
            current_pfolio_amount: current_total,
            assets,
            budget,
            fees,
            is_buy_only: options.is_buy_only,
            use_all_budget: options.use_all_budget,
        })
    }
}

impl TryFrom<JsAnalyzeOptions> for suggestions::ProblemOptions {
    type Error = String;

    fn try_from(options: JsAnalyzeOptions) -> Result<Self, Self::Error> {
        let assets = options
            .assets
            .into_iter()
            .map(|(aid, a)| suggestions::ProblemAsset::try_from(a).map(|a| (aid, a)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        let current_total = assets
            .values()
            .map(|a| a.price * a.shares)
            .sum::<Decimal>()
            .round_dp(AMOUNT_DECIMALS);

        Ok(suggestions::ProblemOptions {
            current_pfolio_amount: current_total,
            assets,
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
            fees,
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

        let fees = fees.map(TransactionFees::try_from).transpose()?;

        Ok(advanced::ProblemAsset {
            symbol,
            shares: parse_shares(shares),
            price: parse_amount(price),
            target_weight: parse_percentage(target_weight),
            is_whole_shares,
            fees,
        })
    }
}

impl TryFrom<JsAnalyzeAsset> for suggestions::ProblemAsset {
    type Error = String;

    fn try_from(asset: JsAnalyzeAsset) -> Result<Self, Self::Error> {
        let JsAnalyzeAsset {
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

        Ok(suggestions::ProblemAsset {
            symbol,
            shares: parse_shares(shares),
            price: parse_amount(price),
            target_weight: parse_percentage(target_weight),
            is_whole_shares,
        })
    }
}

impl TryFrom<JsTransactionFees> for TransactionFees {
    type Error = String;

    fn try_from(value: JsTransactionFees) -> Result<Self, Self::Error> {
        if let Some(max) = value.max_fee_impact {
            if !(0.0..=1.0).contains(&max) {
                return Err(format!(
                    "Invalid max_fee_impact ({max}). Must be in [0, 1] range"
                ));
            }
        }

        let max_fee_impact = value
            .max_fee_impact
            .map(parse_percentage)
            .unwrap_or_else(TransactionFees::default_max_fee_impact);

        Ok(Self {
            max_fee_impact,
            fee_structure: value.fee_structure.try_into()?,
        })
    }
}

impl TryFrom<JsFeeStructure> for FeeStructure {
    type Error = String;

    fn try_from(value: JsFeeStructure) -> Result<Self, Self::Error> {
        Ok(match value {
            JsFeeStructure::ZeroFee => Self::default(),
            JsFeeStructure::Fixed(fee) => FeeStructure::Fixed(fee.try_into()?),
            JsFeeStructure::Variable(fee) => FeeStructure::Variable(fee.try_into()?),
        })
    }
}

impl TryFrom<JsFeeStructureFixed> for FeeStructureFixed {
    type Error = String;

    fn try_from(value: JsFeeStructureFixed) -> Result<Self, Self::Error> {
        if let Some(amount) = value.fee_amount {
            if amount < 0. {
                return Err(format!("Invalid fee_amount ({amount}). Must be positive"));
            }
        }

        let fee_amount = value.fee_amount.map(parse_amount).unwrap_or(Decimal::ZERO);

        Ok(Self { fee_amount })
    }
}

impl TryFrom<JsFeeStructureVariable> for FeeStructureVariable {
    type Error = String;

    fn try_from(value: JsFeeStructureVariable) -> Result<Self, Self::Error> {
        if let Some(rate) = value.fee_rate {
            if !(0.0..=1.0).contains(&rate) {
                return Err(format!(
                    "Invalid fee_rate ({rate}). Must be in [0, 1] range"
                ));
            }
        }

        let fee_rate = value
            .fee_rate
            .map(parse_percentage)
            .unwrap_or(Decimal::ZERO);

        Ok(Self {
            min_fee: value.min_fee.map(parse_amount),
            max_fee: value.max_fee.map(parse_amount),
            fee_rate,
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
                budget, current_total
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
            target_weight: parse_percentage(target_weight),
            current_amount: parse_amount(current_amount),
        })
    }
}
