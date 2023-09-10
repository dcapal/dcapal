extern crate nalgebra as na;

use nalgebra::{DMatrix, DVector};

pub struct ProblemAsset {
    pub symbol: String,
    pub qty: f64,
    pub price: f64,
    pub current_weight: f64,
    pub target_weight: f64,
}

fn main() {
    let etf1 = ProblemAsset {
        symbol: "AAPL".to_string(),
        qty: 1.00,
        price: 102.72,
        current_weight: 0.064,
        target_weight: 0.8,
    };
    let etf2 = ProblemAsset {
        symbol: "MSFT".to_string(),
        qty: 5.00,
        price: 27.99,
        current_weight: 0.087,
        target_weight: 0.1,
    };
    let etf3 = ProblemAsset {
        symbol: "AMZN".to_string(),
        qty: 300.00,
        price: 4.56,
        current_weight: 0.849,
        target_weight: 0.1,
    };
    calculate_allocation_amount(vec![etf1, etf2, etf3]);
}

pub fn calculate_allocation_amount(assets: Vec<ProblemAsset>) {
    let (buy, keep): (Vec<_>, Vec<_>) = assets.iter().partition(|a| a.current_weight < a.target_weight);

    let n = buy.len();
    let mut covariance_matrix = DMatrix::<f64>::zeros(n, n);
    let mut rhs_vector = DVector::<f64>::zeros(n);

    let combined_assets: Vec<_> = buy.iter().chain(keep.iter()).cloned().collect();

    for (i, buy_asset) in buy.iter().enumerate() {
        // Compute the right-hand side vector 'b'
        rhs_vector[i] = buy_asset.price * buy_asset.qty;

        for (j, asset) in combined_assets.iter().enumerate() {
            if asset.current_weight < asset.target_weight {
                if i == j {
                    // Diagonal elements of the covariance matrix
                    covariance_matrix[(i, j)] = asset.price * asset.target_weight - asset.price;
                } else {
                    // Off-diagonal elements of the covariance matrix
                    covariance_matrix[(i, j)] = asset.price * buy_asset.target_weight;
                }
            }
            rhs_vector[i] -= asset.price * asset.qty * buy_asset.target_weight;
        }
    }

    // Perform LU decomposition and solve for 'x'
    let decomposition = covariance_matrix.lu();
    let allocation_amount = decomposition.solve(&rhs_vector).expect("Linear resolution failed.");

    println!("Allocation Amounts: {:?}", allocation_amount);

    // Calculate the total amount needed to invest for reaching the target allocation
    let sum_product = buy.iter().zip(allocation_amount.iter()).fold(0.0, |acc, (asset, x)| {
        acc + (asset.price * x)
    });

    println!("Total Investment for 'buy' assets: {}", sum_product);
}
