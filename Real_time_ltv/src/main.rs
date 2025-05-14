#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolValue;
use fibonacci_lib::{PublicValuesLtv, real_time_ltv};
use hex;

pub fn main() {
    // Read inputs in the order written by evm.rs
    let debt_amount = sp1_zkvm::io::read::<u32>();       // Debt amount (in USD, scaled)
    let collateral_amount = sp1_zkvm::io::read::<u32>(); // BTC collateral (in units, e.g., scaled)
    let btc_price_usd = sp1_zkvm::io::read::<u32>();     // BTC price in USD

    eprintln!("Inputs: debt_amount={}, collateral_amount={}, btc_price_usd={}", 
             debt_amount, collateral_amount, btc_price_usd);

    // Compute real-time LTV
    let real_time_ltv = real_time_ltv(debt_amount, collateral_amount, btc_price_usd);
    eprintln!("Computed LTV: {}", real_time_ltv);

    // Encode public values into PublicValuesLtv
    let public_values = PublicValuesLtv {
        real_time_ltv,
    };

    // ABI-encode the public values
    let bytes = public_values.abi_encode();
    eprintln!("Encoded public values: 0x{}", hex::encode(&bytes));

    // Commit the encoded public values to the zkVM
    sp1_zkvm::io::commit_slice(&bytes);
}