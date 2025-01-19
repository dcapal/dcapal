use bigdecimal::BigDecimal;
use sea_orm::DeriveEntityModel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, DeriveEntityModel)]
#[serde(tag = "type")]
#[sea_orm(entity_name = "fee_structure")]
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