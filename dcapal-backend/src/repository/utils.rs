use redis::{FromRedisValue, Value};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error};

use crate::error::{DcaError, Result};

pub async fn store_json<C, T>(con: &mut C, id: &str, value: &T, replace: bool) -> Result<bool>
where
    T: ?Sized + Serialize,
    C: redis::aio::ConnectionLike,
{
    let json = serde_json::to_string(value).unwrap();
    let mut cmd = redis::cmd("JSON.SET");
    cmd.arg(id).arg("$").arg(&json);

    if !replace {
        cmd.arg("NX");
    }

    let res = cmd.query_async::<_, Value>(con).await?;

    if matches!(res, Value::Okay) {
        debug!("Successfully stored '{}': {}", id, json);
        return Ok(true);
    }

    if !replace && matches!(res, Value::Nil) {
        debug!("Not stored. Aready existing key '{}'", id);
        return Ok(false);
    }

    Err(DcaError::RepositoryStoreFailure(id.to_string()))
}

pub async fn find_json<C, T>(con: &mut C, idx: &str, field: &str, value: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
    C: redis::aio::ConnectionLike,
{
    let res = redis::cmd("FT.SEARCH")
        .arg(idx)
        .arg(format!("'@{}:({})'", field, value))
        .arg("LIMIT")
        .arg(&[0u32, 10_000])
        .query_async::<_, Value>(con)
        .await?;
    let res = res.as_sequence().unwrap();

    let count = u64::from_redis_value(&res[0])?;
    if count == 0 {
        return Ok(vec![]);
    }

    let values = (2..res.len())
        .step_by(2)
        .filter_map(|idx| {
            let record = &res[idx].as_sequence().unwrap()[1];
            let json = String::from_redis_value(record);
            match json {
                Err(e) => {
                    error!("{}: {:?}", e, record);
                    None
                }
                Ok(json) => serde_json::from_str(&json).map(Some).unwrap_or_else(|_| {
                    let type_name = std::any::type_name::<T>();
                    error!(
                        "Failed to deserialize from JSON into {}: {}",
                        type_name, json
                    );
                    None
                }),
            }
        })
        .collect();

    Ok(values)
}
