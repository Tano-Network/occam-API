//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can have an
//! EVM-Compatible proof generated which can be verified on-chain.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release --bin evm -- --system groth16
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release --bin evm -- --system plonk
//! ```

use alloy_sol_types::SolType;
use clap::{Parser, ValueEnum};
use fibonacci_lib::PublicValuesStruct;
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    include_elf, HashableKey, ProverClient, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use std::path::PathBuf;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-program");

/// The arguments for the EVM command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct EVMArgs {
    #[arg(long, default_value = "20")]
    n: u32,
    #[arg(long, default_value = "20")]
    collateral_amount: u32,
    #[arg(long, default_value = "10")]
    debt_amount: u32,
    #[arg(long, default_value = "50000")]
    usbd_loan: u32,
    #[arg(long, default_value = "5")]
    btc_balance: u32,

    #[arg(long, value_enum, default_value = "groth16")]
    system: ProofSystem,
}

/// Enum representing the available proof systems
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ProofSystem {
    Plonk,
    Groth16,
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1FibonacciProofFixture {
    a: u32,
    b: u32,
    n: u32,
    icr: u32,
    collateral_amount: u32,
    liquidation_threshold: u32,
    real_time_ltv: u32,
    btc_price_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Deserialize)]
struct BtcPriceResponse {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    usd: f64,
}

async fn fetch_btc_price() -> Result<u32, Box<dyn std::error::Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let resp: BtcPriceResponse = reqwest::get(url).await?.json().await?;
    Ok(resp.bitcoin.usd.round() as u32)
}

#[tokio::main]
async fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = EVMArgs::parse();

    // Fetch BTC price in USD.
    let btc_price_usd = fetch_btc_price().await.expect("Failed to fetch BTC price");

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the program.
    let (pk, vk) = client.setup(FIBONACCI_ELF);

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.n);
    stdin.write(&args.collateral_amount);
    stdin.write(&args.debt_amount);
    stdin.write(&args.debt_amount);
    stdin.write(&args.usbd_loan);
    stdin.write(&btc_price_usd);  // <-- Insert fetched BTC price

    println!("n: {}", args.n);
    println!("collateral_amount: {}", args.collateral_amount);
    println!("debt_amount: {}", args.debt_amount);
    println!("usbd_loan: {}", args.usbd_loan);
    println!("btc_balance: {}", args.btc_balance);
    println!("btc_price_usd (fetched): {}", btc_price_usd);
    println!("Proof System: {:?}", args.system);

    // Generate the proof based on the selected proof system.
    let proof = match args.system {
        ProofSystem::Plonk => client.prove(&pk, &stdin).plonk().run(),
        ProofSystem::Groth16 => client.prove(&pk, &stdin).groth16().run(),
    }
    .expect("failed to generate proof");

    create_proof_fixture(&proof, &vk, args.system, btc_price_usd);
}

/// Create a fixture for the given proof.
fn create_proof_fixture(
    proof: &SP1ProofWithPublicValues,
    vk: &SP1VerifyingKey,
    system: ProofSystem,
    btc_price_usd: u32,
) {
    // Deserialize the public values.
    let bytes = proof.public_values.as_slice();
    let PublicValuesStruct {
        n,
        a,
        b,
        icr,
        collateral_amount,
        liquidation_threshold,
        real_time_ltv,
    } = PublicValuesStruct::abi_decode(bytes).unwrap();
    // Create the testing fixture so we can test things end-to-end.
    let fixture = SP1FibonacciProofFixture {
        a,
        b,
        n,
        icr,
        collateral_amount,
        liquidation_threshold,
        real_time_ltv,
        btc_price_usd,
        vkey: vk.bytes32().to_string(),
        public_values: format!("0x{}", hex::encode(bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    println!("Verification Key: {}", fixture.vkey);
    println!("Public Values: {}", fixture.public_values);
    println!("Proof Bytes: {}", fixture.proof);

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join(format!("{:?}-fixture.json", system).to_lowercase()),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");
}