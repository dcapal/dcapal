use rust_decimal::Decimal;
use sqlx::{
    Postgres,
    encode::IsNull,
    error::BoxDynError,
    postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
};

use crate::app::domain::portfolio::{FeeStructure, TransactionFees};

/// The fee type names stored in the `fee_type` columns.
///
/// The columns are `TEXT` or `VARCHAR` depending on the migration history, so
/// the enum is encoded and decoded as a string, which accepts both.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString, strum::IntoStaticStr,
)]
pub enum FeeType {
    /// No transaction fee applies.
    ZeroFee,
    /// A fixed amount applies to each transaction.
    Fixed,
    /// A percentage rate applies, optionally bounded.
    Variable,
}

impl sqlx::Type<Postgres> for FeeType {
    fn type_info() -> PgTypeInfo {
        <str as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <str as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl PgHasArrayType for FeeType {
    fn array_type_info() -> PgTypeInfo {
        <&str as PgHasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &PgTypeInfo) -> bool {
        <&str as PgHasArrayType>::array_compatible(ty)
    }
}

impl sqlx::Encode<'_, Postgres> for FeeType {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&str as sqlx::Encode<Postgres>>::encode(self.into(), buf)
    }
}

impl sqlx::Decode<'_, Postgres> for FeeType {
    /// Rejects a stored name that is not a known fee type.
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(<&str as sqlx::Decode<Postgres>>::decode(value)?.parse()?)
    }
}

/// A stored fee column set that cannot be read as a fee policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FeeFieldsError {
    /// A Fixed fee is stored without its amount.
    #[error("Fixed fee requires fee_amount to be Some.")]
    FixedWithoutAmount,
    /// A Variable fee is stored without its rate.
    #[error("Variable fee requires fee_rate to be Some.")]
    VariableWithoutRate,
}

/// The fee columns shared by Portfolios and Portfolio Assets.
///
/// An absent fee type means no fee policy is stored.
#[derive(Debug, Clone, Default, PartialEq, sqlx::FromRow)]
pub struct FeeFields {
    /// The maximum fee impact configured for transactions.
    pub max_fee_impact: Option<Decimal>,
    /// The persisted fee structure name.
    pub fee_type: Option<FeeType>,
    /// The fixed fee amount, when the fee structure is fixed.
    pub fee_amount: Option<Decimal>,
    /// The variable fee rate, when the fee structure is variable.
    pub fee_rate: Option<Decimal>,
    /// The minimum variable fee, when configured.
    pub min_fee: Option<Decimal>,
    /// The maximum variable fee, when configured.
    pub max_fee: Option<Decimal>,
}

impl FeeFields {
    /// Maps an optional application fee policy to its persisted columns.
    ///
    /// An absent policy leaves every fee column empty.
    pub fn from_fees(fees: Option<&TransactionFees>) -> Self {
        let Some(fees) = fees else {
            return Self::default();
        };
        let base = Self {
            max_fee_impact: fees.max_fee_impact,
            fee_type: Some(fees.fee_structure.fee_type().into()),
            ..Default::default()
        };
        match fees.fee_structure {
            FeeStructure::ZeroFee => base,
            FeeStructure::Fixed { fee_amount } => Self {
                fee_amount: Some(fee_amount),
                ..base
            },
            FeeStructure::Variable {
                fee_rate,
                min_fee,
                max_fee,
            } => Self {
                fee_rate: Some(fee_rate),
                min_fee,
                max_fee,
                ..base
            },
        }
    }

    /// Reads the stored columns leniently, as the legacy v1 API always has:
    /// columns the fee type does not use are ignored and values are not
    /// range-checked. Only a missing amount or rate, without which no policy
    /// exists, is an error. No fee type means no policy.
    pub fn into_fees(self) -> Result<Option<TransactionFees>, FeeFieldsError> {
        let Some(fee_type) = self.fee_type else {
            return Ok(None);
        };
        let fee_structure = match fee_type {
            FeeType::ZeroFee => FeeStructure::ZeroFee,
            FeeType::Fixed => FeeStructure::Fixed {
                fee_amount: self.fee_amount.ok_or(FeeFieldsError::FixedWithoutAmount)?,
            },
            FeeType::Variable => FeeStructure::Variable {
                fee_rate: self.fee_rate.ok_or(FeeFieldsError::VariableWithoutRate)?,
                min_fee: self.min_fee,
                max_fee: self.max_fee,
            },
        };
        Ok(Some(TransactionFees {
            max_fee_impact: self.max_fee_impact,
            fee_structure,
        }))
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    #[test]
    fn lenient_decoding_ignores_unused_columns_and_ranges() {
        // GIVEN stored fee columns no v2 validation would accept (a Fixed fee
        // carrying a rate, an out-of-range impact, a negative amount),
        // WHEN they are read for the v1 API,
        // THEN the policy is still produced from the columns its type uses.
        let fields = FeeFields {
            max_fee_impact: Some(dec!(500)),
            fee_type: Some(FeeType::Fixed),
            fee_amount: Some(dec!(-1)),
            fee_rate: Some(dec!(3)),
            ..Default::default()
        };

        let fees = fields.into_fees().expect("lenient decode");

        assert_eq!(
            fees,
            Some(TransactionFees {
                max_fee_impact: Some(dec!(500)),
                fee_structure: FeeStructure::Fixed {
                    fee_amount: dec!(-1)
                },
            })
        );
    }

    #[test]
    fn lenient_decoding_requires_the_amount_or_rate_of_the_fee_type() {
        // GIVEN a Fixed fee without amount and a Variable fee without rate,
        // WHEN they are read,
        // THEN each reports which column is missing; orphan columns without a
        // fee type simply mean no policy.
        let fixed = FeeFields {
            fee_type: Some(FeeType::Fixed),
            ..Default::default()
        };
        let variable = FeeFields {
            fee_type: Some(FeeType::Variable),
            ..Default::default()
        };
        let orphan = FeeFields {
            fee_amount: Some(dec!(1)),
            ..Default::default()
        };

        assert_eq!(fixed.into_fees(), Err(FeeFieldsError::FixedWithoutAmount));
        assert_eq!(
            variable.into_fees(),
            Err(FeeFieldsError::VariableWithoutRate)
        );
        assert_eq!(orphan.into_fees(), Ok(None));
    }

    #[test]
    fn fee_policies_round_trip_through_the_columns() {
        // GIVEN each application fee policy,
        // WHEN it is mapped to columns and read back,
        // THEN the same policy results, and an absent policy leaves all columns empty.
        let policies = [
            FeeStructure::ZeroFee,
            FeeStructure::Fixed {
                fee_amount: dec!(2),
            },
            FeeStructure::Variable {
                fee_rate: dec!(0.2),
                min_fee: Some(dec!(1)),
                max_fee: None,
            },
        ];

        for fee_structure in policies {
            let fees = TransactionFees {
                max_fee_impact: Some(dec!(5)),
                fee_structure,
            };
            let read = FeeFields::from_fees(Some(&fees)).into_fees();
            assert_eq!(read, Ok(Some(fees)));
        }
        assert_eq!(FeeFields::from_fees(None), FeeFields::default());
    }
}
