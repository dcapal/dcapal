use std::{collections::HashMap, sync::Arc};

use futures::{StreamExt, stream};
use itertools::Itertools;

use crate::{
    DateTime,
    app::{
        commands::lookup_assets::LookupAssetsCommand,
        domain::asset::{AssetRef, MetadataSource, ResolvedAsset, StoredAsset},
        infra::stats::{SyncEvent, SyncStage, record_sync_event},
    },
    ports::outbound::{
        adapter::{AssetProvider, AssetProviderError},
        repository::asset::{AssetDirectory, AssetDirectoryError},
    },
};

/// Maximum number of provider lookups in flight for one resolution.
const PROVIDER_CONCURRENCY: usize = 8;

/// Maximum number of provider lookups that one resolution may start.
///
/// References unknown to the directory must all fit, otherwise the resolution
/// fails. Whatever budget they leave over is used, best effort, to refresh
/// directory rows whose metadata no provider has confirmed yet.
pub const MAX_NEW_ASSETS_PER_RESOLUTION: usize = 50;

/// How long a client-supplied row is left alone after a provider answered a
/// refresh without confirming it, so rows the provider cannot confirm are not
/// looked up again on every synchronization.
pub const CONFIRM_RETRY_COOLDOWN: chrono::Duration = chrono::Duration::hours(24);

/// Errors raised while resolving asset references at their providers.
#[derive(Debug, thiserror::Error)]
pub enum AssetResolutionError {
    /// These references are unknown to the directory and to their provider.
    #[error("{} asset reference(s) cannot be resolved", .0.len())]
    Unresolved(Vec<AssetRef>),
    /// One resolution would send more unknown references to providers than allowed.
    #[error("more than {limit} new assets cannot be resolved at once")]
    TooManyNewAssets {
        /// The maximum number of unknown references per resolution.
        limit: usize,
    },
    /// A provider could not answer, so the references' existence is undecided.
    #[error("asset provider is unavailable")]
    ProviderUnavailable(#[source] AssetProviderError),
    /// The shared asset directory failed while checking the references.
    #[error("asset directory failed")]
    Directory(#[from] AssetDirectoryError),
}

/// Outcome of a read-only directory lookup, both lists in request order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLookup {
    /// Metadata of the references stored in the directory.
    pub assets: Vec<ResolvedAsset>,
    /// References that are not stored.
    pub not_found: Vec<AssetRef>,
}

/// Outcome of resolving references: what to write back to the directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedAssets {
    /// Provider-resolved metadata to store: assets absent from the directory
    /// and client-supplied rows refreshed from their provider.
    pub to_upsert: Vec<ResolvedAsset>,
    /// Client-supplied rows whose provider knows them but whose cached
    /// metadata is kept, so they only need marking as provider-confirmed.
    pub to_confirm: Vec<AssetRef>,
    /// Client-supplied rows whose provider answered "unknown", so the attempt
    /// is recorded and the row is not retried before [`CONFIRM_RETRY_COOLDOWN`].
    /// Provider outages are not recorded.
    pub attempted: Vec<AssetRef>,
}

impl ResolvedAssets {
    /// Tells whether the directory has nothing to be told.
    #[cfg(test)]
    pub fn is_clean(&self) -> bool {
        self.to_upsert.is_empty() && self.to_confirm.is_empty() && self.attempted.is_empty()
    }
}

/// Why a provider lookup was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookupPurpose {
    /// The directory has no row; the lookup decides whether the asset exists.
    New,
    /// The directory row is client-supplied; the lookup is a best-effort refresh.
    Refresh,
}

/// Resolves asset references into shared metadata.
pub struct AssetService {
    directory: Arc<dyn AssetDirectory>,
    provider: Arc<dyn AssetProvider>,
}

impl AssetService {
    /// Creates a service over the shared directory and the provider adapters.
    pub fn new(directory: Arc<dyn AssetDirectory>, provider: Arc<dyn AssetProvider>) -> Self {
        Self {
            directory,
            provider,
        }
    }

    /// Reads the directory only, never contacting a provider.
    pub async fn lookup_known(
        &self,
        command: LookupAssetsCommand,
    ) -> Result<AssetLookup, AssetDirectoryError> {
        let known = self.directory.find_by_refs(command.assets()).await?;
        let (stored, not_found) = split_known(command.assets(), known);
        let assets = stored.into_iter().map(|stored| stored.asset).collect();

        Ok(AssetLookup { assets, not_found })
    }

    /// Resolves every distinct reference, in first-seen order, and returns
    /// what must be written back to the directory.
    ///
    /// Rows the provider already confirmed are used as they are. References
    /// missing from the directory are looked up at their provider with bounded
    /// concurrency, unless more than [`MAX_NEW_ASSETS_PER_RESOLUTION`] are
    /// missing, which fails before any provider call. Rows whose metadata was
    /// only client-supplied are refreshed from the provider, best effort: only
    /// as many as the budget left by the missing references allows are looked
    /// up, and any failure or "unknown" answer keeps the cached metadata. A row
    /// whose last answered attempt is younger than [`CONFIRM_RETRY_COOLDOWN`] is
    /// not looked up again; an "unknown" answer starts that cooldown, a failure
    /// does not. When the
    /// provider does not vouch for the names it reports (Kraken's catalog has
    /// no better name than the symbol), the cached metadata is kept and the row
    /// is only marked for confirmation.
    /// Nothing is stored; see [`Self::onboard`].
    ///
    /// Emits the `resolve` stage duration and the sync events
    /// `too_many_new_assets`, `refresh_attempted` (number of refresh lookups
    /// started) and `new_assets_resolved` (number of unknown references a
    /// provider resolved).
    /// A provider failure for a missing reference takes precedence over
    /// unresolved references because the failure leaves their existence undecided.
    pub async fn resolve<'a>(
        &self,
        refs: impl IntoIterator<Item = &'a AssetRef>,
    ) -> Result<ResolvedAssets, AssetResolutionError> {
        let _timer = SyncStage::Resolve.timer();
        let distinct = dedupe(refs);
        let known = self.directory.find_by_refs(&distinct).await?;
        let (stored, missing) = split_known(&distinct, known);
        if missing.len() > MAX_NEW_ASSETS_PER_RESOLUTION {
            record_sync_event(SyncEvent::TooManyNewAssets, 1);
            return Err(AssetResolutionError::TooManyNewAssets {
                limit: MAX_NEW_ASSETS_PER_RESOLUTION,
            });
        }

        let refresh_budget = MAX_NEW_ASSETS_PER_RESOLUTION - missing.len();
        let now = chrono::Utc::now();
        let refreshes = stored
            .iter()
            .filter(|stored| stored.source == MetadataSource::ClientSupplied)
            .filter(|stored| is_due_for_retry(stored.last_confirm_attempt, now))
            .map(|stored| AssetRef::from(&stored.asset))
            .take(refresh_budget)
            .collect::<Vec<_>>();

        record_sync_event(SyncEvent::RefreshAttempted, refreshes.len() as u64);

        // Owned references keep the lookup futures free of borrowed closure
        // arguments, which would make them unusable inside Axum handlers.
        let planned = missing
            .into_iter()
            .map(|reference| (reference, LookupPurpose::New))
            .chain(
                refreshes
                    .into_iter()
                    .map(|reference| (reference, LookupPurpose::Refresh)),
            );
        let lookups = stream::iter(planned)
            .map(|(reference, purpose)| async move {
                let outcome = self.provider.lookup(&reference).await;
                (reference, purpose, outcome)
            })
            .buffer_unordered(PROVIDER_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

        let mut to_upsert = Vec::new();
        let mut to_confirm = Vec::new();
        let mut attempted = Vec::new();
        let mut unresolved = Vec::new();
        let mut outage = None;
        let mut new_resolved = 0u64;
        for (reference, purpose, outcome) in lookups {
            match (purpose, outcome) {
                (LookupPurpose::New, Ok(Some(found))) => {
                    new_resolved += 1;
                    to_upsert.push(found.asset);
                }
                (LookupPurpose::New, Ok(None)) => unresolved.push(reference),
                (LookupPurpose::New, Err(error)) => outage = outage.or(Some(error)),
                (LookupPurpose::Refresh, Ok(Some(found))) => {
                    if found.name_authoritative {
                        to_upsert.push(found.asset);
                    } else {
                        // The provider does not vouch for the name, which
                        // would replace a possibly better cached name.
                        to_confirm.push(reference);
                    }
                }
                // The provider does not know it: keep the cached metadata and
                // do not ask again before the cooldown elapsed.
                (LookupPurpose::Refresh, Ok(None)) => attempted.push(reference),
                // A refresh is best effort: keep the cached metadata. The
                // attempt is not recorded so the next sync retries.
                (LookupPurpose::Refresh, Err(_)) => {}
            }
        }
        record_sync_event(SyncEvent::NewAssetsResolved, new_resolved);
        if let Some(error) = outage {
            return Err(AssetResolutionError::ProviderUnavailable(error));
        }

        // Position of every distinct reference, to report results in request order.
        let order = distinct
            .iter()
            .enumerate()
            .map(|(position, reference)| (reference, position))
            .collect::<HashMap<_, _>>();
        if !unresolved.is_empty() {
            unresolved.sort_by_key(|reference| order.get(reference).copied());
            return Err(AssetResolutionError::Unresolved(unresolved));
        }

        to_upsert.sort_by_cached_key(|asset| order.get(&AssetRef::from(asset)).copied());
        to_confirm.sort_by_key(|reference| order.get(reference).copied());
        attempted.sort_by_key(|reference| order.get(reference).copied());
        Ok(ResolvedAssets {
            to_upsert,
            to_confirm,
            attempted,
        })
    }

    /// Writes resolution results to the directory: provider-resolved metadata
    /// is upserted (never over provider-confirmed rows) and kept client-supplied
    /// rows are marked confirmed; refresh attempts the provider answered
    /// without confirming are timestamped. Empty inputs cause no directory call.
    pub async fn onboard(
        &self,
        to_upsert: &[ResolvedAsset],
        to_confirm: &[AssetRef],
        attempted: &[AssetRef],
    ) -> Result<(), AssetDirectoryError> {
        let _timer = SyncStage::Onboard.timer();
        if !to_upsert.is_empty() {
            self.directory.upsert_resolved(to_upsert).await?;
        }
        if !to_confirm.is_empty() {
            self.directory.confirm(to_confirm).await?;
        }
        if !attempted.is_empty() {
            self.directory
                .record_confirm_attempts(attempted, chrono::Utc::now())
                .await?;
        }
        Ok(())
    }
}

/// Tells whether a client-supplied row may be refreshed now: it was never
/// attempted, or its last answered attempt is older than the cooldown.
fn is_due_for_retry(last_attempt: Option<DateTime>, now: DateTime) -> bool {
    last_attempt.is_none_or(|attempt| now - attempt >= CONFIRM_RETRY_COOLDOWN)
}

/// Returns the distinct references in first-seen order, cloning each only once.
fn dedupe<'a>(refs: impl IntoIterator<Item = &'a AssetRef>) -> Vec<AssetRef> {
    refs.into_iter().unique().cloned().collect()
}

/// Splits distinct references into stored assets and the references not stored,
/// each in the order of `distinct`.
fn split_known(
    distinct: &[AssetRef],
    known: Vec<StoredAsset>,
) -> (Vec<StoredAsset>, Vec<AssetRef>) {
    let mut known = known
        .into_iter()
        .map(|stored| (AssetRef::from(&stored.asset), stored))
        .collect::<HashMap<_, _>>();
    let mut stored = Vec::new();
    let mut missing = Vec::new();
    for reference in distinct {
        match known.remove(reference) {
            Some(asset) => stored.push(asset),
            None => missing.push(reference.clone()),
        }
    }
    (stored, missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::domain::portfolio::{AssetClass, Provider},
        ports::outbound::{
            adapter::{MockAssetProvider, ProviderAsset},
            repository::asset::MockAssetDirectory,
        },
    };

    fn reference(symbol: &str) -> AssetRef {
        AssetRef::new(Provider::YF, symbol)
    }

    fn asset(symbol: &str) -> ResolvedAsset {
        ResolvedAsset {
            provider: Provider::YF,
            symbol: symbol.to_string(),
            name: format!("{symbol} name"),
            currency: "EUR".to_string(),
            asset_class: AssetClass::Equity,
        }
    }

    /// Metadata the provider reports for `symbol`, distinct from the cached one.
    fn provider_asset(symbol: &str) -> ResolvedAsset {
        ResolvedAsset {
            name: format!("{symbol} official"),
            ..asset(symbol)
        }
    }

    /// A provider answer whose name is authoritative, as Yahoo's is.
    fn vouched(asset: ResolvedAsset) -> ProviderAsset {
        ProviderAsset {
            asset,
            name_authoritative: true,
        }
    }

    fn stored(asset: ResolvedAsset, source: MetadataSource) -> StoredAsset {
        StoredAsset {
            asset,
            source,
            last_confirm_attempt: None,
        }
    }

    /// A client-supplied row whose provider last answered `age` ago.
    fn attempted_ago(symbol: &str, age: chrono::Duration) -> StoredAsset {
        StoredAsset {
            last_confirm_attempt: Some(chrono::Utc::now() - age),
            ..unverified(symbol)
        }
    }

    fn confirmed(symbol: &str) -> StoredAsset {
        stored(asset(symbol), MetadataSource::ProviderConfirmed)
    }

    fn unverified(symbol: &str) -> StoredAsset {
        stored(asset(symbol), MetadataSource::ClientSupplied)
    }

    fn service(directory: MockAssetDirectory, provider: MockAssetProvider) -> AssetService {
        AssetService::new(Arc::new(directory), Arc::new(provider))
    }

    fn directory_with(rows: Vec<StoredAsset>) -> MockAssetDirectory {
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(move |_| Ok(rows.clone()));
        directory
    }

    #[tokio::test]
    async fn resolves_confirmed_assets_without_the_provider() {
        // GIVEN a directory holding provider-confirmed rows for every requested asset,
        // WHEN the references are resolved,
        // THEN the provider is never called and nothing needs writing back.
        let directory = directory_with(vec![confirmed("B"), confirmed("A")]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().never();

        let resolved = service(directory, provider)
            .resolve(&[reference("A"), reference("B")])
            .await
            .unwrap();

        assert!(resolved.is_clean());
    }

    #[tokio::test]
    async fn resolves_unknown_assets_through_the_provider() {
        // GIVEN a directory holding only a confirmed A and a provider that knows B,
        // WHEN A and B are resolved,
        // THEN only B is looked up at the provider and queued for upsert, and nothing
        // is stored by resolving.
        let mut directory = directory_with(vec![confirmed("A")]);
        directory.expect_upsert_resolved().never();
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(vouched(asset(r.symbol())))));

        let resolved = service(directory, provider)
            .resolve(&[reference("B"), reference("A")])
            .await
            .unwrap();

        assert_eq!(resolved.to_upsert, vec![asset("B")]);
        assert!(resolved.to_confirm.is_empty());
    }

    #[tokio::test]
    async fn refreshes_unverified_assets_from_the_provider() {
        // GIVEN a client-supplied row for A and a provider reporting different metadata,
        // WHEN A is resolved,
        // THEN the provider's metadata is queued for upsert instead of the cached one.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(vouched(provider_asset(r.symbol())))));

        let resolved = service(directory, provider)
            .resolve(&[reference("A")])
            .await
            .unwrap();

        assert_eq!(resolved.to_upsert, vec![provider_asset("A")]);
        assert!(resolved.to_confirm.is_empty());
    }

    #[tokio::test]
    async fn unverified_asset_falls_back_to_cache_when_the_provider_does_not_know_it() {
        // GIVEN a client-supplied row for A and a provider that answers "unknown",
        // WHEN A is resolved,
        // THEN resolution succeeds, nothing is queued for upsert, so the cached metadata
        // stays, and the attempt is queued for recording.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().times(1).returning(|_| Ok(None));

        let resolved = service(directory, provider)
            .resolve(&[reference("A")])
            .await
            .unwrap();

        assert!(resolved.to_upsert.is_empty() && resolved.to_confirm.is_empty());
        assert_eq!(resolved.attempted, vec![reference("A")]);
    }

    #[tokio::test]
    async fn unverified_asset_falls_back_to_cache_when_the_provider_fails() {
        // GIVEN a client-supplied row for A and a provider that is down,
        // WHEN A is resolved,
        // THEN resolution still succeeds and nothing is queued, so the cached metadata stays.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|_| Err(AssetProviderError::Malformed("boom".into())));

        let resolved = service(directory, provider)
            .resolve(&[reference("A")])
            .await
            .unwrap();

        assert!(resolved.is_clean());
    }

    #[tokio::test]
    async fn unverified_kraken_asset_keeps_its_cached_name_and_is_queued_for_confirmation() {
        // GIVEN a client-supplied Kraken row with a descriptive name and a Kraken
        // catalog that only knows the symbol as the name,
        // WHEN it is resolved,
        // THEN nothing is queued for upsert, so the cached metadata is kept, and the
        // reference is queued for confirmation only.
        let kraken = ResolvedAsset {
            provider: Provider::Kraken,
            name: "Bitcoin".to_string(),
            ..asset("BTC")
        };
        let directory =
            directory_with(vec![stored(kraken.clone(), MetadataSource::ClientSupplied)]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().times(1).returning(|r| {
            Ok(Some(ProviderAsset {
                asset: ResolvedAsset {
                    provider: Provider::Kraken,
                    name: r.symbol().to_string(),
                    ..asset(r.symbol())
                },
                name_authoritative: false,
            }))
        });

        let resolved = service(directory, provider)
            .resolve(&[AssetRef::from(&kraken)])
            .await
            .unwrap();

        assert!(resolved.to_upsert.is_empty());
        assert_eq!(resolved.to_confirm, vec![AssetRef::from(&kraken)]);
    }

    #[tokio::test]
    async fn refresh_lookups_share_the_budget_and_never_starve_unknown_assets() {
        // GIVEN 10 unknown references and more unverified rows than the remaining budget,
        // WHEN everything is resolved,
        // THEN exactly 50 lookups run: all unknown ones plus the first 40 unverified
        // ones in request order; the rest keep their cached metadata.
        let unverified_rows = (0..60)
            .map(|i| unverified(&format!("U{i:02}")))
            .collect::<Vec<_>>();
        let mut refs = unverified_rows
            .iter()
            .map(|row| AssetRef::from(&row.asset))
            .collect::<Vec<_>>();
        refs.extend((0..10).map(|i| reference(&format!("N{i}"))));
        let directory = directory_with(unverified_rows);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(MAX_NEW_ASSETS_PER_RESOLUTION)
            .returning(|r| Ok(Some(vouched(provider_asset(r.symbol())))));

        let resolved = service(directory, provider).resolve(&refs).await.unwrap();

        assert_eq!(resolved.to_upsert.len(), MAX_NEW_ASSETS_PER_RESOLUTION);
        let upserted = |symbol: &str| resolved.to_upsert.iter().any(|a| a.symbol == symbol);
        assert!(upserted("N0") && upserted("N9"));
        assert!(upserted("U00") && upserted("U39"));
        assert!(!upserted("U40") && !upserted("U59"));
    }

    #[tokio::test]
    async fn full_unknown_budget_leaves_unverified_assets_alone() {
        // GIVEN exactly the maximum number of unknown references plus an unverified row,
        // WHEN they are resolved,
        // THEN only the unknown references reach the provider and the unverified row is untouched.
        let directory = directory_with(vec![unverified("OLD")]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(MAX_NEW_ASSETS_PER_RESOLUTION)
            .withf(|r| r.symbol() != "OLD")
            .returning(|r| Ok(Some(vouched(asset(r.symbol())))));
        let mut refs = vec![reference("OLD")];
        refs.extend((0..MAX_NEW_ASSETS_PER_RESOLUTION).map(|i| reference(&format!("S{i}"))));

        let resolved = service(directory, provider).resolve(&refs).await.unwrap();

        assert_eq!(resolved.to_upsert.len(), MAX_NEW_ASSETS_PER_RESOLUTION);
        assert!(resolved.to_confirm.is_empty());
    }

    #[tokio::test]
    async fn queues_resolved_assets_in_request_order() {
        // GIVEN an empty directory and a provider that knows three assets,
        // WHEN they are resolved in a non-alphabetical order,
        // THEN the assets queued for upsert follow the request order.
        let directory = directory_with(vec![]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|r| Ok(Some(vouched(asset(r.symbol())))));

        let resolved = service(directory, provider)
            .resolve(&[reference("C"), reference("A"), reference("B")])
            .await
            .unwrap();

        assert_eq!(resolved.to_upsert, vec![asset("C"), asset("A"), asset("B")]);
    }

    #[tokio::test]
    async fn reports_unresolved_tickers() {
        // GIVEN a provider that does not know ticker NOPE,
        // WHEN it is resolved next to a known asset,
        // THEN resolution fails naming exactly the unknown reference.
        let directory = directory_with(vec![confirmed("A")]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().returning(|_| Ok(None));

        let error = service(directory, provider)
            .resolve(&[reference("A"), reference("NOPE")])
            .await
            .unwrap_err();

        assert!(
            matches!(error, AssetResolutionError::Unresolved(refs) if refs == vec![reference("NOPE")])
        );
    }

    #[tokio::test]
    async fn provider_outage_is_distinct_from_unknown_tickers() {
        // GIVEN one provider failure and one unknown ticker,
        // WHEN both are resolved,
        // THEN the outage is reported rather than the unresolved reference.
        let directory = directory_with(vec![]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().returning(|r| {
            if r.symbol() == "DOWN" {
                Err(AssetProviderError::Malformed("boom".into()))
            } else {
                Ok(None)
            }
        });

        let error = service(directory, provider)
            .resolve(&[reference("NOPE"), reference("DOWN")])
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            AssetResolutionError::ProviderUnavailable(_)
        ));
    }

    #[tokio::test]
    async fn duplicate_references_are_resolved_once() {
        // GIVEN the same reference submitted three times in different casing,
        // WHEN it is resolved,
        // THEN the directory and the provider see it once and one asset is queued.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .withf(|refs| refs.len() == 1)
            .returning(|_| Ok(vec![]));
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(vouched(asset(r.symbol())))));

        let resolved = service(directory, provider)
            .resolve(&[reference("a"), reference("A"), reference(" a ")])
            .await
            .unwrap();

        assert_eq!(resolved.to_upsert, vec![asset("A")]);
    }

    #[tokio::test]
    async fn rejects_more_unknown_assets_than_the_cap_before_any_provider_call() {
        // GIVEN an empty directory and one more unknown reference than the cap allows,
        // WHEN the references are resolved,
        // THEN resolution fails with TooManyNewAssets and no provider is called.
        let directory = directory_with(vec![]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().never();
        let refs = (0..=MAX_NEW_ASSETS_PER_RESOLUTION)
            .map(|i| reference(&format!("S{i}")))
            .collect::<Vec<_>>();

        let error = service(directory, provider)
            .resolve(&refs)
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            AssetResolutionError::TooManyNewAssets { limit } if limit == MAX_NEW_ASSETS_PER_RESOLUTION
        ));
    }

    #[tokio::test]
    async fn known_assets_do_not_count_towards_the_new_asset_cap() {
        // GIVEN a directory holding more confirmed assets than the cap plus one unknown reference,
        // WHEN they are resolved,
        // THEN only the unknown one reaches the provider and resolution succeeds.
        let known = (0..=MAX_NEW_ASSETS_PER_RESOLUTION)
            .map(|i| confirmed(&format!("K{i}")))
            .collect::<Vec<_>>();
        let mut refs = known
            .iter()
            .map(|a| AssetRef::from(&a.asset))
            .collect::<Vec<_>>();
        refs.push(reference("NEW"));
        let directory = directory_with(known);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(vouched(asset(r.symbol())))));

        let resolved = service(directory, provider).resolve(&refs).await.unwrap();

        assert_eq!(resolved.to_upsert, vec![asset("NEW")]);
    }

    #[tokio::test]
    async fn lookup_known_reads_only_the_directory_whatever_the_source() {
        // GIVEN a directory holding an unverified A but not B,
        // WHEN both are looked up,
        // THEN A is returned with its cached metadata, B is reported as not found,
        // and no provider is contacted.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().never();
        let command =
            LookupAssetsCommand::try_new(&["yf:b".to_string(), "yf:a".to_string()]).unwrap();

        let lookup = service(directory, provider)
            .lookup_known(command)
            .await
            .unwrap();

        assert_eq!(lookup.assets, vec![asset("A")]);
        assert_eq!(lookup.not_found, vec![reference("B")]);
    }

    #[tokio::test]
    async fn onboard_persists_only_what_is_given() {
        // GIVEN resolved assets to upsert and references to confirm,
        // WHEN they are onboarded,
        // THEN the directory receives each list once; an empty list causes no call.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_upsert_resolved()
            .withf(|assets| assets == [asset("A")])
            .times(1)
            .returning(|_| Ok(()));
        directory
            .expect_confirm()
            .withf(|refs| refs == [reference("C")])
            .times(1)
            .returning(|_| Ok(()));
        directory
            .expect_record_confirm_attempts()
            .withf(|refs, _| refs == [reference("D")])
            .times(1)
            .returning(|_, _| Ok(()));
        let service = service(directory, MockAssetProvider::new());

        service
            .onboard(&[asset("A")], &[reference("C")], &[reference("D")])
            .await
            .unwrap();
        service.onboard(&[], &[], &[]).await.unwrap();
    }

    #[tokio::test]
    async fn recent_attempts_are_skipped_and_old_ones_retried_after_the_cooldown() {
        // GIVEN client-supplied rows last attempted 1h ago (A) and 25h ago (B),
        // plus one never attempted (C),
        // WHEN they are resolved,
        // THEN only B and C reach the provider; A is left alone.
        let directory = directory_with(vec![
            attempted_ago("A", chrono::Duration::hours(1)),
            attempted_ago("B", chrono::Duration::hours(25)),
            unverified("C"),
        ]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(2)
            .withf(|r| r.symbol() != "A")
            .returning(|_| Ok(None));

        let resolved = service(directory, provider)
            .resolve(&[reference("A"), reference("B"), reference("C")])
            .await
            .unwrap();

        assert_eq!(resolved.attempted, vec![reference("B"), reference("C")]);
    }

    #[tokio::test]
    async fn recent_attempts_do_not_consume_the_refresh_budget() {
        // GIVEN 50 recently attempted rows and one never attempted row,
        // WHEN they are resolved,
        // THEN the never attempted row is still looked up.
        let mut rows = (0..MAX_NEW_ASSETS_PER_RESOLUTION)
            .map(|i| attempted_ago(&format!("R{i:02}"), chrono::Duration::hours(1)))
            .collect::<Vec<_>>();
        rows.push(unverified("FRESH"));
        let refs = rows
            .iter()
            .map(|row| AssetRef::from(&row.asset))
            .collect::<Vec<_>>();
        let directory = directory_with(rows);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .withf(|r| r.symbol() == "FRESH")
            .returning(|_| Ok(None));

        let resolved = service(directory, provider).resolve(&refs).await.unwrap();

        assert_eq!(resolved.attempted, vec![reference("FRESH")]);
    }

    #[tokio::test]
    async fn provider_outage_during_refresh_is_not_recorded_as_an_attempt() {
        // GIVEN a never attempted client-supplied row and a provider that is down,
        // WHEN it is resolved,
        // THEN resolution succeeds and no attempt is recorded, so the next sync retries.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|_| Err(AssetProviderError::Malformed("boom".into())));

        let resolved = service(directory, provider)
            .resolve(&[reference("A")])
            .await
            .unwrap();

        assert!(resolved.attempted.is_empty());
    }

    #[tokio::test]
    async fn confirmed_and_refreshed_rows_are_not_recorded_as_attempts() {
        // GIVEN a client-supplied row the provider knows and vouches for,
        // WHEN it is resolved,
        // THEN it is upserted as confirmed and no attempt is recorded.
        let directory = directory_with(vec![unverified("A")]);
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .returning(|r| Ok(Some(vouched(provider_asset(r.symbol())))));

        let resolved = service(directory, provider)
            .resolve(&[reference("A")])
            .await
            .unwrap();

        assert!(resolved.attempted.is_empty());
        assert_eq!(resolved.to_upsert.len(), 1);
    }

    #[tokio::test]
    async fn onboard_records_attempts_in_one_call_only_when_there_are_any() {
        // GIVEN two attempted references,
        // WHEN they are onboarded,
        // THEN the directory records both in a single call; with none, no call is made.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_record_confirm_attempts()
            .withf(|refs, _| refs == [reference("A"), reference("B")])
            .times(1)
            .returning(|_, _| Ok(()));
        let service = service(directory, MockAssetProvider::new());

        service
            .onboard(&[], &[], &[reference("A"), reference("B")])
            .await
            .unwrap();
        service.onboard(&[], &[], &[]).await.unwrap();
    }
}
