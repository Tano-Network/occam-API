#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use alloy_sol_types::private::FixedBytes;
use fibonacci_lib::{PublicValuesDogeTx, DogeTxInput};
use sha2::{Digest, Sha256};

pub fn main() {
    let input: DogeTxInput = sp1_zkvm::io::read();

    // Verify recipient address (hardcoded for simplicity)
    let expected_recipient = "DPGGRKJaKtTkNhc6uodtdyQEyv8RsWxL6H";
    assert_eq!(input.recipient_address, expected_recipient, "Recipient address mismatch");

    // Hash the sender address
    let sender_hash_digest = Sha256::digest(input.sender_address.as_bytes());
    let sender_hash_array: [u8; 32] = sender_hash_digest.into();
    let fixed_hash = FixedBytes::<32>::from(sender_hash_array);

    let public_values = PublicValuesDogeTx {
        total_doge: input.amount,
        sender_address_hash: fixed_hash,
    };

    let bytes = PublicValuesDogeTx::abi_encode(&public_values);
    sp1_zkvm::io::commit_slice(&bytes);

}

