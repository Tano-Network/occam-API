use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use sp1_sdk::Prover; // needed for .setup() / .prove()
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesDogeTx, DogeTxInput, PublicValuesXrpTx,PublicValuesXrpBalance, XrpBalanceInput};
use tokio::task;
use anyhow::Result;
use sp1_sdk::SP1ProofMode;
use anyhow::anyhow;
use sp1_sdk::network::FulfillmentStrategy;
use alloy_sol_types::SolType; // ✅ needed for abi_encode / abi_decode
use fibonacci_lib::XrpTxInput;
 use serde_json::Value;
 



#[allow(unused_variables, unused_imports, dead_code)]
pub const DOGE_TX_ELF: &[u8] = include_elf!("doge_tx-program");

#[allow(unused_variables, unused_imports, dead_code)]
pub const XRP_TX_ELF: &[u8] = include_elf!("Xrp_tx-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const XRP_BALANCE_ELF: &[u8] = include_elf!("Xrp_balance-program");

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxRequest {
    owner_address: String,
    tx_hash: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct XrpBalanceRequest {
    address: String,
    
    proof_system: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct XrpTxRequest {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XrpBalanceResponse {
    total_xrp: u64,
    address: String, 
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XrpTxResponse {
    total_xrp: u64,
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
#[post("/prove-xrp-transaction")]
async fn prove_xrp_transaction(req: web::Json<XrpTxRequest>) -> impl Responder {
    println!("🔍 Received XRP transaction proof request: {:?}", req);

    // 1) Fetch transaction details using Tatum
    let tx_details = match fetch_xrp_tx(&req.tx_hash).await {
        Ok(details) => {
            println!("✅ Successfully fetched XRP transaction from Tatum");
            details
        },
        Err(e) => {
            eprintln!("❌ Failed to fetch transaction details: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Transaction fetch failed: {}", e));
        }
    };

    // ADD DEBUG LOG FOR FULL RESPONSE
    println!("📋 Full Tatum API Response: {}", 
        serde_json::to_string_pretty(&tx_details)
            .unwrap_or_else(|_| "Failed to serialize response".to_string())
    );

    // Check for API errors first
    if let Some(error) = tx_details.get("error") {
        eprintln!("🚫 Tatum API returned error: {}", error);
        return HttpResponse::BadRequest()
            .body(format!("Tatum API error: {}", error));
    }

    // List available top-level fields
    if let Some(obj) = tx_details.as_object() {
        println!("📝 Available top-level fields: {:?}", obj.keys().collect::<Vec<_>>());
    }

    // 2) Extract transaction data from Tatum response
    // Tatum returns the transaction data directly at the root level, not wrapped in "transaction"
    let tx_json = &tx_details;
    
    println!("🔍 Transaction JSON: {}", 
        serde_json::to_string_pretty(tx_json)
            .unwrap_or_else(|_| "Failed to serialize transaction".to_string())
    );

    // 3) Verify recipient address and extract amount
    const EXPECTED_RECIPIENT: &str = "rLAc6d8QtzMMhp1ziGvBGzLk81gDfM25du";
    println!("🎯 Expected recipient: {}", EXPECTED_RECIPIENT);

    let sender_address = match tx_json.get("Account").and_then(|v| v.as_str()) {
        Some(addr) => {
            println!("👤 Found sender address: {}", addr);
            addr.to_string()
        },
        None => {
            eprintln!("❌ No sender address found in transaction");
            if let Some(obj) = tx_json.as_object() {
                eprintln!("Available transaction fields: {:?}", obj.keys().collect::<Vec<_>>());
            }
            return HttpResponse::BadRequest().body("No sender address found in the transaction");
        }
    };

    let destination = match tx_json.get("Destination").and_then(|v| v.as_str()) {
        Some(dest) => {
            println!("🏠 Found destination address: {}", dest);
            dest
        },
        None => {
            eprintln!("❌ No destination address found in transaction");
            if let Some(obj) = tx_json.as_object() {
                eprintln!("Available transaction fields: {:?}", obj.keys().collect::<Vec<_>>());
            }
            return HttpResponse::BadRequest().body("No destination address found in the transaction");
        }
    };

    if destination != EXPECTED_RECIPIENT {
        eprintln!("❌ Destination mismatch: expected {}, got {}", EXPECTED_RECIPIENT, destination);
        return HttpResponse::BadRequest()
            .body(format!("Transaction destination ({}) does not match expected recipient address ({})", destination, EXPECTED_RECIPIENT));
    }
    println!("✅ Destination address matches expected recipient");

    // Extract amount (XRP amounts are in drops, 1 XRP = 1,000,000 drops)
    let amount_field = tx_json.get("Amount");
    println!("💰 Amount field: {:?}", amount_field);

    let total_xrp = match amount_field.and_then(|v| v.as_str()) {
        Some(amount_str) => {
            println!("💵 Amount string: {}", amount_str);
            match amount_str.parse::<u64>() {
                Ok(drops) => {
                    println!("✅ Parsed amount: {} drops ({} XRP)", drops, drops as f64 / 1_000_000.0);
                    drops
                },
                Err(e) => {
                    eprintln!("❌ Failed to parse amount '{}': {}", amount_str, e);
                    return HttpResponse::BadRequest().body("Invalid amount format in transaction");
                }
            }
        },
        None => {
            eprintln!("❌ No amount found or amount is not a string");
            // Try to get it as a number
            match amount_field.and_then(|v| v.as_u64()) {
                Some(amount_num) => {
                    println!("✅ Found amount as number: {} drops", amount_num);
                    amount_num
                },
                None => {
                    eprintln!("Amount field type: {:?}", amount_field.map(|v| v.to_string()));
                    return HttpResponse::BadRequest().body("No amount found in the transaction");
                }
            }
        }
    };

    println!("🎉 Transaction validation successful!");
    println!("📊 Summary - Sender: {}, Recipient: {}, Amount: {} drops", 
        sender_address, destination, total_xrp);

    // 4) Clone fields BEFORE moving into blocking closure
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();
    let owner_address_plain = req.owner_address.clone();
    let owner_address_for_closure = owner_address_plain.clone();
    let amount = total_xrp;

    println!("🔐 Starting proof generation with system: {}", proof_system);

    // 5) Prove (blocking)
    let proof_result = task::spawn_blocking(move || {
        println!("🏗️ Building prover client...");
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(XRP_TX_ELF);
        println!("✅ Prover client setup complete");

        // Build stdin
        let mut stdin = SP1Stdin::new();

        // txid as bytes (from hex)
        let txid_decoded = hex::decode(&tx_hash)
            .map_err(|e| anyhow!("Invalid tx hash: {}", e))?;
        let txid_bytes: [u8; 32] = txid_decoded
            .try_into()
            .map_err(|e| anyhow!("Invalid tx hash length: {:?}", e))?;

        println!("🔑 Prepared circuit input");

        // Prepare circuit input
        let input = XrpTxInput {
            txid: txid_bytes,
            recipient_address: EXPECTED_RECIPIENT.to_string(),
            sender_address: sender_address_clone,
            owner_address: owner_address_for_closure,
            tx_hash: tx_hash.clone(),
            amount,
        };

        stdin.write(&input);

        // Prove
        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system.as_str() {
            "groth16" => {
                println!("🔒 Using Groth16 proof system");
                builder.mode(SP1ProofMode::Groth16)
            },
            "plonk" => {
                println!("🔒 Using PLONK proof system");
                builder.mode(SP1ProofMode::Plonk)
            },
            _ => return Err(anyhow!("Invalid proof system: {}", proof_system)),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);

        println!("⚡ Running proof generation...");
        let proof = builder.run()?;
        println!("✅ Proof generation complete!");
        
        Ok((proof, vk, tx_hash))
    })
    .await;

    // 6) Handle proof result
    let (proof, vk, tx_hash_from_proof) = match proof_result {
        Ok(Ok((proof, vk, tx_hash))) => {
            println!("🎉 Proof generation successful!");
            (proof, vk, tx_hash)
        },
        Ok(Err(e)) => {
            eprintln!("❌ Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("💥 Proof generation task panicked: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let owner_address_from_proof = owner_address_plain;

    // 7) Decode public values
    println!("🔍 Decoding public values...");
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesXrpTx::abi_decode(public_bytes) {
        Ok(val) => {
            println!("✅ Successfully decoded public values");
            val
        },
        Err(e) => {
            eprintln!("❌ Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // 8) Build response
    let response = XrpTxResponse {
        total_xrp: public_values.total_xrp,
        sender_address,
        owner_address: owner_address_from_proof,
        tx_hash: tx_hash_from_proof,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    println!("🚀 Sending successful response");
    HttpResponse::Ok().json(response)
}


#[post("/prove-xrp-balance")]
async fn prove_xrp_balance(req: web::Json<XrpBalanceRequest>) -> impl Responder {
    println!("🔍 Received XRP balance proof request: {:?}", req);

    // Validate XRP address format
    if !req.address.starts_with('r') || req.address.len() < 25 || req.address.len() > 34 {
        eprintln!("❌ Invalid XRP address format: {}", req.address);
        return HttpResponse::BadRequest().body("Invalid XRP address format");
    }

    // Validate proof system
    if req.proof_system != "groth16" && req.proof_system != "plonk" {
        eprintln!("❌ Invalid proof system: {}", req.proof_system);
        return HttpResponse::BadRequest().body("Invalid proof system. Use 'groth16' or 'plonk'");
    }

    println!("🔄 Fetching XRP balance for {}", req.address);

    // 1) Fetch balance using your existing function
    let balance_drops = match xrp_balance_fetch(&req.address).await {
        Ok(balance) => {
            println!("✅ Successfully fetched balance: {} drops ({} XRP)", 
                balance, balance as f64 / 1_000_000.0);
            balance
        },
        Err(e) => {
            eprintln!("❌ Failed to fetch balance: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Balance fetch failed: {}", e));
        }
    };

    // 2) Clone data for the blocking task
    let address_clone = req.address.clone();
    let proof_system_clone = req.proof_system.clone();

    println!("🔐 Starting proof generation with system: {}", proof_system_clone);

    // 3) Generate proof in blocking task
    let proof_result = task::spawn_blocking(move || {
        println!("🏗️ Building prover client...");
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(XRP_BALANCE_ELF);
        println!("✅ Prover client setup complete");

        // Build stdin
        let mut stdin = SP1Stdin::new();

        // Create circuit input with simplified struct
        let input = XrpBalanceInput {
            address: address_clone.clone(),
            amount: balance_drops, // XRP in drops
        };

        println!("🔑 Prepared circuit input for address: {} with amount: {} drops", 
            address_clone, balance_drops);
        stdin.write(&input);

        // Configure proof mode
        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system_clone.as_str() {
            "groth16" => {
                println!("🔒 Using Groth16 proof system");
                builder.mode(SP1ProofMode::Groth16)
            },
            "plonk" => {
                println!("🔒 Using PLONK proof system");
                builder.mode(SP1ProofMode::Plonk)
            },
            _ => return Err(anyhow!("Invalid proof system: {}", proof_system_clone)),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);

        println!("⚡ Running proof generation for XRP balance...");
        let proof = builder.run()?;
        println!("✅ Balance proof generation complete!");
        
        Ok((proof, vk, address_clone))
    })
    .await;

    // 4) Handle proof result
    let (proof, vk, address_from_proof) = match proof_result {
        Ok(Ok((proof, vk, address))) => {
            println!("🎉 Balance proof generation successful!");
            (proof, vk, address)
        },
        Ok(Err(e)) => {
            eprintln!("❌ Balance proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("💥 Balance proof generation task panicked: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    // 5) Decode public values
    println!("🔍 Decoding public values for balance proof...");
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesXrpBalance::abi_decode(public_bytes) {
        Ok(val) => {
            println!("✅ Successfully decoded balance proof public values");
            val
        },
        Err(e) => {
            eprintln!("❌ Decoding balance proof public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // 6) Build response
    let response = XrpBalanceResponse {
        total_xrp: balance_drops, // Use the fetched balance directly
        address: address_from_proof,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    println!("🚀 Sending successful balance proof response for {}", req.address);
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
//This function fetches XRP transaction details from the XRP Ledger using its public API.
async fn fetch_xrp_tx(tx_hash: &str) -> Result<Value, Box<dyn Error>> {
    // Tatum XRP Testnet API endpoint
    let url = format!("https://api.tatum.io/v3/xrp/transaction/{}", tx_hash);
    
    println!("🔍 Querying Tatum XRP Testnet API: {}", url);
    
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("x-api-key", "t-6863d36d6ce4fd5a9dff5947-0eea1990a01e4081a4b5c2e6") // Your API key
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;
    
    println!("📡 Response status: {}", resp.status());
    
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }
    
    let json: Value = resp.json().await?;
    println!("📥 Tatum response: {}", serde_json::to_string_pretty(&json)?);
    
    // Tatum returns transaction data directly, not wrapped in "result"
    Ok(json)
}


async  fn xrp_balance_fetch(address: &str) -> Result<u64, Box<dyn Error>> {

    let url = format!("https://api.tatum.io/v3/xrp/account/{}/balance", address);
    
    let client = reqwest::Client::new();
     let response = client
        .get(&url)
        .header("x-api-key", "t-6863d36d6ce4fd5a9dff5947-0eea1990a01e4081a4b5c2e6") // Your API key
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;

    if response.status() == 404 {
        return Ok(0); // Account not found
    }

    let json: serde_json::Value = response.json().await?;
    let balance_xrp: f64 = json["balance"].as_str().unwrap_or("0").parse()?;
    let balance_drops = (balance_xrp * 1_000_000.0) as u64;
    
    Ok(balance_drops)
}






// ---- Main ----
#[tokio::main]
async fn main() -> std::io::Result<()> {
    setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:4000");

    HttpServer::new(|| {
        App::new()
            .service(prove_doge_transaction)
            .service(prove_xrp_transaction)
            .service(prove_xrp_balance)
    })
    .workers(1)
    .bind(("0.0.0.0", 4000))?
    .run()
    .await
}


