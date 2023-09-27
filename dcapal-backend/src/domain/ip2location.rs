use std::{net::IpAddr, path::Path};

use ip2location::{LocationDB, LocationRecord};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing_log::log::error;

use crate::error::Result;

pub struct Ip2LocationService {
    db: Mutex<LocationDB>,
}

impl Ip2LocationService {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = LocationDB::from_file_mmap(path)?;

        Ok(Self { db: Mutex::new(db) })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<GeoData> {
        let record = {
            let mut db = self.db.lock();
            db.ip_lookup(ip)
        };

        match record {
            Ok(r) => Some(r.into()),
            Err(e) => match e {
                ip2location::error::Error::RecordNotFound => None,
                _ => {
                    error!("Failed to lookup IP {ip}: {e:?}");
                    None
                }
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoData {
    pub ip: String,
    pub latitude: String,
    pub longitude: String,
}

impl From<LocationRecord> for GeoData {
    fn from(r: LocationRecord) -> Self {
        Self {
            ip: r.ip.to_string(),
            latitude: r.latitude.unwrap_or_default().to_string(),
            longitude: r.longitude.unwrap_or_default().to_string(),
        }
    }
}
