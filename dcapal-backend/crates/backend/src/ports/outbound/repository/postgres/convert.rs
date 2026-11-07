//! Canonical conversions between application enums and their stored codes.

use crate::{
    app::domain::{
        asset::AssetRef,
        portfolio::{
            AllocationMode as AppAllocationMode, AssetClass as AppAssetClass,
            Provider as AppProvider,
        },
    },
    ports::outbound::repository::postgres::types::{
        AllocationMode as DbAllocationMode, AssetClass as DbAssetClass, Provider as DbProvider,
    },
};

/// Generates the total, mutually inverse conversions between an application
/// enum and its stored counterpart, which share variant names one to one.
macro_rules! convert_enum {
    ($app:ty, $db:ty, [$($variant:ident),+ $(,)?]) => {
        impl From<$app> for $db {
            fn from(value: $app) -> Self {
                match value {
                    $(<$app>::$variant => <$db>::$variant,)+
                }
            }
        }

        impl From<$db> for $app {
            fn from(value: $db) -> Self {
                match value {
                    $(<$db>::$variant => <$app>::$variant,)+
                }
            }
        }
    };
}

convert_enum!(AppProvider, DbProvider, [YF, Kraken]);
convert_enum!(
    AppAssetClass,
    DbAssetClass,
    [Other, Equity, Bond, Cash, Crypto, Commodity]
);
convert_enum!(AppAllocationMode, DbAllocationMode, [Simple, Strategic]);

impl From<&AppProvider> for DbProvider {
    fn from(provider: &AppProvider) -> Self {
        provider.clone().into()
    }
}

/// Splits asset references into the parallel provider and symbol arrays
/// that SQL `UNNEST($1::smallint[], $2::text[])` joins consume.
pub(super) fn unnest_refs<'a>(
    refs: impl IntoIterator<Item = &'a AssetRef>,
) -> (Vec<DbProvider>, Vec<String>) {
    refs.into_iter()
        .map(|reference| {
            (
                DbProvider::from(&reference.provider),
                reference.symbol.clone(),
            )
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_class_order_matches_the_stored_codes() {
        // GIVEN every application Asset Class,
        // WHEN they are ordered by their natural order,
        // THEN the order equals the order of their stored codes, which is the
        // order synchronization responses list class targets in.
        let classes = [
            AppAssetClass::Other,
            AppAssetClass::Equity,
            AppAssetClass::Bond,
            AppAssetClass::Cash,
            AppAssetClass::Crypto,
            AppAssetClass::Commodity,
        ];

        let codes: Vec<i16> = classes
            .iter()
            .map(|class| i16::from(DbAssetClass::from(*class)))
            .collect();

        assert!(classes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(codes.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
