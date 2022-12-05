pub mod dto;
mod utils;

use chrono::{TimeZone, Utc};
use deadpool_redis::Connection;
use futures::{StreamExt, TryStreamExt};
use redis::{AsyncCommands, FromRedisValue, Value};
use tracing::{error, info};

use std::string::ToString;

use crate::{
    adapter::GeoData,
    domain::entity::{Asset, AssetId, AssetKind, Market, MarketId},
    error::{DcaError, Result},
    DateTime,
};

use self::dto::MarketDto;

const REDIS_BASE: &str = "dca-pal:be";

#[derive(Clone)]
pub struct MiscRepository {
    redis: deadpool_redis::Pool,
}

impl MiscRepository {
    const ENTITY: &'static str = concatcp!(REDIS_BASE, ':', "misc");
    const CW_LAST_FETCHED: &'static str = "cw-last-fetched";

    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn get_cw_last_fetched(&self) -> Result<Option<DateTime>> {
        let mut redis = self.redis.get().await?;
        let last_fetched: Option<i64> = redis.hget(Self::ENTITY, Self::CW_LAST_FETCHED).await?;

        if last_fetched.is_none() {
            return Ok(None);
        }

        let last_fetched = Utc.timestamp_opt(last_fetched.unwrap(), 0).unwrap();
        Ok(Some(last_fetched))
    }

    pub async fn set_cw_last_fetched(&self, ts: DateTime) -> Result<()> {
        let mut redis = self.redis.get().await?;

        let ts = ts.timestamp();
        redis.hset(Self::ENTITY, Self::CW_LAST_FETCHED, ts).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct MarketDataRepository {
    redis: deadpool_redis::Pool,
}

impl MarketDataRepository {
    const ASSET_BASE: &'static str = concatcp!(REDIS_BASE, ':', "asset");
    const ASSET_IDX: &'static str = concatcp!(REDIS_BASE, ':', "assetIdx");

    const MARKET_BASE: &'static str = concatcp!(REDIS_BASE, ':', "market");
    const MARKET_IDX: &'static str = concatcp!(REDIS_BASE, ':', "marketIdx");

    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn find_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        let mut redis = self.redis.get().await?;

        let values = utils::find_json(&mut redis, Self::ASSET_IDX, "id", id).await?;
        Ok(values.into_iter().next())
    }

    pub async fn load_assets_by_type(&self, kind: AssetKind) -> Result<Vec<Asset>> {
        let mut redis = self.redis.get().await?;

        let kind = kind.to_string();
        utils::find_json(&mut redis, Self::ASSET_IDX, "type", &kind).await
    }

    pub async fn store_asset(&self, asset: &Asset) -> Result<()> {
        let mut redis = self.redis.get().await?;

        let id = format!("{}:{}", Self::ASSET_BASE, asset.id());
        utils::store_json(&mut redis, &id, asset, false).await?;

        Ok(())
    }

    pub async fn find_market(&self, id: &MarketId) -> Result<Option<Market>> {
        let mut redis = self.redis.get().await?;

        let market: Option<MarketDto> = utils::find_json(&mut redis, Self::MARKET_IDX, "id", id)
            .await?
            .into_iter()
            .next();

        if market.is_none() {
            return Ok(None);
        }

        let market = market.unwrap();
        self.resolve_market(market).await
    }

    async fn resolve_market(&self, market: MarketDto) -> Result<Option<Market>> {
        let (base, quote) = tokio::join!(
            self.find_asset(&market.base),
            self.find_asset(&market.quote)
        );

        let (base, quote) = (base?, quote?);
        if base.is_none() {
            error!(mkt = market.id, "Base asset not found: {}", &market.base);
            Ok(None)
        } else if quote.is_none() {
            error!(mkt = market.id, "Quote asset not found: {}", &market.quote);
            Ok(None)
        } else {
            let (base, quote) = (base.unwrap(), quote.unwrap());
            Ok(Some(Market::new(market.id, base, quote)))
        }
    }

    pub async fn find_markets(&self, ids: &[&MarketId]) -> Result<Vec<Option<Market>>> {
        let mut redis = self.redis.get().await?;

        let mut pipe = redis::pipe();
        pipe.atomic();
        ids.iter()
            .map(|id| {
                let mut cmd = redis::cmd("FT.SEARCH");
                cmd.arg(Self::MARKET_IDX).arg(format!("'@id:({})'", id));
                cmd
            })
            .for_each(|cmd| {
                pipe.add_command(cmd);
            });

        let res: Vec<Value> = pipe.query_async(&mut redis).await?;
        let dtos = res
            .into_iter()
            .zip(ids)
            .map(|(v, id)| {
                let res = v.as_sequence().unwrap();

                let count = u64::from_redis_value(&res[0]);
                if let Err(e) = count {
                    error!(
                        "Failed to parse 'count' for '{}' response '{:?}': {}",
                        id, v, e
                    );
                    return None;
                }
                if count.unwrap() == 0 {
                    return None;
                }

                let json = String::from_redis_value(&res[2].as_sequence().unwrap()[1]);
                if let Err(e) = json {
                    error!(
                        "Failed to collect JSON value for '{}' response '{:?}': {}",
                        id, v, e
                    );
                    return None;
                }

                let json = json.unwrap();
                let asset: Option<MarketDto> =
                    serde_json::from_str(&json).map(Some).unwrap_or_else(|_| {
                        let type_name = std::any::type_name::<MarketDto>();
                        error!(
                            "Failed to deserialize from JSON into {}: {}",
                            type_name, json
                        );
                        None
                    });

                asset
            })
            .collect::<Vec<_>>();

        // Resolve DTOs into Market domain object
        let markets = futures::stream::iter(dtos)
            .then(|m| async move {
                if let Some(m) = m {
                    self.resolve_market(m).await
                } else {
                    Ok(None)
                }
            })
            .inspect_err(|e| error!("Failed to resolve MarketDto: {}", e))
            .filter_map(|m| async move { m.ok() })
            .collect::<Vec<_>>()
            .await;

        Ok(markets)
    }

    pub async fn store_market(&self, market: &Market) -> Result<()> {
        let mut redis = self.redis.get().await?;

        let id = format!("{}:{}", Self::MARKET_BASE, market.id);
        let dto = MarketDto::from(market.clone());
        utils::store_json(&mut redis, &id, &dto, false).await?;

        Ok(())
    }

    pub async fn update_mkt_price(&self, market: &Market) -> Result<()> {
        if market.price().is_none() {
            return Ok(());
        }

        let mut redis = self.redis.get().await?;

        let id = format!("{}:{}", Self::MARKET_BASE, market.id);
        let price = market.price().as_ref().unwrap();
        let res = redis::pipe()
            .atomic()
            .cmd("JSON.SET")
            .arg(&id)
            .arg("$.price")
            .arg(price.price)
            .ignore()
            .cmd("JSON.SET")
            .arg(&id)
            .arg("$.ts")
            .arg(price.ts.timestamp())
            .ignore()
            .query_async::<_, Value>(&mut redis)
            .await?;

        if let Value::Bulk(v) = res {
            if v.is_empty() {
                return Ok(());
            }
        }

        Err(DcaError::RepositoryStoreFailure(format!(
            "Price for market '{}': {:?}",
            market.id, price
        )))
    }

    pub async fn load_markets(&self) -> Result<Vec<Market>> {
        let mut redis = self.redis.get().await?;

        let pattern = format!("{}:*", Self::MARKET_BASE);
        let ids: Vec<String> = redis.keys(&pattern).await?;

        let markets_json: Vec<String> = redis::cmd("JSON.MGET")
            .arg(&ids)
            .arg("$")
            .query_async(&mut redis)
            .await?;

        // Parse JSON into markets DTO
        let (markets, errors): (Vec<_>, Vec<_>) = markets_json
            .into_iter()
            .map(|s| s[1..s.len() - 1].to_owned())
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
                error!("Failed to parse JSON into Market: {}", e);
            });

        // Resolve DTOs into Market domain object
        let markets = futures::stream::iter(markets)
            .then(|m| async move { self.resolve_market(m).await })
            .inspect_err(|e| error!("Failed to resolve MarketDto: {}", e))
            .filter_map(|m| async move { m.ok() })
            .filter_map(futures::future::ready)
            .collect::<Vec<_>>()
            .await;

        Ok(markets)
    }

    pub async fn migrate_redis(&self) -> Result<()> {
        let mut redis = self.redis.get().await?;

        Self::migrate_asset_idx(&mut redis).await?;
        Self::migrate_market_idx(&mut redis).await
    }

    async fn migrate_asset_idx(redis: &mut Connection) -> Result<()> {
        let res = redis::cmd("FT.INFO")
            .arg(Self::ASSET_IDX)
            .query_async(redis)
            .await;

        if let Err(ref e) = res {
            if let redis::ErrorKind::ExtensionError = e.kind() {
                if e.detail().unwrap() == "Index name" {
                    info!("Missing index '{}'. Creating", Self::ASSET_IDX);

                    redis::cmd("FT.CREATE")
                        .arg(Self::ASSET_IDX)
                        .arg(&["ON", "JSON", "PREFIX", "1"])
                        .arg(MarketDataRepository::ASSET_BASE)
                        .arg(&[
                            "SCHEMA", "$.id", "AS", "id", "TEXT", "$.symbol", "as", "symbol",
                            "TEXT", "$.type", "AS", "type", "TEXT",
                        ])
                        .query_async::<_, ()>(redis)
                        .await?;

                    info!("Redis index '{}' created", Self::ASSET_IDX);
                    return Ok(());
                }
            }
        }

        let res = res?;
        if let Value::Bulk(v) = res {
            if let Some(s) = v.get(0) {
                match String::from_redis_value(s) {
                    Ok(s) if s == "index_name" => {
                        info!("Index '{}' already exists", Self::ASSET_IDX);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        Err(DcaError::Generic(format!(
            "Failed to check for index {}",
            Self::ASSET_IDX,
        )))
    }

    async fn migrate_market_idx(redis: &mut Connection) -> Result<()> {
        let res = redis::cmd("FT.INFO")
            .arg(Self::MARKET_IDX)
            .query_async(redis)
            .await;

        if let Err(ref e) = res {
            if let redis::ErrorKind::ExtensionError = e.kind() {
                if e.detail().unwrap() == "Index name" {
                    info!("Missing index '{}'. Creating", Self::MARKET_IDX);

                    redis::cmd("FT.CREATE")
                        .arg(Self::MARKET_IDX)
                        .arg(&["ON", "JSON", "PREFIX", "1"])
                        .arg(MarketDataRepository::MARKET_BASE)
                        .arg(&[
                            "SCHEMA", "$.id", "AS", "id", "TEXT", "$.base", "as", "base", "TEXT",
                            "$.quote", "as", "quote", "TEXT",
                        ])
                        .query_async::<_, ()>(redis)
                        .await?;

                    info!("Redis index '{}' created", Self::MARKET_IDX);
                    return Ok(());
                }
            }
        }

        let res = res?;
        if let Value::Bulk(v) = res {
            if let Some(s) = v.get(0) {
                match String::from_redis_value(s) {
                    Ok(s) if s == "index_name" => {
                        info!("Index '{}' already exists", Self::MARKET_IDX);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        Err(DcaError::Generic(format!(
            "Failed to check for index {}",
            Self::MARKET_IDX,
        )))
    }
}

#[derive(Clone)]
pub struct StatsRepository {
    redis: deadpool_redis::Pool,
}

impl StatsRepository {
    const STATS: &'static str = concatcp!(REDIS_BASE, ':', "stats");
    const VISITORS: &'static str = concatcp!(StatsRepository::STATS, ':', "visitors");
    const VISITOR_IP: &'static str = concatcp!(StatsRepository::STATS, ':', "visitor-ip");

    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn bump_visit(&self, ip: &str) -> Result<i64> {
        let mut redis = self.redis.get().await?;
        let count = redis::cmd("HINCRBY")
            .arg(&[Self::VISITORS, ip])
            .arg(1)
            .query_async(&mut redis)
            .await?;

        Ok(count)
    }

    pub async fn find_visitor_ip(&self, ip: &str) -> Result<Option<GeoData>> {
        let mut redis = self.redis.get().await?;
        let geo: Option<String> = redis.hget(Self::VISITOR_IP, ip).await?;

        if let Some(geo) = geo {
            Ok(Some(serde_json::from_str(&geo).unwrap()))
        } else {
            Ok(None)
        }
    }

    pub async fn store_visitor_ip(&self, ip: &str, geo: &GeoData) -> Result<bool> {
        let mut redis = self.redis.get().await?;

        let geo = serde_json::to_string(geo).unwrap();
        let n_records: u32 = redis.hset_nx(Self::VISITOR_IP, ip, geo).await?;

        Ok(n_records > 0)
    }
}
