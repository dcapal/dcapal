//! The [`repository`](self) module contains interfaces to persistent storage services, like Redis.

use std::collections::HashMap;
use std::fmt::Display;

use chrono::{TimeZone, Utc};
use redis::AsyncCommands;
use uuid::Uuid;

use crate::{app::services::ip2location::GeoData, error::Result, DateTime};
pub mod dto;
pub mod market_data;
pub mod portfolio;
pub mod user;

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
        let _: () = redis.hset(Self::ENTITY, Self::CW_LAST_FETCHED, ts).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct StatsRepository {
    redis: deadpool_redis::Pool,
}

impl StatsRepository {
    const STATS: &'static str = concatcp!(REDIS_BASE, ':', "stats");
    const IMPORTED_PORTFOLIO_KEY: &'static str = "imported-portfolio-total";

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

    pub async fn increase_imported_portfolio_count(&self) -> Result<i64> {
        let mut redis = self.redis.get().await?;

        let count: i64 = redis
            .hincr(Self::STATS, Self::IMPORTED_PORTFOLIO_KEY, 1)
            .await?;

        Ok(count)
    }

    pub async fn get_imported_portfolio_count(&self) -> Result<i64> {
        let mut redis = self.redis.get().await?;

        let count: Option<i64> = redis
            .hget(Self::STATS, Self::IMPORTED_PORTFOLIO_KEY)
            .await?;

        Ok(count.unwrap_or_default())
    }
}

#[derive(Clone)]
pub struct ImportedRepository {
    redis: deadpool_redis::Pool,
}

pub struct ImportedPortfolio {
    pub id: Uuid,
    pub expires_at: DateTime,
}

impl ImportedRepository {
    const IMPORTED: &'static str = concatcp!(REDIS_BASE, ':', "imported");

    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn store_portfolio(&self, pfolio: &serde_json::Value) -> Result<ImportedPortfolio> {
        let mut redis = self.redis.get().await?;

        let id = Uuid::new_v4();
        let key = Self::redis_imported_key(id.simple());
        let value = serde_json::to_string(&pfolio).unwrap();

        let _: () = redis.set_ex(&key, value, 60).await?;

        let expires_at: i64 = redis::cmd("EXPIRETIME")
            .arg(&key)
            .query_async(&mut redis)
            .await?;

        let expires_at = DateTime::from_timestamp(expires_at, 0).unwrap_or_else(Utc::now);

        Ok(ImportedPortfolio { id, expires_at })
    }

    pub async fn find_portfolio(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let mut redis = self.redis.get().await?;

        let key = Self::redis_imported_key(id);
        let portfolio: Option<String> = redis.get(key).await?;
        Ok(portfolio.map(|s| serde_json::from_str(&s).unwrap()))
    }

    fn redis_imported_key(id: impl Display) -> String {
        format!("{}:{}", Self::IMPORTED, id)
    }
}
