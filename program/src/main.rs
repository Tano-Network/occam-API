//! A simple program that takes a number n as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
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
    let n = sp1_zkvm::io::read::<u32>(); // for fibonacci
    let collateral_amount = sp1_zkvm::io::read::<u32>(); // for ICR
    let debt_amount = sp1_zkvm::io::read::<u32>(); // for ICR
    let btc_price_usd = sp1_zkvm::io::read::<u32>(); // <-- added input for BTC price
    let usbd_loan = sp1_zkvm::io::read::<u32>(); // for liquidation threshold
    let btc_balance = sp1_zkvm::io::read::<u32>(); // for real-time LTV

    // Compute Fibonacci
    let (a, b) = fibonacci(n);

    // Compute ICR using BTC price
    let (icr, adjusted_collateral) = calculate_icr(collateral_amount, debt_amount, btc_price_usd);

    // Compute liquidation threshold from adjusted collateral
    let liquidation_threshold = calculate_liquidation_threshold(adjusted_collateral,btc_price_usd,icr);

    // Compute real-time LTV using loan and BTC balance
    let real_time_ltv = real_time_ltv(usbd_loan, btc_balance,btc_price_usd);

    // Encode and commit public values
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct {
        n,
        a,
        b,
        icr,
        collateral_amount: adjusted_collateral,
        liquidation_threshold,
        real_time_ltv,
    });

    sp1_zkvm::io::commit_slice(&bytes);
}