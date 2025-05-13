use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use fibonacci_lib::PublicValuesStruct;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, HashableKey};
use std::error::Error;

// Program binary
pub const DeFi_ELF: &[u8] = include_elf!("fibonacci-program");
pub const icr_elf: &[u8] = include_elf!("Icr");
pub const liquidation_elf: &[u8] = include_elf!("Liquid");
pub const real_time_ltv_elf: &[u8] = include_elf!("Real_time_ltv");

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
pub struct UserDataLiquidation{
    collateral_amount: u32,  // Amount of BTC
    min_icr: u32,        // Debt in USD
    proof_system: String,    // "groth16" or "plonk"
   
}



#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseIcr{
    icr: u32,
    collateral_value_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestIcr{
    collateral_amount: u32,  // Amount of BTC
    debt_amount: u32,        // Debt in USD
    proof_system: String,    // "groth16" or "plonk"
   
}

// Updated response struct for ICR endpoint to include proof
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
    collateral_amount: u32,  // Amount of BTC
    debt_amount: u32,        // Debt in USD
    proof_system: String,    // "groth16" or "plonk"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDataLiquidation {
    proof_system: String,    // "groth16" or "plonk"
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

// // === API endpoints ===
// #[post("/generate-proof")]
// async fn generate_proof_handler(
//     req: web::Json<GenerateProofRequest>,
// ) -> impl Responder {
//     println!("Received proof generation request: {:?}", req);

//     // Fetch BTC price
//     let btc_price = match fetch_btc_price().await {
//         Ok(price) => price,
//         Err(e) => {
//             eprintln!("Failed to fetch BTC price: {:?}", e);
//             return HttpResponse::InternalServerError().body("BTC price fetch failed");
//         }
//     };
//     let btc_price_usd = btc_price as u32;
    
//     // Initialize prover client and generate proving/verification keys
//     let client = ProverClient::from_env();
//     let (pk, vk) = client.setup(DeFi_ELF);

//     // Prepare zkVM input
//     let mut stdin = SP1Stdin::new();
//     stdin.write(&req.collateral_amount);  // BTC amount
//     stdin.write(&req.debt_amount);        // Debt in USD
//     stdin.write(&btc_price_usd);          // BTC price in USD
//     stdin.write(&MIN_ICR);                // Minimum ICR required

//     // Generate proof
//     let proof_result = match req.proof_system.as_str() {
//         "plonk" => client.prove(&pk, &stdin).plonk().run(),
//         "groth16" => client.prove(&pk, &stdin).groth16().run(),
//         _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
//     };

//     let proof = match proof_result {
//         Ok(p) => p,
//         Err(e) => {
//             eprintln!("Proof generation failed: {:?}", e);
//             return HttpResponse::InternalServerError().body("Proof generation failed");
//         }
//     };

//     // Decode public values from proof
//     let public_bytes = proof.public_values.as_slice();
//     let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
//         Ok(val) => val,
//         Err(e) => {
//             eprintln!("Decoding public values failed: {:?}", e);
//             return HttpResponse::InternalServerError().body("Failed to decode public values");
//         }
//     };

//     // Prepare response
//     let response = ProofResponse {
//         icr: public_values.icr,
//         collateral_value_usd: public_values.collateral_amount,
//         liquidation_threshold: public_values.liquidation_threshold,
//         real_time_ltv: public_values.real_time_ltv,
//         btc_price_usd,
//         vkey: vk.bytes32(),
//         public_values: format!("0x{}", hex::encode(public_bytes)),
//         proof: format!("0x{}", hex::encode(proof.bytes())),
//     };

//     HttpResponse::Ok().json(response)
// }

// #[post("/generate-proof-batch")]
// async fn generate_proof_batch_handler(
//     req: web::Json<UserDataLiquidation>,
// ) -> impl Responder {
//     println!("Received batch proof generation request");

//     // Fetch user data
//     let users_data = match fetch_users_data().await {
//         Ok(data) => data,
//         Err(e) => {
//             eprintln!("Failed to fetch user data: {:?}", e);
//             return HttpResponse::InternalServerError().body("User data fetch failed");
//         }
//     };
//     println!("Fetched {} users from API", users_data.len());

//     // Fetch BTC price
//     let btc_price = match fetch_btc_price().await {
//         Ok(price) => price,
//         Err(e) => {
//             eprintln!("Failed to fetch BTC price: {:?}", e);
//             return HttpResponse::InternalServerError().body("BTC price fetch failed");
//         }
//     };
//     let btc_price_usd = btc_price as u32;

//     // Initialize prover client once (reuse for all users)
// let client = ProverClient::from_env();    let (pk, vk) = client.setup(DeFi_ELF);

//     // Results vector to store all user proofs
//     let mut results = Vec::new();

//     // Generate proofs for each user
//     for user in &users_data {
//         println!(
//             "Processing User ID: {}, Address: {}, Amount in BTC: {}",
//             user.id, user.user_address, user.amount_in_btc
//         );

//         // Parse user data
//         let collateral_amount = (user.amount_in_btc * 100.0) as u32; // Convert to integer (BTC * 100)
//         let debt_amount = match user.usbd_minted.parse::<f64>() {
//             Ok(val) => (val * 100.0) as u32, // Convert to integer (USD * 100)
//             Err(_) => {
//                 eprintln!("Failed to parse usbd_minted for user {}", user.id);
//                 continue; // Skip this user
//             }
//         };

//         // Prepare zkVM input
//         let mut stdin = SP1Stdin::new();
//         stdin.write(&collateral_amount);
//         stdin.write(&debt_amount);
//         stdin.write(&btc_price_usd);
//         stdin.write(&MIN_ICR);

//         // Generate proof
//         let proof_result = match req.proof_system.as_str() {
//             "plonk" => client.prove(&pk, &stdin).plonk().run(),
//             "groth16" => client.prove(&pk, &stdin).groth16().run(),
//             _ => return HttpResponse::BadRequest().body("Invalid proof system"),
//         };

//         let proof = match proof_result {
//             Ok(p) => p,
//             Err(e) => {
//                 eprintln!("Proof generation failed for user {}: {:?}", user.id, e);
//                 continue; // Skip this user
//             }
//         };

//         // Decode public values
//         let public_bytes = proof.public_values.as_slice();
//         let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
//             Ok(val) => val,
//             Err(e) => {
//                 eprintln!("Decoding public values failed for user {}: {:?}", user.id, e);
//                 continue;
//             }
//         };

//         // Prepare response for this user
//         let response = ProofResponse {
//             icr: public_values.icr,
//             collateral_value_usd: public_values.collateral_amount,
//             liquidation_threshold: public_values.liquidation_threshold,
//             real_time_ltv: public_values.real_time_ltv,
//             btc_price_usd,
//             vkey: vk.bytes32(),
//             public_values: format!("0x{}", hex::encode(public_bytes)),
//             proof: format!("0x{}", hex::encode(proof.bytes())),
//         };

//         results.push((user.id, user.user_address.clone(), response));
//     }

//     HttpResponse::Ok().json(results)


// // Updated endpoint to generate ICR for a specific user, including proof
// #[get("/icr/{user_id}")]
// async fn icr_handler(
//     user_id: web::Path<u32>,
//     query: web::Query<UserDataLiquidation>,
// ) -> impl Responder {
//     println!("Received ICR generation request for user ID: {}", *user_id);

//     // Fetch user data
//     let users_data = match fetch_users_data().await {
//         Ok(data) => data,
//         Err(e) => {
//             eprintln!("Failed to fetch user data: {:?}", e);
//             return HttpResponse::InternalServerError().body("User data fetch failed");
//         }
//     };

//     // Find the user with the specified ID
//     let user = match users_data.iter().find(|u| u.id == *user_id) {
//         Some(user) => user,
//         None => {
//             return HttpResponse::NotFound().body(format!("User with ID {} not found", *user_id));
//         }
//     };

//     println!(
//         "Processing User ID: {}, Address: {}, Amount in BTC: {}",
//         user.id, user.user_address, user.amount_in_btc
//     );

//     // Fetch BTC price
//     let btc_price = match fetch_btc_price().await {
//         Ok(price) => price,
//         Err(e) => {
//             eprintln!("Failed to fetch BTC price: {:?}", e);
//             return HttpResponse::InternalServerError().body("BTC price fetch failed");
//         }
//     };
//     let btc_price_usd = btc_price as u32;

//     // Parse user data
//     let collateral_amount = (user.amount_in_btc * 100.0) as u32; // Convert to integer (BTC * 100)
//     let debt_amount = match user.usbd_minted.parse::<f64>() {
//         Ok(val) => (val * 100.0) as u32, // Convert to integer (USD * 100)
//         Err(_) => {
//             eprintln!("Failed to parse usbd_minted for user {}", user.id);
//             return HttpResponse::BadRequest().body("Invalid usbd_minted value");
//         }
//     };

//     // Initialize prover client and generate proving/verification keys
//     let client = ProverClient::from_env();
//     let (pk, vk) = client.setup(DeFi_ELF);

//     // Prepare zkVM input
//     let mut stdin = SP1Stdin::new();
//     stdin.write(&collateral_amount);
//     stdin.write(&debt_amount);
//     stdin.write(&btc_price_usd);
//     stdin.write(&MIN_ICR);

//     // Generate proof
//     let proof_result = match query.proof_system.as_str() {
//         "plonk" => client.prove(&pk, &stdin).plonk().run(),
//         "groth16" => client.prove(&pk, &stdin).groth16().run(),
//         _ => return HttpResponse::BadRequest().body("Invalid proof system - must be 'plonk' or 'groth16'"),
//     };

//     let proof = match proof_result {
//         Ok(p) => p,
//         Err(e) => {
//             eprintln!("Proof generation failed for user {}: {:?}", user.id, e);
//             return HttpResponse::InternalServerError().body("Proof generation failed");
//         }
//     };

//     // Decode public values
//     let public_bytes = proof.public_values.as_slice();
//     let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
//         Ok(val) => val,
//         Err(e) => {
//             eprintln!("Decoding public values failed for user {}: {:?}", user.id, e);
//             return HttpResponse::InternalServerError().body("Failed to decode public values");
//         }
//     };

//     // Prepare response
//     let response = IcrResponse {
//         icr: public_values.icr,
//         vkey: vk.bytes32(),
//         public_values: format!("0x{}", hex::encode(public_bytes)),
//         proof: format!("0x{}", hex::encode(proof.bytes())),
//     };

//     HttpResponse::Ok().json(response)
// }

#[post("/generate-proof-icr")]
async fn prove_icr_final(
    req: web::Json<ProofRequestIcr>,
) -> impl Responder {
    println!("Received ICR proof generation request: {:?}", req);

    // Fetch BTC price
    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    // Initialize prover client and generate proving/verification keys
    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(icr_elf); // Correctly using icr_elf

    // Prepare zkVM input
    let mut stdin = SP1Stdin::new();
    stdin.write(&req.collateral_amount);  // BTC amount
    stdin.write(&req.debt_amount);        // Debt in USD
    stdin.write(&btc_price_usd);          // BTC price in USD

    // Generate proof
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

    // Decode public values from proof
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // Prepare response
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

    // Fetch BTC price
    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    // Initialize prover client and generate proving/verification keys
    let client = ProverClient::from_env();
    let (pk, vk) = client.setup(liquidation_elf); // Correctly using liquidation_elf

    // Prepare zkVM input
    let mut stdin = SP1Stdin::new();
    stdin.write(&req.proof_system);  // BTC amount

    // Generate proof
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

    // Decode public values from proof
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesStruct::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // Prepare response
    let response = ProofResponseIcr {
        icr: public_values.icr,
        collateral_value_usd: public_values.collateral_amount,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

// === Helper functions ===
// async fn fetch_btc_price() -> Result<f64, Box<dyn Error>> {
//     let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
//     let resp: BtcPriceResponse = reqwest::get(url).await?.json().await?;
//     Ok(resp.bitcoin.usd)
// }

// async fn fetch_users_data() -> Result<Vec<UserData>, Box<dyn std::error::Error + Send + Sync>> {
//     println!("Fetching user data from API...");
//     let url = "http://139.59.8.108:3010/service/users-data";
//     let response = reqwest::get(url).await?;

//     if !response.status().is_success() {
//         return Err(format!("API returned error status: {}", response.status()).into());
//     }

//     let users: Vec<UserData> = response.json().await?;
//     println!("Fetched {} users from API", users.len());
//     Ok(users)
// }

// === Main function ===
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    sp1_sdk::utils::setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .service(generate_proof_handler)
            .service(generate_proof_batch_handler)
            .service(icr_handler) // Register new endpoint
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}