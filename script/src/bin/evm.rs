use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, HashableKey};
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesStruct, PublicValuesIcr, PublicValuesLiquidation, PublicValuesLtv};

// Program binary
pub const DeFi_ELF: &[u8] = include_elf!("fibonacci-program");
pub const icr_elf: &[u8] = include_elf!("icr-program");
pub const liquidation_elf: &[u8] = include_elf!("liquid-program");
pub const real_time_ltv_elf: &[u8] = include_elf!("Real_time_ltv-program");

// Minimum ICR required (150%)
const MIN_ICR: u32 = 150;

// === Data structures ===
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponse {
    icr: u32,
    collateral_value_usd: u32,
    liquidation_threshold: u32,
    real_time_ltv: u32,
    btc_price_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataLiquidationResponse {
    liquidation_threshold: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataLiquidation {
    collateral_amount: u32,
    min_icr: u32,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseIcr {
    icr: u32,
    collateral_value_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseLtv {
    real_time_ltv: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestIcr {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestLtv {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcrResponse {
    icr: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateProofRequest {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}

#[derive(Debug, Deserialize)]
struct BtcPriceResponse {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    usd: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserData {
    id: u32,
    user_address: String,
    amount_in_btc: f64,
    price_at_deposited: String,
    usbd_minted: String,
    collateral_ratio: String,
    #[serde(default)]
    created_at: String,
}

// === API endpoints ===
#[post("/generate-proof")]
async fn generate_proof_handler(
    req: web::Json<GenerateProofRequest>,
) -> impl Responder {
    println!("Received proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(DeFi_ELF);

    let mut stdin = SP1Stdin::new();
    stdin.write(&req.collateral_amount);
    stdin.write(&req.debt_amount);
    stdin.write(&btc_price_usd);
    stdin.write(&MIN_ICR);

    let proof_result = match req.proof_system.as_str() {
        "plonk" => client.prove(&pk, &stdin).plonk().run(),
        "groth16" => client.prove(&pk, &stdin).groth16().run(),
        _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
    };

    let proof = match proof_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = ProofResponse {
        icr: public_values.icr,
        collateral_value_usd: public_values.collateral_amount,
        liquidation_threshold: public_values.liquidation_threshold,
        real_time_ltv: public_values.real_time_ltv,
        btc_price_usd,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-batch")]
async fn generate_proof_batch_handler(
    req: web::Json<UserDataLiquidation>,
) -> impl Responder {
    println!("Received batch proof generation request");

    let users_data = match fetch_users_data().await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch user data: {:?}", e);
            return HttpResponse::InternalServerError().body("User data fetch failed");
        }
    };
    println!("Fetched {} users from API", users_data.len());

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(DeFi_ELF);

    let mut results = Vec::new();

    for user in &users_data {
        println!(
            "Processing User ID: {}, Address: {}, Amount in BTC: {}",
            user.id, user.user_address, user.amount_in_btc
        );

        let collateral_amount = (user.amount_in_btc * 100.0) as u32;
        let debt_amount = match user.usbd_minted.parse::<f64>() {
            Ok(val) => (val * 100.0) as u32,
            Err(_) => {
                eprintln!("Failed to parse usbd_minted for user {}", user.id);
                continue;
            }
        };

        let mut stdin = SP1Stdin::new();
        stdin.write(&collateral_amount);
        stdin.write(&debt_amount);
        stdin.write(&btc_price_usd);
        stdin.write(&MIN_ICR);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return HttpResponse::BadRequest().body("Invalid proof system"),
        };

        let proof = match proof_result {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Proof generation failed for user {}: {:?}", user.id, e);
                continue;
            }
        };

        let public_bytes = proof.public_values.as_slice();
        eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
        let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
            Ok(val) => val,
            Err(e) => {
                eprintln!("Decoding public values failed for user {}: {:?}", user.id, e);
                continue;
            }
        };

        let response = ProofResponse {
            icr: public_values.icr,
            collateral_value_usd: public_values.collateral_amount,
            liquidation_threshold: public_values.liquidation_threshold,
            real_time_ltv: public_values.real_time_ltv,
            btc_price_usd,
            vkey: vk.bytes32(),
            public_values: format!("0x{}", hex::encode(public_bytes)),
            proof: format!("0x{}", hex::encode(proof.bytes())),
        };

        results.push((user.id, user.user_address.clone(), response));
    }

    HttpResponse::Ok().json(results)
}

#[post("/generate-proof-icr")]
async fn prove_icr_final(
    req: web::Json<ProofRequestIcr>,
) -> impl Responder {
    println!("Received ICR proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(icr_elf);

    let mut stdin = SP1Stdin::new();
    stdin.write(&req.collateral_amount);
    stdin.write(&req.debt_amount);
    stdin.write(&btc_price_usd);

    let proof_result = match req.proof_system.as_str() {
        "plonk" => client.prove(&pk, &stdin).plonk().run(),
        "groth16" => client.prove(&pk, &stdin).groth16().run(),
        _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
    };

    let proof = match proof_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesIcr::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = ProofResponseIcr {
        icr: public_values.icr,
        collateral_value_usd: public_values.collateral_amount,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-liquidation")]
async fn prove_liquidation(
    req: web::Json<UserDataLiquidation>,
) -> impl Responder {
    println!("Received liquidation proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(liquidation_elf);

    let mut stdin = SP1Stdin::new();
    stdin.write(&req.collateral_amount);
    stdin.write(&req.min_icr);
    stdin.write(&btc_price_usd);

    let proof_result = match req.proof_system.as_str() {
        "plonk" => client.prove(&pk, &stdin).plonk().run(),
        "groth16" => client.prove(&pk, &stdin).groth16().run(),
        _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
    };

    let proof = match proof_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesLiquidation::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = UserDataLiquidationResponse {
        liquidation_threshold: public_values.liquidation_threshold,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-ltv")]
async fn prove_ltv(
    req: web::Json<ProofRequestLtv>,
) -> impl Responder {
    println!("Received LTV proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;
    println!("Inputs: debt_amount={}, collateral_amount={}, btc_price_usd={}", 
             req.debt_amount, req.collateral_amount, btc_price_usd);

    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(real_time_ltv_elf);

    let mut stdin = SP1Stdin::new();
    stdin.write(&req.debt_amount);
    stdin.write(&req.collateral_amount);
    stdin.write(&btc_price_usd);
    println!("Written to stdin: debt_amount={}, collateral_amount={}, btc_price_usd={}", 
             req.debt_amount, req.collateral_amount, btc_price_usd);

    let proof_result = match req.proof_system.as_str() {
        "plonk" => client.prove(&pk, &stdin).plonk().run(),
        "groth16" => client.prove(&pk, &stdin).groth16().run(),
        _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
    };

    let proof = match proof_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesLtv::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };
    println!("Decoded real_time_ltv: {}", public_values.real_time_ltv);

    let response = ProofResponseLtv {
        real_time_ltv: public_values.real_time_ltv,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

// === Helper functions ===
async fn fetch_btc_price() -> Result<f64, Box<dyn Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let resp = reqwest::get(url).await?;
    let text = resp.text().await?;
    println!("CoinGecko response: {}", text);
    let json: BtcPriceResponse = serde_json::from_str(&text)?;
    if json.bitcoin.usd <= 0.0 {
        return Err("BTC price is zero or negative".into());
    }
    Ok(json.bitcoin.usd)
}

async fn fetch_users_data() -> Result<Vec<UserData>, Box<dyn std::error::Error + Send + Sync>> {
    println!("Fetching user data from API...");
    let url = "http://139.59.8.108:3010/service/users-data";
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(format!("API returned error status: {}", response.status()).into());
    }

    let users: Vec<UserData> = response.json().await?;
    println!("Fetched {} users from API", users.len());
    Ok(users)
}


// === Root route ===
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("SP1 Proof Server is running!")
}

// === Main function ===
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .service(index) // Root health check route
            .service(generate_proof_handler)
            .service(generate_proof_batch_handler)
            .service(prove_icr_final)
            .service(prove_liquidation)
            .service(prove_ltv)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
