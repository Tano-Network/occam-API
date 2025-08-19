#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use alloy_sol_types::private::FixedBytes;
use alloy_primitives::hex;
use fibonacci_lib::{PublicValuesDogeTx, DogeTxInput};
use sha2::{Digest, Sha256};

pub fn main() {
    let input: DogeTxInput = sp1_zkvm::io::read();

    // Verify recipient address (hardcoded for simplicity)
    let expected_recipient = "DHGrS3MYGyKzRVdMNxziTPF7QXvaYoEndA";
    assert_eq!(input.recipient_address, expected_recipient, "Recipient address mismatch");

    // Hash the sender address
    let sender_hash_digest = Sha256::digest(input.sender_address.as_bytes());
    let sender_hash_array: [u8; 32] = sender_hash_digest.into();
    let fixed_hash = FixedBytes::<32>::from(sender_hash_array);

    //hash the owner address
    let owner_hash_digest = Sha256::digest(input.owner_address.as_bytes());
    let owner_hash_array: [u8; 32] = owner_hash_digest.into();
    let owner_fixed_hash = FixedBytes::<32>::from(owner_hash_array);

    //trandaction hash
    let tx_hash_bytes = hex::decode(input.tx_hash).expect("Invalid tx_hash format");
    let txid_bytes: [u8; 32] = tx_hash_bytes
        .as_slice()
        .try_into()
        .expect("tx_hash must be 32 bytes long");

    let public_values = PublicValuesDogeTx {
        total_doge: input.amount,
        sender_address_hash: fixed_hash,
        owner_address: owner_fixed_hash,
        tx_hash: FixedBytes::<32>::from(txid_bytes),
    };

    let bytes = PublicValuesDogeTx::abi_encode(&public_values);
    sp1_zkvm::io::commit_slice(&bytes);

}

