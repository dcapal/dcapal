use nalgebra::{DMatrix, DVector};
use rust_decimal::Decimal;

pub struct ProblemAsset {
    pub symbol: String,
    pub qty: Decimal,
    pub price: Decimal,
    pub current_weight: Decimal,
    pub target_weight: Decimal,
    pub current_amount: Decimal,
}

//to retrieve n I should iterate over the assets and remove those with current_weight > target_weight

fn void() {
    let mut dm = DMatrix::<f64>::zeros(2, 2);
    dm.row_mut(0)
        .copy_from_slice(&[0.8 * 102.72 - 102.72, 27.99 * 0.8]);
    dm.row_mut(1)
        .copy_from_slice(&[102.72 * 0.1, 0.1 * 27.99 - 27.99]);

    let mut b = DVector::<f64>::zeros(2);
    b.copy_from_slice(&[
        1.00 * 102.72 - 0.8 * 300.00 * 4.56 - 0.8 * 1.00 * 102.72 - 0.8 * 5.00 * 27.99,
        5.00 * 27.99 - 0.1 * 300.00 * 4.56 - 0.1 * 1.00 * 102.72 - 0.1 * 5.00 * 27.99,
    ]);

    let decomp = dm.lu();
    let x = decomp.solve(&b).expect("Linear resolution failed.");
    println!("Solution: {:?}", x);
}
