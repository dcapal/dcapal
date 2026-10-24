use std::{collections::HashSet, sync::Arc};

use futures::{StreamExt, stream};

use crate::{
    app::{
        commands::lookup_assets::LookupAssetsCommand,
        domain::asset::{AssetRef, ResolvedAsset},
    },
    ports::outbound::{
        adapter::{AssetProvider, AssetProviderError},
        repository::asset::{AssetDirectory, AssetDirectoryError},
    },
};

/// Maximum number of provider lookups in flight for one resolution.
const PROVIDER_CONCURRENCY: usize = 8;

/// Errors raised while resolving or reading shared asset metadata.
#[derive(Debug, thiserror::Error)]
pub enum AssetServiceError {
    /// These references are unknown to the directory and to their provider.
    #[error("{} asset reference(s) cannot be resolved", .0.len())]
    Unresolved(Vec<AssetRef>),
    /// A provider could not answer, so the references' existence is undecided.
    #[error("asset provider is unavailable")]
    ProviderUnavailable(#[source] AssetProviderError),
    /// The shared asset directory failed.
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
    ) -> Result<AssetLookup, AssetServiceError> {
        let known = self.directory.find_by_refs(command.assets()).await?;
        let (assets, not_found) = split_known(command.assets(), known);

        Ok(AssetLookup { assets, not_found })
    }

    /// Returns the metadata of every distinct reference, in first-seen order.
    ///
    /// References missing from the directory are looked up at their provider
    /// with bounded concurrency. Nothing is stored; see [`Self::onboard`].
    /// A provider failure takes precedence over unresolved references because
    /// the failure leaves their existence undecided.
    pub async fn resolve(
        &self,
        refs: &[AssetRef],
    ) -> Result<Vec<ResolvedAsset>, AssetServiceError> {
        let distinct = dedupe(refs);
        let known = self.directory.find_by_refs(&distinct).await?;
        let (mut resolved, missing) = split_known(&distinct, known);

        // Owned references keep the lookup futures free of borrowed closure
        // arguments, which would make them unusable inside Axum handlers.
        let lookups = stream::iter(missing)
            .map(|reference| async move {
                let outcome = self.provider.lookup(&reference).await;
                (reference, outcome)
            })
            .buffer_unordered(PROVIDER_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

        let mut found = Vec::new();
        let mut unresolved = Vec::new();
        let mut outage = None;
        for (reference, outcome) in lookups {
            match outcome {
                Ok(Some(asset)) => found.push(asset),
                Ok(None) => unresolved.push(reference),
                Err(error) => outage = outage.or(Some(error)),
            }
        }
        if let Some(error) = outage {
            return Err(AssetServiceError::ProviderUnavailable(error));
        }
        if !unresolved.is_empty() {
            unresolved.sort_by_key(|r| distinct.iter().position(|d| d == r));
            return Err(AssetServiceError::Unresolved(unresolved));
        }

        resolved.extend(found);
        resolved.sort_by_key(|asset| distinct.iter().position(|d| *d == asset.as_ref()));
        Ok(resolved)
    }

    /// Stores resolved assets that the directory lacks; existing ones win.
    pub async fn onboard(&self, assets: &[ResolvedAsset]) -> Result<(), AssetServiceError> {
        Ok(self.directory.insert_missing(assets).await?)
    }
}

fn dedupe(refs: &[AssetRef]) -> Vec<AssetRef> {
    let mut seen = HashSet::new();
    refs.iter()
        .filter(|r| seen.insert((*r).clone()))
        .cloned()
        .collect()
}

/// Splits distinct references into stored assets and the references not stored,
/// each in the order of `distinct`.
fn split_known(
    distinct: &[AssetRef],
    known: Vec<ResolvedAsset>,
) -> (Vec<ResolvedAsset>, Vec<AssetRef>) {
    let mut known = known
        .into_iter()
        .map(|asset| (asset.as_ref(), asset))
        .collect::<std::collections::HashMap<_, _>>();
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
        ports::outbound::{adapter::MockAssetProvider, repository::asset::MockAssetDirectory},
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

    fn service(directory: MockAssetDirectory, provider: MockAssetProvider) -> AssetService {
        AssetService::new(Arc::new(directory), Arc::new(provider))
    }

    #[tokio::test]
    async fn resolves_known_assets_without_the_provider() {
        // GIVEN a directory holding every requested asset,
        // WHEN the references are resolved,
        // THEN the stored metadata is returned and the provider is never called.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(vec![asset("B"), asset("A")]));
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().never();

        let resolved = service(directory, provider)
            .resolve(&[reference("A"), reference("B")])
            .await
            .unwrap();

        assert_eq!(resolved, vec![asset("A"), asset("B")]);
    }

    #[tokio::test]
    async fn resolves_unknown_assets_through_the_provider() {
        // GIVEN a directory holding only A and a provider that knows B,
        // WHEN A and B are resolved,
        // THEN B comes from the provider, results keep request order, and nothing is stored.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(vec![asset("A")]));
        directory.expect_insert_missing().never();
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(asset(&r.symbol))));

        let resolved = service(directory, provider)
            .resolve(&[reference("B"), reference("A")])
            .await
            .unwrap();

        assert_eq!(resolved, vec![asset("B"), asset("A")]);
    }

    #[tokio::test]
    async fn reports_unresolved_tickers() {
        // GIVEN a provider that does not know ticker NOPE,
        // WHEN it is resolved next to a known asset,
        // THEN resolution fails naming exactly the unknown reference.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(vec![asset("A")]));
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().returning(|_| Ok(None));

        let error = service(directory, provider)
            .resolve(&[reference("A"), reference("NOPE")])
            .await
            .unwrap_err();

        assert!(
            matches!(error, AssetServiceError::Unresolved(refs) if refs == vec![reference("NOPE")])
        );
    }

    #[tokio::test]
    async fn provider_outage_is_distinct_from_unknown_tickers() {
        // GIVEN one provider failure and one unknown ticker,
        // WHEN both are resolved,
        // THEN the outage is reported rather than the unresolved reference.
        let mut directory = MockAssetDirectory::new();
        directory.expect_find_by_refs().returning(|_| Ok(vec![]));
        let mut provider = MockAssetProvider::new();
        provider.expect_lookup().returning(|r| {
            if r.symbol == "DOWN" {
                Err(AssetProviderError::Malformed("boom".into()))
            } else {
                Ok(None)
            }
        });

        let error = service(directory, provider)
            .resolve(&[reference("NOPE"), reference("DOWN")])
            .await
            .unwrap_err();

        assert!(matches!(error, AssetServiceError::ProviderUnavailable(_)));
    }

    #[tokio::test]
    async fn duplicate_references_are_resolved_once() {
        // GIVEN the same reference submitted three times in different casing,
        // WHEN it is resolved,
        // THEN the directory and the provider see it once and one asset is returned.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .withf(|refs| refs.len() == 1)
            .returning(|_| Ok(vec![]));
        let mut provider = MockAssetProvider::new();
        provider
            .expect_lookup()
            .times(1)
            .returning(|r| Ok(Some(asset(&r.symbol))));

        let resolved = service(directory, provider)
            .resolve(&[reference("a"), reference("A"), reference(" a ")])
            .await
            .unwrap();

        assert_eq!(resolved, vec![asset("A")]);
    }

    #[tokio::test]
    async fn lookup_known_reads_only_the_directory() {
        // GIVEN a directory holding A but not B,
        // WHEN both are looked up,
        // THEN A is found, B is reported as not found, and no provider is contacted.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_find_by_refs()
            .returning(|_| Ok(vec![asset("A")]));
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
    async fn onboard_delegates_to_the_directory() {
        // GIVEN resolved assets, WHEN they are onboarded,
        // THEN the directory receives them for an idempotent insert.
        let mut directory = MockAssetDirectory::new();
        directory
            .expect_insert_missing()
            .withf(|assets| assets == [asset("A")])
            .times(1)
            .returning(|_| Ok(()));

        service(directory, MockAssetProvider::new())
            .onboard(&[asset("A")])
            .await
            .unwrap();
    }
}
