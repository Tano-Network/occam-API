//! A simple program that takes a number n as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use fibonacci_lib::{
    fibonacci,
    calculate_icr,
    calculate_liquidation_threshold,
    real_time_ltv,
    PublicValuesStruct,
};

pub fn main() {
    // Read inputs
    let n = sp1_zkvm::io::read::<u32>();               // Fibonacci
    let collateral_amount = sp1_zkvm::io::read::<u32>(); // BTC collateral (in sats or units)
    let debt_amount = sp1_zkvm::io::read::<u32>();     // Loan/debt amount
    let btc_price_usd = sp1_zkvm::io::read::<u32>();   // BTC price in USD
    let usbd_loan = sp1_zkvm::io::read::<u32>();       // Loan value for LTV
    let btc_balance = sp1_zkvm::io::read::<u32>();     // BTC balance for LTV

    // Compute Fibonacci
    let (a, b) = fibonacci(n);

    // Compute ICR (and get USD collateral for logging)
    let (icr, collateral_usd_value) = calculate_icr(collateral_amount, debt_amount, btc_price_usd);

    // Use original BTC collateral to avoid double-scaling in threshold
    let liquidation_threshold = calculate_liquidation_threshold(collateral_amount, btc_price_usd, icr);

    // Compute real-time LTV
    let real_time_ltv = real_time_ltv(usbd_loan, btc_balance, btc_price_usd);

    // Encode and commit public values
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct {
        n,
        a,
        b,
        icr,
        collateral_amount: collateral_usd_value, // Save the USD-scaled value
        liquidation_threshold,
        real_time_ltv,
    });

    sp1_zkvm::io::commit_slice(&bytes);
}