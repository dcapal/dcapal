use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use tracing::{debug, error};

use super::MarketDataRepository;
use crate::{
    app::domain::entity::{Market, MarketId},
    error::{DcaError, Result},
    ports::outbound::repository::{REDIS_BASE, dto::MarketDto},
};

const MARKET_KEY: &str = concatcp!(REDIS_BASE, ':', "market");

#[async_trait]
pub trait RedisMarket {
    async fn store(&self, conn: &mut impl redis::AsyncCommands) -> Result<bool>;

    async fn find_by_id(
        id: &MarketId,
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Option<Market>>;

    async fn find_by_ids(
        ids: &[&MarketId],
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Vec<Option<Market>>>;

    async fn load_all(
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Vec<Market>>;
}

#[async_trait]
impl RedisMarket for Market {
    async fn store(&self, conn: &mut impl redis::AsyncCommands) -> Result<bool> {
        let dto = MarketDto::from(self.clone());
        let json = serde_json::to_string(&dto).unwrap();
        let _: () = conn.hset(MARKET_KEY, &self.id, &json).await?;

        debug!("Successfully stored '{} {}': {}", MARKET_KEY, self.id, json);
        Ok(true)
    }

    async fn find_by_id(
        id: &MarketId,
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Option<Market>> {
        let json: Option<String> = conn.hget(MARKET_KEY, id).await?;
        let Some(json) = json else {
            return Ok(None);
        };

        let market = serde_json::from_str(&json).map_err(|e| {
            DcaError::JsonDeserializationFailure(
                json,
                std::any::type_name::<MarketDto>().to_string(),
                e,
            )
        })?;

        resolve_market(market, repo).await
    }

    async fn find_by_ids(
        ids: &[&MarketId],
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Vec<Option<Market>>> {
        let jsons: Vec<Option<String>> = redis::cmd("HMGET").arg(ids).query_async(conn).await?;

        let dtos: Vec<Option<MarketDto>> = jsons
            .into_iter()
            .flat_map(|json| {
                json.map(|j| {
                    let dto: Result<MarketDto> = serde_json::from_str(&j).map_err(|e| {
                        DcaError::JsonDeserializationFailure(
                            j,
                            std::any::type_name::<MarketDto>().to_string(),
                            e,
                        )
                    });

                    match dto {
                        Ok(dto) => Some(dto),
                        Err(e) => {
                            error!("{:?}", e);
                            None
                        }
                    }
                })
            })
            .collect();

        let markets = futures::stream::iter(dtos)
            .then(|m| async move {
                if let Some(m) = m {
                    resolve_market(m, repo).await
                } else {
                    Ok(None)
                }
            })
            .inspect_err(|e| error!("Failed to resolve MarketDto: {}", e))
            .filter_map(|m| async move { m.ok() })
            .collect()
            .await;

        Ok(markets)
    }

    async fn load_all(
        conn: &mut impl redis::AsyncCommands,
        repo: &MarketDataRepository,
    ) -> Result<Vec<Market>> {
        let jsons: Vec<String> = conn.hvals(MARKET_KEY).await?;

        // Parse JSON into markets DTO
        let (markets, errors): (Vec<_>, Vec<_>) = jsons
            .into_iter()
            .map(|s| serde_json::from_str(&s))
            .partition(std::result::Result::is_ok);

        let markets: Vec<MarketDto> = markets
            .into_iter()
            .map(std::result::Result::unwrap)
            .collect();

        errors
            .into_iter()
            .map(std::result::Result::unwrap_err)
            .for_each(|e| {
                error!("Failed to parse JSON into MarketDto: {}", e);
            });

        // Resolve DTOs into Market domain object
        let markets = futures::stream::iter(markets)
            .then(|m| async move { resolve_market(m, repo).await })
            .inspect_err(|e| error!("Failed to resolve MarketDto: {}", e))
            .filter_map(|m| async move { m.ok() })
            .filter_map(futures::future::ready)
            .collect::<Vec<_>>()
            .await;

        Ok(markets)
    }
}

async fn resolve_market(market: MarketDto, repo: &MarketDataRepository) -> Result<Option<Market>> {
    let (base, quote) = tokio::join!(
        repo.find_asset(&market.base),
        repo.find_asset(&market.quote)
    );

    match (base?, quote?) {
        (None, _) => {
            error!(mkt = market.id, "Base asset not found: {}", &market.base);
            Ok(None)
        }
        (_, None) => {
            error!(mkt = market.id, "Quote asset not found: {}", &market.quote);
            Ok(None)
        }
        (Some(b), Some(q)) => Ok(Some(Market::new(market.id, b, q, market.price))),
    }
}
