use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use sp1_sdk::Prover; // needed for .setup() / .prove()
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesDogeTx, DogeTxInput};
use tokio::task;
use anyhow::Result;
use sp1_sdk::SP1ProofMode;
use anyhow::anyhow;
use sp1_sdk::network::FulfillmentStrategy;
use alloy_sol_types::SolType; // ✅ needed for abi_encode / abi_decode

#[allow(unused_variables, unused_imports, dead_code)]
pub const DOGE_TX_ELF: &[u8] = include_elf!("doge_tx-program");

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxRequest {
    owner_address: String,
    tx_hash: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxResponse {
    total_doge: u64,
    sender_address: String,
    owner_address: String, 
    tx_hash: String,
    vkey: String,
    public_values: String,
    proof: String,
}

// ---- External API structs ----
#[derive(Debug, Deserialize, Clone)]
struct BlockstreamUtxo {
    txid: String,
    vout: u32,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct UtxoValue {
    value: u64,
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

// ---- Handler ----
#[post("/prove-doge-transaction")]
async fn prove_doge_transaction(req: web::Json<DogeTxRequest>) -> impl Responder {
    println!("Received Dogecoin transaction proof request: {:?}", req);

    // 1) Fetch transaction details
    let tx_details = match fetch_doge_tx(&req.tx_hash).await {
        Ok(details) => details,
        Err(e) => {
            eprintln!("Failed to fetch transaction details: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Transaction fetch failed: {}", e));
        }
    };

    // 2) Verify recipient address and sum outputs
    const EXPECTED_RECIPIENT: &str = "DHGrS3MYGyKzRVdMNxziTPF7QXvaYoEndA";
    let mut total_doge = 0u64;
    let mut sender_address = String::new();

    for output in tx_details.outputs.iter() {
        if output.recipient == EXPECTED_RECIPIENT {
            total_doge = total_doge.saturating_add(output.value);
        }
    }

    if total_doge == 0 {
        return HttpResponse::BadRequest()
            .body("No outputs found for the expected recipient address");
    }

    if let Some(input) = tx_details.inputs.first() {
        sender_address = input.recipient.clone();
    } else {
        return HttpResponse::BadRequest().body("No inputs found in the transaction");
    }

    // 3) Clone fields BEFORE moving into blocking closure
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();
    let owner_address_plain = req.owner_address.clone();
    let owner_address_for_closure = owner_address_plain.clone(); // FIXED: Clone for closure
    let amount = total_doge;

    // 4) Prove (blocking)
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(DOGE_TX_ELF);

        // Build stdin
        let mut stdin = SP1Stdin::new();

        // txid as bytes (from hex)
        let txid_decoded = hex::decode(&tx_hash)
            .map_err(|e| anyhow!("Invalid tx hash: {}", e))?;
        let txid_bytes: [u8; 32] = txid_decoded
            .try_into()
            .map_err(|e| anyhow!("Invalid tx hash length: {:?}", e))?;

        // Prepare circuit input
        let input = DogeTxInput {
            txid: txid_bytes,
            recipient_address: EXPECTED_RECIPIENT.to_string(),
            sender_address: sender_address_clone,
            owner_address: owner_address_for_closure, // FIXED: Use the cloned version
            tx_hash: tx_hash.clone(),
            amount,
        };

        stdin.write(&input);

        // Prove
        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system.as_str() {
            "groth16" => builder.mode(SP1ProofMode::Groth16),
            "plonk" => builder.mode(SP1ProofMode::Plonk),
            _ => return Err(anyhow!("Invalid proof system")),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);

        let proof = builder.run()?;
        Ok((proof, vk, tx_hash))
    })
    .await;

    // 5) Handle proof result - FIXED: Match only 3 elements, not 4
    let (proof, vk, tx_hash_from_proof) = match proof_result {
        Ok(Ok((proof, vk, tx_hash))) => (proof, vk, tx_hash),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task panicked: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    // FIXED: Use the original owner_address_plain that wasn't moved
    let owner_address_from_proof = owner_address_plain;

    // 6) Decode public values
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesDogeTx::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // owner_address in PublicValuesDogeTx is a FixedBytes<32> (hashed owner).
    // let owner_address_hashed_hex =
    //     format!("0x{}", hex::encode(public_values.owner_address.as_slice()));

    // 7) Build response
    let response = DogeTxResponse {
        total_doge: public_values.total_doge,
        sender_address,
        owner_address: owner_address_from_proof,
        tx_hash: tx_hash_from_proof,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

// ---- External fetchers ----
async fn fetch_doge_tx(tx_hash: &str) -> Result<BlockchairTx, Box<dyn Error>> {
    let url = format!(
        "https://api.blockchair.com/dogecoin/dashboards/transaction/{}",
        tx_hash
    );
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }
    let json: serde_json::Value = resp.json().await?;
    let tx_data = json["data"][tx_hash]
        .as_object()
        .ok_or_else(|| anyhow!("Invalid transaction data"))?;
    let tx: BlockchairTx = serde_json::from_value(serde_json::Value::Object(tx_data.clone()))?;
    Ok(tx)
}

// ---- Main ----
#[tokio::main]
async fn main() -> std::io::Result<()> {
    setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:4000");

    HttpServer::new(|| {
        App::new()
            .service(prove_doge_transaction)
    })
    .workers(1)
    .bind(("0.0.0.0", 4000))?
    .run()
    .await
}


