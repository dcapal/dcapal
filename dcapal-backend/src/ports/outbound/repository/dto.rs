use crate::app::domain::entity::{AssetId, Market, MarketId, Price};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketDto {
    pub id: MarketId,
    pub pair: String,
    pub base: AssetId,
    pub quote: AssetId,
    #[serde(flatten)]
    pub price: Option<Price>,
}

impl MarketDto {}

impl From<Market> for MarketDto {
    fn from(m: Market) -> Self {
        let (base, quote) = (m.base.id().clone(), m.quote.id().clone());
        let price = *m.price();
        Self {
            id: m.id,
            pair: m.pair,
            base,
            quote,
            price,
        }
    }
}
