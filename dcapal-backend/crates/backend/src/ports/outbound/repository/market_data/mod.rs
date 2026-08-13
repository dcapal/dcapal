mod redis_asset;
mod redis_market;

use self::{redis_asset::RedisAsset, redis_market::RedisMarket};
use crate::{
    app::domain::entity::{Asset, AssetId, AssetKind, Market, MarketId},
    error::{DcaError, Result},
};

#[derive(Clone)]
pub struct MarketDataRepository {
    redis: deadpool_redis::Pool,
}

impl MarketDataRepository {
    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn find_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        let mut redis = self.redis.get().await?;

        Asset::find_by_id(id, &mut redis).await
    }

    pub async fn load_assets_by_type(&self, kind: AssetKind) -> Result<Vec<Asset>> {
        let mut redis = self.redis.get().await?;

        Asset::load_by_type(kind, &mut redis).await
    }

    pub async fn store_asset(&self, asset: &Asset) -> Result<()> {
        let mut redis = self.redis.get().await?;

        asset.store(&mut redis).await?;

        Ok(())
    }

    pub async fn find_market(&self, id: &MarketId) -> Result<Option<Market>> {
        let mut redis = self.redis.get().await?;

        Market::find_by_id(id, &mut redis, self).await
    }

    pub async fn find_markets(&self, ids: &[&MarketId]) -> Result<Vec<Option<Market>>> {
        let mut redis = self.redis.get().await?;

        Market::find_by_ids(ids, &mut redis, self).await
    }

    pub async fn store_market(&self, market: &Market) -> Result<()> {
        let mut redis = self.redis.get().await?;

        if market.store(&mut redis).await? {
            Ok(())
        } else {
            Err(DcaError::RepositoryStoreFailure(market.id.clone()))
        }
    }

    pub async fn update_mkt_price(&self, market: &Market) -> Result<()> {
        if market.price().is_none() {
            return Ok(());
        }

        let mut redis = self.redis.get().await?;

        market.store(&mut redis).await?;

        Ok(())
    }

    pub async fn load_markets(&self) -> Result<Vec<Market>> {
        let mut redis = self.redis.get().await?;

        Market::load_all(&mut redis, self).await
    }
}
