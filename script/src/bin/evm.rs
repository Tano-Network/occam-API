use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesIcr, PublicValuesLiquidation, PublicValuesLtv};
use fibonacci_lib::{PublicValuesBtcHoldings, Utxo, BtcHoldingsInput, PublicValuesDogeTx, DogeTxInput};
use tokio::task;
use anyhow::Result;
use sp1_sdk::SP1ProofMode;
use sp1_sdk::Prover;
use sp1_sdk::network::FulfillmentStrategy;

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
pub const PUTCALL_ELF: &[u8] = include_elf!("putCall-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const DOGE_TX_ELF: &[u8] = include_elf!("doge_tx-program");

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxRequest {
    tx_hash: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxResponse {
    total_doge: u64,
    sender_address: String,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Deserialize, Clone)]
struct BlockstreamUtxo {
    txid: String,
    vout: u32,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct UtxoValue {
    value: u64, // in satoshis
}

#[derive(Debug, Deserialize)]
struct BlockchairTx {
    transaction: TransactionDetails,
    inputs: Vec<InputDetails>,
    outputs: Vec<OutputDetails>,
}

#[derive(Debug, Deserialize)]
struct TransactionDetails {
    hash: String,
    input_total: u64,
    output_total: u64,
}

#[derive(Debug, Deserialize)]
struct InputDetails {
    recipient: String,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct OutputDetails {
    recipient: String,
    value: u64,
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

    // === Step 1: Fetch UTXOs ===
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

    // === Step 2: Clone for move into spawn_blocking ===
    let org_id = req.org_id.clone();
    let proof_system = req.proof_system.clone();
    let total_call_value = req.call_value.clone();
    let total_put_value = req.put_value.clone();
    let utxos_for_task = utxos.clone();

    // === Step 3: Run proof generation in blocking task ===
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(PUTCALL_ELF);

        // === Build stdin ===
        let mut stdin = SP1Stdin::new();

        // Convert UTXOs into circuit format
        let converted_utxos: Vec<Utxo> = utxos_for_task
            .into_iter()
            .filter_map(|u| {
                let txid_decoded = hex::decode(&u.txid).ok()?;
                let txid_bytes: [u8; 32] = txid_decoded.try_into().ok()?;
                Some(Utxo {
                    txid: txid_bytes,
                    index: u.vout,
                    amount: u.value,
                    pubkey: vec![0u8; 33], // Dummy pubkey
                })
            })
            .collect();

        let expected_total = converted_utxos.iter().map(|u| u.amount).sum::<u64>();

        let input = BtcHoldingsInput {
            utxos: converted_utxos,
            signatures: vec![], // Add actual signatures later if needed
            expected_total,
            org_id,
            total_call_value: total_call_value.clone(),
            total_put_value: total_put_value.clone(),
        };

        stdin.write(&input);

        // === Prove ===
        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system.as_str() {
            "groth16" => builder.mode(SP1ProofMode::Groth16),
            "plonk" => builder.mode(SP1ProofMode::Plonk),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);

        let proof = builder.run()?;
        Ok((proof, vk))
    })
    .await;

    // === Step 4: Handle proof result ===
    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task panicked: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    // === Step 5: Decode public values ===
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesBtcHoldings::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // === Step 6: Build Response ===
    let response = BtcHoldingsResponse {
        total_btc: public_values.total_btc,
        org_hash: format!("0x{}", hex::encode(public_values.org_hash)),
        put_value: public_values.total_put_value.to_string(),
        call_value: public_values.total_call_value.to_string(),
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
        verifier_version: "0x1234abcd".to_string(), // TODO: Replace with actual version
    };

    HttpResponse::Ok().json(response)
}

#[post("/prove-doge-transaction")]
async fn prove_doge_transaction(req: web::Json<DogeTxRequest>) -> impl Responder {
    println!("Received Dogecoin transaction proof request: {:?}", req);

    // === Step 1: Fetch transaction details ===
    let tx_details = match fetch_doge_tx(&req.tx_hash).await {
        Ok(details) => details,
        Err(e) => {
            eprintln!("Failed to fetch transaction details: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Transaction fetch failed: {}", e));
        }
    };

    // === Step 2: Verify recipient address ===
    const EXPECTED_RECIPIENT: &str = "DPGGRKJaKtTkNhc6uodtdyQEyv8RsWxL6H";
    let mut total_doge = 0u64;
    let mut sender_address = String::new();

    for output in tx_details.outputs.iter() {
        if output.recipient == EXPECTED_RECIPIENT {
            total_doge = total_doge.saturating_add(output.value);
        }
    }

    if total_doge == 0 {
        return HttpResponse::BadRequest().body("No outputs found for the expected recipient address");
    }

    if let Some(input) = tx_details.inputs.first() {
        sender_address = input.recipient.clone();
    } else {
        return HttpResponse::BadRequest().body("No inputs found in the transaction");
    }

    // === Step 3: Clone for move into spawn_blocking ===
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();

    // === Step 4: Run proof generation in blocking task ===
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(DOGE_TX_ELF);

        // === Build stdin ===
        let mut stdin = SP1Stdin::new();

        let txid_decoded = hex::decode(&tx_hash).map_err(|e| anyhow::anyhow!("Invalid tx hash: {}", e))?;
        let txid_bytes: [u8; 32] = txid_decoded.try_into().map_err(|e| anyhow::anyhow!("Invalid tx hash length: {}", e))?;

        let input = DogeTxInput {
            txid: txid_bytes,
            recipient_address: EXPECTED_RECIPIENT.to_string(),
            sender_address: sender_address_clone,
            amount: total_doge,
        };

        stdin.write(&input);

        // === Prove ===
        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system.as_str() {
            "groth16" => builder.mode(SP1ProofMode::Groth16),
            "plonk" => builder.mode(SP1ProofMode::Plonk),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);

        let proof = builder.run()?;
        Ok((proof, vk))
    })
    .await;

    // === Step 5: Handle proof result ===
    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task panicked: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    // === Step 6: Decode public values ===
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesDogeTx::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // === Step 7: Build Response ===
    let response = DogeTxResponse {
        total_doge: public_values.total_doge,
        sender_address,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

async fn fetch_utxos(address: &str) -> Result<Vec<BlockstreamUtxo>, Box<dyn Error>> {
    let url = format!("https://blockstream.info/api/address/{}/utxo", address);
    let resp = reqwest::get(&url).await?;
    let utxos: Vec<BlockstreamUtxo> = resp.json().await?;
    Ok(utxos)
}

async fn fetch_doge_tx(tx_hash: &str) -> Result<BlockchairTx, Box<dyn Error>> {
    let url = format!("https://api.blockchair.com/dogecoin/dashboards/transaction/{}", tx_hash);
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }
    let json: serde_json::Value = resp.json().await?;
    let tx_data = json["data"][tx_hash]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid transaction data"))?;
    let tx: BlockchairTx = serde_json::from_value(serde_json::Value::Object(tx_data.clone()))?;
    Ok(tx)
}

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
            .service(prove_doge_transaction)
    })
    .workers(4)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}