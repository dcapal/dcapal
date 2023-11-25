use std::collections::BTreeMap;

use dcapal_optimizer_wasm::{
    JsAdvancedAsset, JsAdvancedOptions, JsProblemOptions, JsTransactionFees,
};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub algorithm: Algorithm,
    #[serde(with = "rust_decimal::serde::float")]
    pub budget: Decimal,
    pub is_buy_only: bool,
    #[serde(default)]
    pub use_all_budget: bool,
    pub portfolio: Portfolio,
    pub expect: Expect,
}

impl Scenario {
    pub fn split(self) -> (JsProblemOptions, Expect) {
        let expect = self.expect;
        let options = match self.algorithm {
            Algorithm::Basic => todo!(),
            Algorithm::Advanced => {
                let budget = self.budget.to_f64().unwrap();
                let assets = self
                    .portfolio
                    .assets
                    .into_iter()
                    .map(|a| (a.symbol.clone(), a.into()))
                    .collect::<_>();

                JsProblemOptions::Advanced(JsAdvancedOptions {
                    budget,
                    pfolio_ccy: self.portfolio.quote_ccy,
                    assets,
                    fees: self.portfolio.fees,
                    is_buy_only: self.is_buy_only,
                    use_all_budget: self.use_all_budget,
                })
            }
        };

        (options, expect)
    }
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
    pub fees: Option<JsTransactionFees>,
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
    pub fees: Option<JsTransactionFees>,
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
    Solved(ExpectedSolution),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedSolution {
    pub solution: Option<BTreeMap<String, AssetSolution>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSolution {
    #[serde(with = "rust_decimal::serde::float_option")]
    pub shares: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub amount: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub weight: Option<Decimal>,
}

impl From<Asset> for JsAdvancedAsset {
    fn from(value: Asset) -> Self {
        JsAdvancedAsset {
            symbol: value.symbol,
            shares: value.qty.to_f64().unwrap(),
            price: value.price.to_f64().unwrap(),
            target_weight: value.target_weight.to_f64().unwrap() / 100.,
            is_whole_shares: value.aclass.is_whole_shares(),
            fees: value.fees,
        }
    }
}
