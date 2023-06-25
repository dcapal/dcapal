use anyhow::anyhow;
use dcapal_optimizer_wasm::{
    optimize::advanced, JsAdvancedAsset, JsAdvancedOptions, JsProblemOptions,
};

use glob::glob;
use log::info;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, io::BufReader, path::Path};

const SCENARIOS_PATH: &str = "./tests/scenarios";

#[test_log::test]
fn test_runner() -> anyhow::Result<()> {
    info!("==> 🚀  DcaPal Test Runner - Engine ignited");

    let path = Path::new(SCENARIOS_PATH).canonicalize()?;
    let pattern = format!("{}/**/*.json", SCENARIOS_PATH);
    info!("==> ⚙️  Loading test scenarios from \"{}\"", path.display());

    let scenarios = glob(&pattern)?.filter_map(|e| e.ok()).collect::<Vec<_>>();
    for path in &scenarios {
        info!("==> ⚙️  Loading scenario {:?}", path.file_name().unwrap());

        let scenario = read_scenario_from_file(path)?;

        let options = JsProblemOptions::from(scenario);
        let is_solved = match options {
            JsProblemOptions::Advanced(o) => solve_advanced(o)?,
            JsProblemOptions::Basic(_) => todo!(),
        };

        assert!(is_solved);

        // TODO: Build Js objects to reuse parsing logic
        // TODO: If expect == build error test Result
        // TODO: Solve and test expectations
    }

    Ok(())
}

fn solve_advanced(options: JsAdvancedOptions) -> anyhow::Result<bool> {
    let options = advanced::ProblemOptions::try_from(options).map_err(|e| anyhow!("{}", e))?;
    let problem = advanced::Problem::new(options);
    let solution = problem.solve();

    Ok(solution.is_solved)
}

fn read_scenario_from_file(path: &Path) -> anyhow::Result<Scenario> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let scenario: Scenario = serde_json::from_reader(reader)?;
    Ok(scenario)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub algorithm: Algorithm,
    #[serde(with = "rust_decimal::serde::float")]
    pub budget: Decimal,
    pub is_buy_only: bool,
    pub portfolio: Portfolio,
    pub expect: Expect,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Algorithm {
    Basic,
    Advanced,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub quote_ccy: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub symbol: String,
    pub name: String,
    pub aclass: AssetClass,
    #[serde(with = "rust_decimal::serde::float")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub qty: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub target_weight: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetClass {
    Equity,
    Crypto,
    Currency,
}

impl AssetClass {
    fn is_whole_shares(&self) -> bool {
        matches!(self, AssetClass::Equity)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "camelCase")]
pub enum Expect {
    BuildError,
    Solved(Solution),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub solution: BTreeMap<String, AssetSolution>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSolution {
    #[serde(with = "rust_decimal::serde::float")]
    pub shares: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub weight: Decimal,
}

impl From<Scenario> for JsProblemOptions {
    fn from(scenario: Scenario) -> Self {
        match scenario.algorithm {
            Algorithm::Basic => todo!(),
            Algorithm::Advanced => Self::Advanced(from_advanced(scenario)),
        }
    }
}

fn from_advanced(scenario: Scenario) -> JsAdvancedOptions {
    let budget = scenario.budget.to_f64().unwrap();
    let pfolio_ccy = scenario.portfolio.quote_ccy;
    let is_buy_only = scenario.is_buy_only;
    let assets = scenario
        .portfolio
        .assets
        .into_iter()
        .map(|a| (a.symbol.clone(), a.into()))
        .collect::<_>();

    JsAdvancedOptions {
        budget,
        pfolio_ccy,
        assets,
        is_buy_only,
    }
}

impl From<Asset> for JsAdvancedAsset {
    fn from(value: Asset) -> Self {
        JsAdvancedAsset {
            symbol: value.symbol,
            shares: value.qty.to_f64().unwrap(),
            price: value.price.to_f64().unwrap(),
            target_weight: value.target_weight.to_f64().unwrap() / 100.,
            is_whole_shares: value.aclass.is_whole_shares(),
        }
    }
}
