//! Asset search across the providers the v2 market-data contract supports.

use std::{sync::Arc, time::Duration};

use moka::sync::Cache;

use crate::{
    app::{
        commands::market_quotes::SearchAssetsCommand,
        domain::{
            market_quote::{FailureKind, PriceFailure},
            portfolio::{AssetClass, Provider},
        },
    },
    error::DcaError,
    ports::outbound::{
        adapter::{AssetSearchError, AssetSearchProvider, ListedAsset},
        repository::market_data::MarketDataRepository,
    },
};

/// Maximum number of assets a Kraken catalog search returns.
pub const CATALOG_SEARCH_LIMIT: i64 = 20;

/// Quote currency of Kraken crypto prices.
const CRYPTO_QUOTE_CURRENCY: &str = "USD";

/// Whether a price is expected to be available for a found asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum PriceAvailability {
    /// A USD market with a stored price exists.
    Available,
    /// The catalog lists the asset but no price is stored for it.
    Unavailable,
    /// The provider's search does not say; ask for the current price to find out.
    Unknown,
}

/// One asset found by a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchHit {
    /// Provider that lists the asset.
    pub provider: Provider,
    /// Provider symbol.
    pub symbol: String,
    /// Display name, when known.
    pub name: Option<String>,
    /// Exchange or market, when known.
    pub venue: Option<String>,
    /// Currency prices of the asset are expressed in, when the search knows it.
    pub currency: Option<String>,
    /// Asset class.
    pub asset_class: AssetClass,
    /// Whether a price is expected to be available.
    pub price_availability: PriceAvailability,
}

/// Overall outcome of a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum SearchOutcome {
    /// At least one asset was found.
    Usable,
    /// The provider answered and nothing matches.
    Empty,
    /// The provider could not be reached or answered with an error.
    ProviderError,
    /// The provider answered with an unreadable payload.
    Malformed,
}

/// The answer to a search: the assets found or the visible failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearch {
    /// The searched provider.
    pub provider: Provider,
    /// The search text.
    pub query: String,
    /// Overall outcome.
    pub outcome: SearchOutcome,
    /// The assets found, best match first.
    pub results: Vec<AssetSearchHit>,
    /// Why the provider gave no answer.
    pub failure: Option<PriceFailure>,
}

/// Failures that prevent a search from producing an [`AssetSearch`].
#[derive(Debug, thiserror::Error)]
pub enum AssetSearchServiceError {
    /// The provider's shared rate limit is exhausted; retry later.
    #[error("the provider rate limit is exhausted")]
    RateLimited,
    /// The catalog could not be read.
    #[error("the asset catalog could not be searched")]
    Storage(#[from] DcaError),
}

/// Searches Yahoo's listings and the Kraken crypto catalog, each on its own.
pub struct AssetSearchService {
    repo: Arc<MarketDataRepository>,
    listings: Arc<dyn AssetSearchProvider>,
    listed_cache: Cache<String, Vec<ListedAsset>>,
}

/// How long the answer to a Yahoo search is reused.
const LISTED_SEARCH_TTL: Duration = Duration::from_secs(300);

/// Number of distinct Yahoo searches kept in memory.
const LISTED_SEARCH_CAPACITY: u64 = 1_000;

impl AssetSearchService {
    /// Creates a service that searches Yahoo through `listings` and Kraken in `repo`.
    pub fn new(repo: Arc<MarketDataRepository>, listings: Arc<dyn AssetSearchProvider>) -> Self {
        Self {
            repo,
            listings,
            listed_cache: Cache::builder()
                .max_capacity(LISTED_SEARCH_CAPACITY)
                .time_to_live(LISTED_SEARCH_TTL)
                .build(),
        }
    }

    /// Searches the command's provider only; results are never merged across providers.
    pub async fn search(
        &self,
        command: SearchAssetsCommand,
    ) -> Result<AssetSearch, AssetSearchServiceError> {
        let SearchAssetsCommand { provider, query } = command;
        let found = match provider {
            Provider::YF => self.search_yahoo(&query).await,
            Provider::Kraken => self.search_kraken(&query).await?,
        };

        Ok(match found {
            Ok(results) => AssetSearch {
                provider,
                query,
                outcome: if results.is_empty() {
                    SearchOutcome::Empty
                } else {
                    SearchOutcome::Usable
                },
                results,
                failure: None,
            },
            Err(AssetSearchError::RateLimited) => {
                return Err(AssetSearchServiceError::RateLimited);
            }
            Err(AssetSearchError::Failed(failure)) => {
                // The reason stays in the logs; clients only see the failure kind.
                tracing::warn!(%provider, %failure, "Asset search failed");
                AssetSearch {
                    provider,
                    query,
                    outcome: match failure.kind {
                        FailureKind::Malformed => SearchOutcome::Malformed,
                        FailureKind::Unavailable | FailureKind::ProviderError => {
                            SearchOutcome::ProviderError
                        }
                    },
                    results: Vec::new(),
                    failure: Some(failure),
                }
            }
        })
    }

    /// Searches Yahoo; prices of its results are not known until asked for.
    async fn search_yahoo(&self, query: &str) -> Result<Vec<AssetSearchHit>, AssetSearchError> {
        // Answers are reused for the same text, ignoring case and surrounding blanks;
        // failures, rate-limit rejections and empty answers (which Yahoo also gives on
        // transient hiccups) are never remembered.
        let key = query.trim().to_lowercase();
        let listed = match self.listed_cache.get(&key) {
            Some(listed) => listed,
            None => {
                let listed = self.listings.search_assets(query).await?;
                if !listed.is_empty() {
                    self.listed_cache.insert(key, listed.clone());
                }
                listed
            }
        };
        Ok(listed
            .into_iter()
            .map(|asset| AssetSearchHit {
                provider: Provider::YF,
                symbol: asset.symbol,
                name: asset.name,
                venue: asset.venue,
                currency: None,
                asset_class: asset.asset_class,
                price_availability: PriceAvailability::Unknown,
            })
            .collect())
    }

    /// Searches the stored Kraken crypto catalog; Kraken has no upstream search.
    async fn search_kraken(
        &self,
        query: &str,
    ) -> Result<Result<Vec<AssetSearchHit>, AssetSearchError>, AssetSearchServiceError> {
        let hits = self
            .repo
            .search_crypto_catalog(query, CATALOG_SEARCH_LIMIT)
            .await?;
        Ok(Ok(hits
            .into_iter()
            .map(|hit| AssetSearchHit {
                provider: Provider::Kraken,
                symbol: hit.symbol,
                name: Some(hit.name),
                venue: None,
                currency: Some(CRYPTO_QUOTE_CURRENCY.to_string()),
                asset_class: AssetClass::Crypto,
                price_availability: if hit.priced {
                    PriceAvailability::Available
                } else {
                    PriceAvailability::Unavailable
                },
            })
            .collect()))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::{
        app::domain::entity::{Asset, Crypto},
        ports::outbound::adapter::{ListedAsset, MockAssetSearchProvider},
    };

    fn command(provider: Provider, query: &str) -> SearchAssetsCommand {
        SearchAssetsCommand {
            provider,
            query: query.to_string(),
        }
    }

    fn service(pool: PgPool, listings: MockAssetSearchProvider) -> AssetSearchService {
        AssetSearchService::new(
            Arc::new(MarketDataRepository::new(pool)),
            Arc::new(listings),
        )
    }

    /// GIVEN a Yahoo search that finds an ETF, finds nothing, fails and answers garbage,
    /// WHEN each search runs,
    /// THEN the outcome is usable (price availability unknown), empty, provider error and
    /// malformed, and a saturated limiter is a rate-limit error.
    #[sqlx::test(migrations = "../../migrations")]
    async fn yahoo_search_outcomes(pool: PgPool) {
        let mut listings = MockAssetSearchProvider::new();
        let mut answers = vec![
            Ok(vec![ListedAsset {
                symbol: "VWCE.DE".into(),
                name: Some("Vanguard".into()),
                venue: Some("XETRA".into()),
                asset_class: AssetClass::Equity,
            }]),
            Ok(vec![]),
            Err(AssetSearchError::Failed(PriceFailure::provider_error(
                "HTTP 503",
            ))),
            Err(AssetSearchError::Failed(PriceFailure::malformed("html"))),
            Err(AssetSearchError::RateLimited),
        ]
        .into_iter();
        listings
            .expect_search_assets()
            .times(5)
            .returning(move |_| answers.next().unwrap());
        let service = service(pool, listings);

        let usable = service.search(command(Provider::YF, "vwce")).await.unwrap();
        assert_eq!(usable.outcome, SearchOutcome::Usable);
        assert_eq!(
            usable.results[0].price_availability,
            PriceAvailability::Unknown
        );
        assert_eq!(usable.results[0].currency, None);
        let outcomes = [
            SearchOutcome::Empty,
            SearchOutcome::ProviderError,
            SearchOutcome::Malformed,
        ];
        for (i, expected) in outcomes.into_iter().enumerate() {
            // Distinct texts, because a successful answer is cached per text.
            let text = format!("x{i}");
            let search = service.search(command(Provider::YF, &text)).await.unwrap();
            assert_eq!(search.outcome, expected);
            assert_eq!(search.failure.is_some(), expected != SearchOutcome::Empty);
        }
        let limited = service.search(command(Provider::YF, "limited")).await;
        assert!(matches!(limited, Err(AssetSearchServiceError::RateLimited)));
    }

    /// GIVEN a Kraken catalog and a Yahoo search that must not be called,
    /// WHEN the Kraken provider is searched,
    /// THEN the catalog answers with USD-priced availability and Yahoo is never asked.
    #[sqlx::test(migrations = "../../migrations")]
    async fn kraken_search_reads_the_catalog_only(pool: PgPool) {
        let repo = MarketDataRepository::new(pool.clone());
        repo.store_asset(&Asset::Crypto(Crypto::new_with_id("btc".into())))
            .await
            .unwrap();
        let mut listings = MockAssetSearchProvider::new();
        listings.expect_search_assets().times(0);
        let service = service(pool, listings);

        let found = service
            .search(command(Provider::Kraken, "btc"))
            .await
            .unwrap();
        let empty = service
            .search(command(Provider::Kraken, "zzz"))
            .await
            .unwrap();

        assert_eq!(found.outcome, SearchOutcome::Usable);
        let hit = &found.results[0];
        assert_eq!(
            (hit.provider, hit.symbol.as_str()),
            (Provider::Kraken, "BTC")
        );
        assert_eq!(hit.currency.as_deref(), Some("USD"));
        assert_eq!(hit.asset_class, AssetClass::Crypto);
        assert_eq!(hit.price_availability, PriceAvailability::Unavailable);
        assert_eq!(empty.outcome, SearchOutcome::Empty);
    }

    /// GIVEN a Yahoo search that found assets, one that found nothing and one that failed,
    /// WHEN each text is searched twice (the first with different case and blanks),
    /// THEN the answer with assets is reused (one provider call), while the empty answer and
    /// the failure are asked of Yahoo again.
    #[sqlx::test(migrations = "../../migrations")]
    async fn successful_yahoo_searches_are_cached_and_failures_are_not(pool: PgPool) {
        let mut listings = MockAssetSearchProvider::new();
        listings
            .expect_search_assets()
            .withf(|q| q.trim().eq_ignore_ascii_case("vwce"))
            .times(1)
            .returning(|_| {
                Ok(vec![ListedAsset {
                    symbol: "VWCE.DE".into(),
                    name: None,
                    venue: None,
                    asset_class: AssetClass::Equity,
                }])
            });
        listings
            .expect_search_assets()
            .withf(|q| q == "nothing")
            .times(2)
            .returning(|_| Ok(vec![]));
        listings
            .expect_search_assets()
            .withf(|q| q == "bad")
            .times(2)
            .returning(|_| {
                Err(AssetSearchError::Failed(PriceFailure::provider_error(
                    "503",
                )))
            });
        let service = service(pool, listings);

        service.search(command(Provider::YF, "vwce")).await.unwrap();
        service
            .search(command(Provider::YF, " VWCE "))
            .await
            .unwrap();
        service
            .search(command(Provider::YF, "nothing"))
            .await
            .unwrap();
        service
            .search(command(Provider::YF, "nothing"))
            .await
            .unwrap();
        service.search(command(Provider::YF, "bad")).await.unwrap();
        let retried = service.search(command(Provider::YF, "bad")).await.unwrap();

        assert_eq!(retried.outcome, SearchOutcome::ProviderError);
    }
}
