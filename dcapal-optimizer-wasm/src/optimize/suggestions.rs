extern crate nalgebra as na;

use std::cmp::Ordering;

pub struct ProblemAsset {
    pub symbol: String,
    pub qty: f64,
    pub price: f64,
    pub current_weight: f64,
    pub target_weight: f64,
}

pub fn calculate_allocation_amount(assets: Vec<ProblemAsset>) -> f64 {
    let overweight_assets = assets
        .iter()
        .filter(|a| a.current_weight > a.target_weight)
        .collect::<Vec<&ProblemAsset>>();

    let max_asset = overweight_assets.iter().max_by(|a, b| {
        a.current_weight
            .partial_cmp(&b.current_weight)
            .unwrap_or(Ordering::Equal)
    });

    let total_current_value = assets
        .iter()
        .fold(0.0, |acc, asset| acc + asset.qty * asset.price);

    match max_asset {
        Some(asset) => {
            (asset.price * asset.qty) * (100.0 / (asset.target_weight * 100.0))
                - total_current_value
        }
        None => 0.00,
    }
}
