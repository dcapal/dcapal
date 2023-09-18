use axum::async_trait;
use tracing::{debug, error};

use crate::{
    domain::entity::{Asset, AssetId, AssetKind},
    error::{DcaError, Result},
    repository::REDIS_BASE,
};

const ASSET_KEY: &str = concatcp!(REDIS_BASE, ':', "asset");
const ASSET_INDEX_TYPE: &str = concatcp!(ASSET_KEY, ":", "index", ":", "type");

#[async_trait]
pub trait RedisAsset {
    async fn store(&self, conn: &mut impl redis::AsyncCommands) -> Result<bool>;

    async fn find_by_id(
        id: &AssetId,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Option<Asset>>;

    async fn load_by_type(
        kind: AssetKind,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Vec<Asset>>;
}

#[async_trait]
impl RedisAsset for Asset {
    async fn store(&self, conn: &mut impl redis::AsyncCommands) -> Result<bool> {
        let json = serde_json::to_string(self).unwrap();
        redis::pipe()
            .atomic()
            .hset(ASSET_KEY, self.id(), &json)
            .zadd(self.kind().as_index(), self.id(), 0)
            .query_async(conn)
            .await?;

        debug!(
            "Successfully stored '{} {}': {}",
            ASSET_KEY,
            self.id(),
            json
        );

        Ok(true)
    }

    async fn find_by_id(
        id: &AssetId,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Option<Self>> {
        let json: Option<String> = conn.hget(ASSET_KEY, id).await?;
        let Some(json) = json else {
            return Ok(None);
        };

        let asset = serde_json::from_str(&json).map_err(|e| {
            DcaError::JsonDeserializationFailure(
                json,
                std::any::type_name::<Asset>().to_string(),
                e,
            )
        })?;

        Ok(Some(asset))
    }

    async fn load_by_type(
        kind: AssetKind,
        conn: &mut impl redis::AsyncCommands,
    ) -> Result<Vec<Asset>> {
        let ids: Vec<String> = conn.zrange(kind.as_index(), 0, -1).await?;

        let mut cmd = redis::pipe();
        cmd.atomic();

        for id in ids {
            cmd.hget(ASSET_KEY, id);
        }

        let jsons: Vec<String> = cmd.query_async(conn).await?;

        let assets = jsons
            .into_iter()
            .filter_map(|json| {
                let asset = serde_json::from_str(&json);
                match asset {
                    Ok(a) => Some(a),
                    Err(e) => {
                        let err = DcaError::JsonDeserializationFailure(
                            json,
                            std::any::type_name::<Asset>().to_string(),
                            e,
                        );
                        error!("{:?}", err);
                        None
                    }
                }
            })
            .collect();

        Ok(assets)
    }
}

trait AsRedisIndex {
    fn as_index(&self) -> &str;
}

impl AsRedisIndex for AssetKind {
    fn as_index(&self) -> &str {
        match self {
            AssetKind::Crypto => concatcp!(ASSET_INDEX_TYPE, ':', "crypto"),
            AssetKind::Fiat => concatcp!(ASSET_INDEX_TYPE, ':', "fiat"),
        }
    }
}
