//! A simple program that takes a number n as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use fibonacci_lib::{fibonacci, calculate_icr, PublicValuesStruct};

pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles reading inputs
    // from the prover.
    let n = sp1_zkvm::io::read::<u32>();//for fibonacci

    let collateral_amount = sp1_zkvm::io::read::<u32>();//for icr 
    let debt_amount = sp1_zkvm::io::read::<u32>();//for icr 
    let btc_price_usd = sp1_zkvm::io::read::<u32>();// <-- added to support BTC price input

    // Compute the n'th fibonacci number using a function from the workspace lib crate.
    let (a, b) = fibonacci(n);

    let (icr, collateral_amount) = calculate_icr(collateral_amount, debt_amount, btc_price_usd); // <-- updated to use btc_price_usd

    // Encode the public values of the program.
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct { n, a, b, icr, collateral_amount });

    // Commit to the public values of the program. The final proof will have a commitment to all the
    // bytes that were committed to.
    sp1_zkvm::io::commit_slice(&bytes);
}