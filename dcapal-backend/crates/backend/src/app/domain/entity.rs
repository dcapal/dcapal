use chrono::{Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{DateTime, app::infra::utils::Expiring};

/// Stable identifier used for a supported asset.
pub type AssetId = String;

/// Stable identifier used for a tradable base/quote market pair.
pub type MarketId = String;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
/// A cryptocurrency identified by its provider id and display symbol.
pub struct Crypto {
    pub id: AssetId,
    pub symbol: String,
}

impl Crypto {
    /// Creates a cryptocurrency whose display symbol is the upper-case id.
    pub fn new_with_id(id: AssetId) -> Self {
        let symbol = id.to_uppercase();
        Self { id, symbol }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
/// A fiat currency identified by its provider id and display symbol.
pub struct Fiat {
    pub id: AssetId,
    pub symbol: String,
}

impl Fiat {
    /// Creates a fiat asset with the supplied provider id and symbol.
    pub fn new(id: AssetId, symbol: String) -> Self {
        Self { id, symbol }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type")]
/// A supported asset, either a cryptocurrency or a fiat currency.
pub enum Asset {
    Crypto(Crypto),
    Fiat(Fiat),
}

#[derive(Debug, Clone, Copy, strum_macros::Display)]
/// The broad asset category used when selecting market-data catalogs.
pub enum AssetKind {
    Crypto,
    Fiat,
}

impl Asset {
    /// Returns the provider id shared by both asset variants.
    pub fn id(&self) -> &AssetId {
        match self {
            Asset::Crypto(a) => &a.id,
            Asset::Fiat(a) => &a.id,
        }
    }

    /// Returns whether this asset represents a fiat currency.
    pub fn is_fiat(&self) -> bool {
        matches!(self, Asset::Fiat(_))
    }

    /// Returns the category used by market-data catalog queries.
    pub fn kind(&self) -> AssetKind {
        match self {
            Asset::Crypto(_) => AssetKind::Crypto,
            Asset::Fiat(_) => AssetKind::Fiat,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// A price and the timestamp at which it was observed.
pub struct Price {
    pub price: f64,
    /// Serialized as Unix seconds in the REST/OpenAPI representation.
    #[schema(value_type = i64, format = Int64)]
    #[serde(with = "chrono::serde::ts_seconds")]
    pub ts: DateTime,
}

impl Price {
    const VALIDITY_MINS: u32 = 5;

    /// Creates a price observation with its source timestamp.
    pub fn new(price: f64, ts: DateTime) -> Self {
        Self { price, ts }
    }
}

impl Expiring for Price {
    fn is_outdated(&self) -> bool {
        let now = Utc::now();
        if now.date_naive() > self.ts.date_naive() || now.time().hour() > self.ts.time().hour() {
            return true;
        }

        let now_mins = now.time().minute();
        let ts_mins = self.ts.time().minute();

        let now_validity_range = (now_mins / Self::VALIDITY_MINS) * Self::VALIDITY_MINS;
        let ts_validity_range = (ts_mins / Self::VALIDITY_MINS) * Self::VALIDITY_MINS;

        now_validity_range > ts_validity_range
    }

    fn time_to_live(&self) -> std::time::Duration {
        (self.ts + Duration::minutes(Self::VALIDITY_MINS as i64) - Utc::now())
            .to_std()
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
    }
}

#[derive(Debug, Clone, Copy, strum_macros::Display)]
/// Supported intervals for historical OHLC market-data queries.
pub enum OHLCFrequency {
    Minutes5,
    Daily,
}

impl OHLCFrequency {
    /// Returns the historical range needed to cover the requested interval.
    pub fn ohlc_range(&self, ts: DateTime) -> (DateTime, DateTime) {
        match self {
            OHLCFrequency::Minutes5 => Self::ohlc_range_minutes_5(ts),
            OHLCFrequency::Daily => Self::ohlc_range_daily(ts),
        }
    }

    fn ohlc_range_minutes_5(ts: DateTime) -> (DateTime, DateTime) {
        static MINS_5: u32 = 5;
        static N_PERIODS: i64 = 12;

        let ts_mins = ts.time().minute();
        let range_upper_mins = (ts_mins / MINS_5) * MINS_5;
        let range_upper = ts
            .with_minute(range_upper_mins)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        let range_low = range_upper - Duration::minutes(N_PERIODS * MINS_5 as i64);

        (range_low, range_upper)
    }

    fn ohlc_range_daily(ts: DateTime) -> (DateTime, DateTime) {
        static N_DAYS: i64 = 1;

        let ts_start_day = ts
            .with_hour(0)
            .and_then(|t| t.with_minute(0))
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap();

        let range_low = ts_start_day - Duration::days(N_DAYS);

        (range_low, ts)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// A base/quote market with an optional cached price observation.
pub struct Market {
    pub id: MarketId,
    pub pair: String,
    pub base: Asset,
    pub quote: Asset,
    #[serde(flatten)]
    price: Option<Price>,
}

impl Market {
    /// Creates a market and derives its display pair from the asset ids.
    pub fn new(id: MarketId, base: Asset, quote: Asset, price: Option<Price>) -> Self {
        Self {
            id,
            pair: format!("{}/{}", base.id().to_uppercase(), quote.id().to_uppercase()),
            base,
            quote,
            price,
        }
    }

    /// Returns the latest cached price, if one is available.
    pub fn price(&self) -> &Option<Price> {
        &self.price
    }

    /// Replaces the cached price observation.
    pub fn set_price(&mut self, price: Price) {
        self.price.replace(price);
    }

    /// Returns whether both sides of the market are fiat currencies.
    pub fn is_fiat(&self) -> bool {
        self.base.is_fiat() && self.quote.is_fiat()
    }

    /// Returns whether the cached price has crossed its five-minute validity window.
    pub fn is_price_outdated(&self) -> bool {
        let last_price = self.price();
        last_price.is_some() && last_price.as_ref().unwrap().is_outdated()
    }
}

impl Expiring for Market {
    fn is_outdated(&self) -> bool {
        self.is_price_outdated()
    }

    fn time_to_live(&self) -> std::time::Duration {
        self.price()
            .as_ref()
            .map(|p| p.time_to_live())
            .unwrap_or_else(|| std::time::Duration::from_secs(0))
    }
}
