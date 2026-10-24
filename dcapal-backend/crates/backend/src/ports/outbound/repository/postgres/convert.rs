//! Canonical conversions between application enums and their stored codes.

use crate::{
    app::domain::portfolio::{AssetClass as AppAssetClass, Provider as AppProvider},
    ports::outbound::repository::postgres::types::{
        AssetClass as DbAssetClass, Provider as DbProvider,
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
