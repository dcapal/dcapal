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

/// Maps an application Provider to its stored representation.
pub(super) fn db_provider(provider: &AppProvider) -> DbProvider {
    match provider {
        AppProvider::YF => DbProvider::YF,
        AppProvider::Kraken => DbProvider::Kraken,
    }
}

/// Maps a stored Provider to its application representation.
pub(super) fn app_provider(provider: DbProvider) -> AppProvider {
    match provider {
        DbProvider::YF => AppProvider::YF,
        DbProvider::Kraken => AppProvider::Kraken,
    }
}

/// Maps an application Asset Class to its stored representation.
pub(super) fn db_asset_class(class: AppAssetClass) -> DbAssetClass {
    match class {
        AppAssetClass::Other => DbAssetClass::Other,
        AppAssetClass::Equity => DbAssetClass::Equity,
        AppAssetClass::Bond => DbAssetClass::Bond,
        AppAssetClass::Cash => DbAssetClass::Cash,
        AppAssetClass::Crypto => DbAssetClass::Crypto,
        AppAssetClass::Commodity => DbAssetClass::Commodity,
    }
}

/// Maps a stored Asset Class to its application representation.
pub(super) fn app_asset_class(class: DbAssetClass) -> AppAssetClass {
    match class {
        DbAssetClass::Other => AppAssetClass::Other,
        DbAssetClass::Equity => AppAssetClass::Equity,
        DbAssetClass::Bond => AppAssetClass::Bond,
        DbAssetClass::Cash => AppAssetClass::Cash,
        DbAssetClass::Crypto => AppAssetClass::Crypto,
        DbAssetClass::Commodity => AppAssetClass::Commodity,
    }
}

/// Maps an application Allocation Mode to its stored representation.
pub(super) fn db_allocation_mode(mode: AppAllocationMode) -> DbAllocationMode {
    match mode {
        AppAllocationMode::Simple => DbAllocationMode::Simple,
        AppAllocationMode::Strategic => DbAllocationMode::Strategic,
    }
}

/// Maps a stored Allocation Mode to its application representation.
pub(super) fn app_allocation_mode(mode: DbAllocationMode) -> AppAllocationMode {
    match mode {
        DbAllocationMode::Simple => AppAllocationMode::Simple,
        DbAllocationMode::Strategic => AppAllocationMode::Strategic,
    }
}

/// Splits asset references into the parallel provider-code and symbol arrays
/// that SQL `UNNEST($1::smallint[], $2::text[])` joins consume.
pub(super) fn unnest_refs<'a>(
    refs: impl IntoIterator<Item = &'a AssetRef>,
) -> (Vec<i16>, Vec<String>) {
    refs.into_iter()
        .map(|reference| {
            (
                i16::from(db_provider(&reference.provider)),
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
            .map(|class| i16::from(db_asset_class(*class)))
            .collect();

        assert!(classes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(codes.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
