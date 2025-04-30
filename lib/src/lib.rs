use alloy_sol_types::sol;

sol! {
    /// The public values encoded as a struct that can be easily deserialized inside Solidity.
    struct PublicValuesStruct {
        uint32 n;
        uint32 a;
        uint32 b;
        uint32 icr;
        uint32 collateral_amount;
        uint32 liquidation_threshold;
        uint32 real_time_ltv;
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
pub fn calculate_icr(
    collateral_amount: u32,
    debt_amount: u32,
    btc_price_usd: u32,
) -> (u32, u32) {
    let collateral_amount_to_usd = (collateral_amount as u64 * btc_price_usd as u64) as u32;

    let icr = if debt_amount == 0 {
        0
    } else {
        ((collateral_amount as u64 * btc_price_usd as u64) / debt_amount as u64) as u32
    };

    (icr, collateral_amount_to_usd)
}

/// Calculate the liquidation threshold from collateral value and ICR.
pub fn calculate_liquidation_threshold(
    collateral_amount: u32,
    btc_price_usd: u32,
    icr: u32,
) -> u32 {
    if icr == 0 {
        return 0;
    }

    let collateral_value = collateral_amount as u64 * btc_price_usd as u64;
    (collateral_value / icr as u64) as u32
}

/// Calculate real-time LTV (Loan-to-Value ratio).
pub fn real_time_ltv(
    loan_usbd: u32,
    btc_balance: u32,
    btc_price_usd: u32,
) -> u32 {
    if btc_balance == 0 || btc_price_usd == 0 {
        return 0;
    }

    let collateral_value = btc_balance as u64 * btc_price_usd as u64;
    ((loan_usbd as u64 * 100) / collateral_value) as u32  // returns percentage
}