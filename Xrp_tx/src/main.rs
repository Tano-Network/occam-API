#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{SolType, private::FixedBytes}; // <-- import SolType here
use alloy_primitives::{hex, Address};
use fibonacci_lib::{PublicValuesXrpTx, XrpTxInput};
use sha2::{Digest, Sha256};
use bs58; // Base58 decoding for XRP addresses

pub fn main() {
    let input: XrpTxInput = sp1_zkvm::io::read();

    // Verify recipient address (hardcoded for simplicity)
    let expected_recipient = "rpJRDWw1M9jm7NRgrrSvHLJFWA7CaeaWuw";
    assert_eq!(input.recipient_address, expected_recipient, "Recipient address mismatch");

    // Hash the sender address (keep SHA256 hash)
    let sender_hash_digest = Sha256::digest(input.sender_address.as_bytes());
    let sender_hash_array: [u8; 32] = sender_hash_digest.into();
    let sender_fixed_hash = FixedBytes::<32>::from(sender_hash_array);

    // Decode XRP base58 address
    let decoded = bs58::decode(&input.owner_address)
        .into_vec()
        .expect("failed to decode base58 address");

    // Ethereum-style `Address` must be exactly 20 bytes
    // XRP Base58 addresses are longer, so we take the last 20 bytes
    let owner_address = Address::from_slice(&decoded[decoded.len().saturating_sub(20)..]);

    // Transaction hash
    let tx_hash_bytes = hex::decode(input.tx_hash).expect("Invalid tx_hash format");
    let txid_bytes: [u8; 32] = tx_hash_bytes
        .as_slice()
        .try_into()
        .expect("tx_hash must be 32 bytes long");

    let public_values = PublicValuesXrpTx {
        total_xrp: input.amount,
        sender_address_hash: sender_fixed_hash,
        owner_address,
        tx_hash: FixedBytes::<32>::from(txid_bytes),
    };

    // Now `abi_encode` works since `SolType` is in scope
    let bytes = PublicValuesXrpTx::abi_encode(&public_values);
    sp1_zkvm::io::commit_slice(&bytes);
}
