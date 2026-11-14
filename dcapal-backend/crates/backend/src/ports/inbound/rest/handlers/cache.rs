//! `Cache-Control` helpers shared by the cacheable catalog and price handlers.

use std::time::Duration;

use axum_extra::headers::CacheControl;
use lazy_static::lazy_static;

use crate::app::infra::utils::Expiring;

lazy_static! {
    /// Cache policy of the asset catalogs: public for five minutes.
    pub(super) static ref ASSETS_CACHE_CONTROL: CacheControl = CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(5 * 60));
}

/// Builds a public `Cache-Control` valid until `t` becomes stale.
pub(super) fn cache_control<T: Expiring>(t: &T) -> CacheControl {
    // Cache only until the domain object itself becomes stale.
    CacheControl::new()
        .with_public()
        .with_max_age(Duration::from_secs(t.time_to_live().as_secs()))
}
