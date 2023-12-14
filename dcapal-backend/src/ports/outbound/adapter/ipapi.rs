use serde::{Deserialize, Serialize};

use crate::{config, error::Result};

#[derive(Clone)]
pub struct IpApi {
    http: reqwest::Client,
    api_key: String,
}

impl IpApi {
    pub fn new(http: reqwest::Client, config: &config::Providers) -> Self {
        Self {
            http,
            api_key: config.ip_api_key.clone(),
        }
    }

    pub async fn fetch_geo(&self, ip: &str) -> Result<Option<GeoData>> {
        let api_key = &self.api_key;
        let url = format!("http://api.ipapi.com/{ip}?access_key={api_key}&format=1");

        let res = self.http.get(url).send().await?;
        if res.status().is_success() {
            let res = res.json::<res::IpApiResult>().await?;
            match res {
                res::IpApiResult::GeoIP(geo) => return Ok(Some(geo.into())),
                res::IpApiResult::Error(_) => return Ok(None),
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoData {
    pub ip: String,
    pub continent_code: String,
    pub continent_name: String,
    pub country_code: String,
    pub country_name: String,
    pub region_name: String,
    pub city: String,
    pub zip: String,
    pub latitude: String,
    pub longitude: String,
}

impl From<res::GeoIp> for GeoData {
    fn from(geo: res::GeoIp) -> Self {
        Self {
            ip: geo.ip,
            continent_code: geo.continent_code.unwrap_or_default(),
            continent_name: geo.continent_name.unwrap_or_default(),
            country_code: geo.country_code.unwrap_or_default(),
            country_name: geo.country_name.unwrap_or_default(),
            region_name: geo.region_name.unwrap_or_default(),
            city: geo.city.unwrap_or_default(),
            zip: geo.zip.unwrap_or_default(),
            latitude: geo.latitude.unwrap_or_default().to_string(),
            longitude: geo.longitude.unwrap_or_default().to_string(),
        }
    }
}

mod res {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    #[serde(untagged)]
    #[allow(clippy::large_enum_variant)]
    pub enum IpApiResult {
        GeoIP(GeoIp),
        Error(Error),
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct GeoIp {
        pub ip: String,
        pub continent_code: Option<String>,
        pub continent_name: Option<String>,
        pub country_code: Option<String>,
        pub country_name: Option<String>,
        pub region_name: Option<String>,
        pub city: Option<String>,
        pub zip: Option<String>,
        pub latitude: Option<f64>,
        pub longitude: Option<f64>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[allow(dead_code)]
    pub struct Error {
        success: bool,
    }
}
