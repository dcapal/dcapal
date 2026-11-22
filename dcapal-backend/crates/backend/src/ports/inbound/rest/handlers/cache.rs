//! `Cache-Control` helpers shared by the cacheable catalog and price handlers.

use std::{sync::LazyLock, time::Duration};

use axum_extra::headers::CacheControl;

use jiff::Timestamp;

use crate::app::{domain::entity::Price, infra::utils::Expiring};

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

/// Builds the `Cache-Control` of v2 price and FX answers from the storage time of each
/// usable value (`None` for a value that is missing).
///
/// The answer is cacheable until the stalest value crosses its price-validity window,
/// like the v1 price; with any missing value, or none at all, it must be revalidated.
pub(super) fn quotes_cache_control(
    fetched: impl IntoIterator<Item = Option<Timestamp>>,
) -> CacheControl {
    let mut oldest_ttl = None;
    for entry in fetched {
        let Some(at) = entry else {
            return CacheControl::new().with_no_cache();
        };
        let ttl = Price::new(1.0, at).time_to_live();
        oldest_ttl = Some(oldest_ttl.map_or(ttl, |t: Duration| t.min(ttl)));
    }
    match oldest_ttl {
        Some(ttl) => CacheControl::new()
            .with_public()
            .with_max_age(Duration::from_secs(ttl.as_secs())),
        None => CacheControl::new().with_no_cache(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GIVEN fresh values, a missing value, and no values,
    /// WHEN the cache policy is built,
    /// THEN fresh values are public with a max-age within the validity window, and a
    /// missing value or an empty answer must be revalidated.
    #[test]
    fn missing_values_disable_caching() {
        let fresh = quotes_cache_control([Some(Timestamp::now())]);
        assert!(fresh.public());
        assert!(fresh.max_age().unwrap() <= Duration::from_secs(300));

        assert!(quotes_cache_control([Some(Timestamp::now()), None]).no_cache());
        assert!(quotes_cache_control([]).no_cache());
    }
}
