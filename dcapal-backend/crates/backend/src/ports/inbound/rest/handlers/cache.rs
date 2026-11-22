//! `Cache-Control` helpers shared by the cacheable catalog and price handlers.

use std::{sync::LazyLock, time::Duration};

use axum_extra::headers::CacheControl;

use crate::app::infra::utils::Expiring;

/// Cache policy of the asset catalogs: public for five minutes.
pub(super) static ASSETS_CACHE_CONTROL: LazyLock<CacheControl> = LazyLock::new(|| {
    CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(5 * 60))
});

/// Builds a public `Cache-Control` valid until `t` becomes stale.
pub(super) fn cache_control<T: Expiring>(t: &T) -> CacheControl {
    // Cache only until the domain object itself becomes stale.
    CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(t.time_to_live().as_secs()))
}
