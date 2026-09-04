use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use std::collections::HashSet;

use tracing::{debug, error};

use super::MarketDataRepository;
use crate::{
    app::domain::entity::{Market, MarketId},
    error::{DcaError, Result},
    ports::outbound::repository::{REDIS_BASE, dto::MarketDto},
};

const MARKET_KEY: &str = concatcp!(REDIS_BASE, ':', "market");
const DELETE_STALE_MARKETS_SCRIPT: &str = r#"
local existing = redis.call('HKEYS', KEYS[1])
local keep = {}
for _, id in ipairs(ARGV) do
    keep[id] = true
end

local stale = {}
for _, id in ipairs(existing) do
    if not keep[id] then
        table.insert(stale, id)
    end
end

if #stale > 0 then
    redis.call('HDEL', KEYS[1], unpack(stale))
end

return stale
"#;

#[async_trait]
pub trait RedisMarket {
    async fn store(&self, conn: &mut impl redis::AsyncCommands) -> Result<bool>;

    async fn delete_not_in(
        market_ids: &HashSet<MarketId>,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Vec<MarketId>>;

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

    async fn delete_not_in(
        market_ids: &HashSet<MarketId>,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Vec<MarketId>> {
        if market_ids.is_empty() {
            return Err(DcaError::Generic(
                "Cannot reconcile markets from an empty Kraken snapshot".to_string(),
            ));
        }

        let mut cmd = redis::cmd("EVAL");
        cmd.arg(DELETE_STALE_MARKETS_SCRIPT).arg(1).arg(MARKET_KEY);
        for market_id in market_ids {
            cmd.arg(market_id);
        }

        let deleted: Vec<MarketId> = cmd.query_async(conn).await?;

        if deleted.is_empty() {
            return Ok(deleted);
        }

        debug!(
            "Removed {} stale Kraken markets from '{}'",
            deleted.len(),
            MARKET_KEY
        );
        Ok(deleted)
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

#[cfg(test)]
fn stale_market_ids(existing: &[MarketId], keep: &HashSet<MarketId>) -> HashSet<MarketId> {
    existing
        .iter()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::stale_market_ids;

    #[test]
    fn reconciliation_deletes_only_fields_outside_the_snapshot() {
        // GIVEN the market hash contains Kraken markets in an arbitrary order
        let existing = [
            "etheur".to_string(),
            "btcusd".to_string(),
            "legacy".to_string(),
            "adaeur".to_string(),
        ];
        let desired = [
            "adaeur".to_string(),
            "btcusd".to_string(),
            "etheur".to_string(),
        ]
        .into_iter()
        .collect::<HashSet<_>>();

        // WHEN stale fields are selected for HDEL
        let stale = stale_market_ids(&existing, &desired);

        // THEN only the field absent from the complete snapshot is deleted
        assert_eq!(stale, ["legacy".to_string()].into_iter().collect());
    }

    #[test]
    fn reconciliation_selection_is_independent_of_redis_key_order() {
        // GIVEN the Redis HKEYS result order changes between calls
        let desired = ["btcusd".to_string()].into_iter().collect::<HashSet<_>>();

        // WHEN stale fields are selected from both orderings
        let first = stale_market_ids(
            &[
                "zeta".to_string(),
                "alpha".to_string(),
                "btcusd".to_string(),
            ],
            &desired,
        );
        let second = stale_market_ids(
            &[
                "alpha".to_string(),
                "btcusd".to_string(),
                "zeta".to_string(),
            ],
            &desired,
        );

        // THEN the same stale fields are selected for the single atomic HDEL
        assert_eq!(
            first,
            ["alpha".to_string(), "zeta".to_string()]
                .into_iter()
                .collect()
        );
        assert_eq!(first, second);
    }

    #[test]
    fn reconciliation_selection_returns_no_fields_when_all_markets_are_live() {
        // GIVEN every Redis market field exists in the current Kraken snapshot
        let existing = ["btcusd".to_string(), "etheur".to_string()];
        let desired = existing.iter().cloned().collect::<HashSet<_>>();

        // WHEN stale fields are selected for HDEL
        let stale = stale_market_ids(&existing, &desired);

        // THEN no field is selected for deletion
        assert!(stale.is_empty());
    }
}
