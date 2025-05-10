use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use fibonacci_lib::PublicValuesStruct;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, HashableKey};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SP1FibonacciProofFixture {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateProofRequest {
    n: u32,
    collateral_amount: u32,
    debt_amount: u32,
    usbd_loan: u32,
    btc_balance: u32,           // Add this
    proof_system: String,       // "groth16" or "plonk"
}

#[derive(Debug, Deserialize)]
struct BtcPriceResponse {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    usd: f64,
}

pub const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-program");

#[post("/generate-proof")]
async fn generate_proof_handler(
    req: web::Json<GenerateProofRequest>,
) -> impl Responder {
    println!("Received proof generation request: {:?}", req);

    // Fetch BTC price
    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };

    // Initialize Prover client and keys
    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(FIBONACCI_ELF);

    // Prepare input for SP1
    let mut stdin = SP1Stdin::new();
    stdin.write(&req.n);
    stdin.write(&req.collateral_amount);
    stdin.write(&req.debt_amount);
    stdin.write(&(btc_price as u32));           // btc_price_usd
    stdin.write(&req.usbd_loan);
    stdin.write(&req.btc_balance);             // Final required value

    // Run proof system
    let proof_result = match req.proof_system.as_str() {
        "plonk" => client.prove(&pk, &stdin).plonk().run(),
        "groth16" => client.prove(&pk, &stdin).groth16().run(),
        _ => return HttpResponse::BadRequest().body("Invalid proof system"),
    };

    let proof = match proof_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation failed");
        }
    };

    // Decode public values
    let public_bytes = proof.public_values.as_slice();
    let PublicValuesStruct {
        n,
        a,
        b,
        icr,
        collateral_amount,
        liquidation_threshold,
        real_time_ltv,
    } = match PublicValuesStruct::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // Assemble result fixture
    let fixture = SP1FibonacciProofFixture {
        a,
        b,
        n,
        icr,
        collateral_amount,
        liquidation_threshold,
        real_time_ltv,
        btc_price_usd: btc_price as u32,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(fixture)
}

async fn fetch_btc_price() -> Result<f64, Box<dyn Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let resp: BtcPriceResponse = reqwest::get(url).await?.json().await?;
    Ok(resp.bitcoin.usd)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    sp1_sdk::utils::setup_logger();
    println!("Starting Actix-web SP1 proof server on http://localhost:8080");

    HttpServer::new(|| App::new().service(generate_proof_handler))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}