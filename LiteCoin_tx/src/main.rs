#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{SolType, private::FixedBytes};
use alloy_primitives::hex;
use fibonacci_lib::{PublicValuesLiteCoinHoldings,LiteCoinHoldingsInput};
use sha2::{Digest, Sha256};
use alloy_primitives::Address;

pub fn main() {
    let input: LiteCoinHoldingsInput = sp1_zkvm::io::read();
    // Verify recipient address (hardcoded for simplicity)
    let expected_recipient = "ltc1qqhvj3grzewmqjp3sehjqjacg2pwkukxmqp84sr";
    assert_eq!(input.recipient_address, expected_recipient, "Recipient address mismatch");
    // Hash the sender address
    let sender_hash_digest = Sha256::digest(input.sender_address.as_bytes());
    let sender_hash_array: [u8; 32] = sender_hash_digest.into();
    let sender_fixed_hash = FixedBytes::<32>::from(sender_hash_array);
    // Keep owner address as string (no parsing needed)
    let owner_address: Address = input.owner_address.parse::<Address>().expect("failed to parse ethereum address");
    // Transaction hash
    let tx_hash_bytes = hex::decode(input.tx_hash).expect("Invalid tx_hash format");
    let txid_bytes: [u8; 32] = tx_hash_bytes
        .as_slice()
        .try_into()
        .expect("tx_hash must be 32 bytes long");
    let public_values = PublicValuesLiteCoinHoldings {
        total_litecoin: input.amount,
        sender_address_hash: sender_fixed_hash,
        owner_address: owner_address, // Keep as string
        tx_hash: FixedBytes::<32>::from(txid_bytes),
    };
    // let input: CardanoTxInput = sp1_zkvm::io::read();

    // // Verify recipient address (hardcoded for simplicity)
    // let expected_recipient = "addr1qyvxngqhhvzunlxlkw4f9m6nep00spqtmrlvfgynmrq5q7r0mjnf84mnk78ytza3sunyvqs3llehvfjuwvk338d69t2qqag5yl";
    // assert_eq!(input.recipient_address, expected_recipient, "Recipient address mismatch");

    // // Hash the sender address
    // let sender_hash_digest = Sha256::digest(input.sender_address.as_bytes());
    // let sender_hash_array: [u8; 32] = sender_hash_digest.into();
    // let sender_fixed_hash = FixedBytes::<32>::from(sender_hash_array);

    // // Keep owner address as string (no parsing needed)
    // let owner_address: Address = input.owner_address.parse::<Address>().expect("failed to parse ethereum address");

    // // Transaction hash
    // let tx_hash_bytes = hex::decode(input.tx_hash).expect("Invalid tx_hash format");
    // let txid_bytes: [u8; 32] = tx_hash_bytes
    //     .as_slice()
    //     .try_into()
    //     .expect("tx_hash must be 32 bytes long");

    // let public_values = PublicValuesCardanoTx {
    //     total_lovelace: input.amount,
    //     sender_address_hash: sender_fixed_hash,
    //     owner_address: owner_address, // Keep as string
    //     tx_hash: FixedBytes::<32>::from(txid_bytes),
    // };

    let bytes = PublicValuesLiteCoinHoldings::abi_encode(&public_values);
    sp1_zkvm::io::commit_slice(&bytes);
}
