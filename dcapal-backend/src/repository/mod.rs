pub mod dto;
pub mod market_data;

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use redis::AsyncCommands;

use crate::{domain::ip2location::GeoData, error::Result, DateTime};

const REDIS_BASE: &str = "dcapal:be";

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

    pub async fn fetch_all_visitors(&self) -> Result<HashMap<String, i32>> {
        let mut redis = self.redis.get().await?;

        Ok(redis.hgetall(Self::VISITORS).await?)
    }
}
