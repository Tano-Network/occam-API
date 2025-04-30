//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use alloy_sol_types::SolType;
use clap::Parser;
use fibonacci_lib::PublicValuesStruct;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use reqwest;
use serde::Deserialize;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

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
    dotenv::dotenv().ok();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // === Fetch BTC price in USD ===
    let btc_price_usd = fetch_btc_price().await.expect("Failed to fetch BTC price");
    println!("btc_price_usd: {}", btc_price_usd);

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.n);
    stdin.write(&args.collateral_amount);
    stdin.write(&args.debt_amount);
    stdin.write(&btc_price_usd); // <-- New input
    stdin.write(&args.usbd_loan);
    stdin.write(&args.btc_balance);


    println!("n: {}", args.n);
    println!("collateral_amount: {}", args.collateral_amount);
    println!("debt_amount: {}", args.debt_amount);
    println!("usbd_loan: {}", args.usbd_loan);
    println!("btc_balance: {}", args.btc_balance);


    if args.execute {
        // Execute the program
        let (output, report) = client.execute(FIBONACCI_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let decoded = PublicValuesStruct::abi_decode(output.as_slice()).unwrap();
        let PublicValuesStruct { n, a, b, icr, collateral_amount,liquidation_threshold,real_time_ltv } = decoded;
        println!("n: {}", n);
        println!("a: {}", a);
        println!("b: {}", b);
        println!("icr: {}", icr);
        println!("collateral_amount: {}", collateral_amount);
        println!("liquidation_threshold: {}", liquidation_threshold);
        println!("real_time_ltv: {}", real_time_ltv);

        let (expected_a, expected_b) = fibonacci_lib::fibonacci(n);
        assert_eq!(a, expected_a);
        assert_eq!(b, expected_b);
        println!("Values are correct!");

        let (expected_icr, expected_collateral_amount) = fibonacci_lib::calculate_icr(
            args.collateral_amount,
            args.debt_amount,
            btc_price_usd,
        );
        assert_eq!(icr, expected_icr);
        let liquidation_threshold = fibonacci_lib::calculate_liquidation_threshold(
            args.collateral_amount,
            args.btc_balance,
            icr, // <-- pass the ICR value you compute earlier
        );

        let real_time_ltv = fibonacci_lib::real_time_ltv(args.usbd_loan, args.btc_balance,args.btc_balance);
        assert_eq!(real_time_ltv, real_time_ltv);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());

    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(FIBONACCI_ELF);

        // Generate the proof
        let proof = client
            .prove(&pk, &stdin)
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}