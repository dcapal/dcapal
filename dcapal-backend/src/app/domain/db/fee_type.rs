use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(tag = "type")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum FeeType {
    #[sea_orm(string_value = "ZeroFee")]
    ZeroFee,
    #[sea_orm(string_value = "Fixed")]
    Fixed,
    #[sea_orm(string_value = "Variable")]
    Variable,
}
