use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesIcr, PublicValuesLiquidation, PublicValuesLtv};
use fibonacci_lib::{PublicValuesBtcHoldings, Utxo, BtcHoldingsInput};
use tokio::task;
use anyhow::Result;

// Program binary
#[allow(unused_variables, unused_imports, dead_code)]
pub const DEFI_ELF: &[u8] = include_elf!("fibonacci-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const ICR_ELF: &[u8] = include_elf!("icr-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const LIQUIDATION_ELF: &[u8] = include_elf!("liquid-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const REAL_TIME_LTV_ELF: &[u8] = include_elf!("Real_time_ltv-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const putCall_ELF: &[u8] = include_elf!("putCall-program");

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
pub struct BtcHoldingsRequest {
    btc_address: String,
    org_id: String,
    call_value: String,
    put_value: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BtcHoldingsResponse {
    total_btc: u64,
    org_hash: String,
    call_value: String,
    put_value: String,
    vkey: String,
    public_values: String,
    proof: String,
    verifier_version: String,
}

#[derive(Debug, Deserialize)]
struct BlockstreamUtxo {
    txid: String,
    vout: u32,
    value: u64,
}

// Helper struct for BTC balance fetch
#[derive(Debug, Deserialize)]
struct UtxoValue {
    value: u64, // in satoshis
}

pub async fn fetch_user_btc_balance(user_address: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let url = format!("https://mempool.space/api/address/{}/utxo", user_address);
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch UTXO: {}", response.status()).into());
    }
    let utxos: Vec<UtxoValue> = response.json().await?;
    let total_sats: u64 = utxos.iter().map(|u| u.value).sum();
    let total_btc = total_sats as f64 / 100_000_000.0;
    Ok(total_btc)
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
pub struct CatalysisResponse {
    user_address: String,
    reward_amount: u32,
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
pub struct UserCatalysis {
    user_address: String,
    reward_amount: u32,
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
pub struct ProofResponseUserBalance {
    user_address: String,
    btc_price_usd: u32,
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
pub struct UserAddress {
    user_address: String,
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

#[post("/generate-proof-icr")]
async fn prove_icr_final(req: web::Json<ProofRequestIcr>) -> impl Responder {
    println!("Received ICR proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(ICR_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.collateral_amount);
        stdin.write(&req.debt_amount);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
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
async fn prove_liquidation(req: web::Json<UserDataLiquidation>) -> impl Responder {
    println!("Received liquidation proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(LIQUIDATION_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.collateral_amount);
        stdin.write(&req.min_icr);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
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
async fn prove_ltv(req: web::Json<ProofRequestLtv>) -> impl Responder {
    println!("Received LTV proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(REAL_TIME_LTV_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.debt_amount);
        stdin.write(&req.collateral_amount);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
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

    let response = ProofResponseLtv {
        real_time_ltv: public_values.real_time_ltv,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/prove-btc-holdings")]
async fn prove_btc_holdings(req: web::Json<BtcHoldingsRequest>) -> impl Responder {
    println!("Received BTC holdings proof request: {:?}", req);

    let utxos = match fetch_utxos(&req.btc_address).await {
        Ok(utxos) => utxos,
        Err(e) => {
            eprintln!("Failed to fetch UTXOs: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("UTXO fetch failed: {}", e));
        }
    };

    if utxos.is_empty() {
        return HttpResponse::BadRequest().body("No UTXOs found for the address");
    }

    let expected_total = utxos.iter().map(|u| u.value).sum::<u64>();
    let org_id = req.org_id.clone();
    let proof_system = req.proof_system.clone();
    let total_call_value = req.call_value.parse::<u64>().unwrap_or(0);
    let total_put_value = req.put_value.parse::<u64>().unwrap_or(0);

    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(putCall_ELF);
        let mut stdin = sp1_sdk::SP1Stdin::new();

        let utxos = utxos
            .into_iter()
            .map(|u| {
                let txid = hex::decode(&u.txid).expect("valid txid hex");
                let pubkey = vec![0u8; 33]; // dummy compressed pubkey
                Utxo {
                    txid: txid.try_into().expect("32 bytes"),
                    index: u.vout,
                    amount: u.value,
                    pubkey,
                }
            })
            .collect::<Vec<_>>();

        let input = BtcHoldingsInput {
            utxos,
            signatures: vec![],
            expected_total,
            org_id,
            total_call_value: total_call_value.to_string(),
            total_put_value: total_put_value.to_string(),
        };
        stdin.write(&input);

        let proof_result = match proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesBtcHoldings::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = BtcHoldingsResponse {
        total_btc: public_values.total_btc,
        org_hash: format!("0x{}", hex::encode(public_values.org_hash)),
        put_value: public_values.total_put_value.to_string(),
        call_value: public_values.total_call_value.to_string(),
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
        verifier_version: "0x1234abcd".to_string(),
    };

    HttpResponse::Ok().json(response)
}

async fn fetch_utxos(address: &str) -> Result<Vec<BlockstreamUtxo>, Box<dyn Error>> {
    let url = format!("https://blockstream.info/api/address/{}/utxo", address);
    let resp = reqwest::get(&url).await?;
    let utxos: Vec<BlockstreamUtxo> = resp.json().await?;
    Ok(utxos)
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .service(prove_icr_final)
            .service(prove_liquidation)
            .service(prove_ltv)
            .service(prove_btc_holdings)
    })
    .workers(4)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}