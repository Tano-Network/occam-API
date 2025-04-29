use alloy_sol_types::sol;

sol! {
    /// The public values encoded as a struct that can be easily deserialized inside Solidity.
    struct PublicValuesStruct {
        uint32 n;
        uint32 a;
        uint32 b;
        uint32 icr;
        uint32 collateral_amount;
    }
}

/// Compute the n'th fibonacci number (wrapping around on overflows), using normal Rust code.
pub fn fibonacci(n: u32) -> (u32, u32) {
    let mut a = 0u32;
    let mut b = 1u32;
    for _ in 0..n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    (a, b)
}

/// Calculate the ICR and USD value of the collateral.
/// `btc_price_usd` should be passed as an argument.
pub fn calculate_icr(
    collateral_amount: u32,
    debt_amount: u32,
    btc_price_usd: u32,
) -> (u32, u32) {
    let collateral_amount_to_usd = collateral_amount * btc_price_usd;
    
    let icr = if debt_amount == 0 {
        0
    } else {
        collateral_amount_to_usd / debt_amount
    };

    (icr, collateral_amount_to_usd)
}