use bigdecimal::BigDecimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(tag = "type")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum FeeStructure {
    #[sea_orm(string_value = "ZeroFee")]
    ZeroFee,
    #[sea_orm(string_value = "Fixed")]
    Fixed {
        fee_amount: BigDecimal,
    },
    #[sea_orm(string_value = "Variable")]
    Variable {
        fee_rate: BigDecimal,
        min_fee: BigDecimal,
        max_fee: Option<BigDecimal>,
    },
}