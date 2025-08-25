#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{SolType, private::FixedBytes};
use alloy_primitives::hex;
use fibonacci_lib::{PublicValuesXrpBalance, XrpBalanceInput};
use sha2::{Digest, Sha256};

pub fn main() {

    let input: XrpBalanceInput = sp1_zkvm::io::read();
    let address = input.address.clone();
    let public_values = PublicValuesXrpBalance {
        total_xrp: input.amount,
        address: address, // Keep as string
    };


    let bytes = PublicValuesXrpBalance::abi_encode(&public_values);
    sp1_zkvm::io::commit_slice(&bytes);
}
