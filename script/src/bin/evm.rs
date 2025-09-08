use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use sp1_sdk::Prover; // needed for .setup() / .prove()
use std::error::Error;
use hex;
use fibonacci_lib::{PublicValuesDogeTx, DogeTxInput, PublicValuesXrpTx,PublicValuesXrpBalance, XrpBalanceInput,PublicValuesCardanoTx, CardanoTxInput,PublicValuesLiteCoinHoldings, LiteCoinHoldingsInput,PublicValuesBitcoinCashHoldings, BitcoinCashHoldingsInput};
use tokio::task;
use anyhow::Result;
use sp1_sdk::SP1ProofMode;
use anyhow::anyhow;
use sp1_sdk::network::FulfillmentStrategy;
use alloy_sol_types::SolType; // ✅ needed for abi_encode / abi_decode
use fibonacci_lib::XrpTxInput;
 use serde_json::Value;
use bitcoin::{Address, Network, PublicKey as BitcoinPublicKey};
use bitcoin::secp256k1::{Secp256k1, PublicKey as SecpPublicKey};
use cashaddr::convert::from_legacy;  // ✅ Fixed import



#[allow(unused_variables, unused_imports, dead_code)]
pub const DOGE_TX_ELF: &[u8] = include_elf!("doge_tx-program");

#[allow(unused_variables, unused_imports, dead_code)]
pub const XRP_TX_ELF: &[u8] = include_elf!("Xrp_tx-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const XRP_BALANCE_ELF: &[u8] = include_elf!("Xrp_balance-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const CARDANO_TX_ELF: &[u8] = include_elf!("Cardano_tx-program");

#[allow(unused_variables, unused_imports, dead_code)]
pub const LITECOIN_TX_ELF: &[u8] = include_elf!("LiteCoin_tx-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const BITCOINCASH_TX_ELF: &[u8] = include_elf!("BitcoinCash_tx-program");

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CardanoTxRequest {
    owner_address: String,
    tx_hash: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiteCoinTxRequest {
    owner_address: String,
    tx_hash: String,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BitcoinCashTxRequest {
    owner_address: String,
    tx_hash: String,
    proof_system: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteCoinTxResponse {
    total_amount: u64,
    sender_address: String,
    owner_address: String, 
    tx_hash: String,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitcoinCashTxResponse {
    total_amount: u64,
    sender_address: String,
    owner_address: String, 
    tx_hash: String,
    vkey: String,
    public_values: String,
    proof: String,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DogeTxResponse {
    total_amount: u64,
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
    total_amount: u64,
    sender_address: String,
    owner_address: String, 
    tx_hash: String,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardanoTxResponse {
    total_amount: u64,
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
        total_amount: public_values.total_doge,
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
    const EXPECTED_RECIPIENT: &str = "rpJRDWw1M9jm7NRgrrSvHLJFWA7CaeaWuw";
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
        total_amount: public_values.total_xrp,
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

#[post("/prove-cardano-transaction")]
async fn prove_cardano_transaction(req: web::Json<CardanoTxRequest>) -> impl Responder {
    println!("🔍 Received Cardano transaction proof request: {:?}", req);

    // 1) Fetch transaction details using Tatum
    let tx_details = match fetch_cardano_tx(&req.tx_hash).await {
        Ok(details) => {
            println!("✅ Successfully fetched Cardano transaction from Tatum");
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
    let tx_json = &tx_details;

    // 3) Verify recipient address and extract amount
    const EXPECTED_RECIPIENT: &str = "addr1qyvxngqhhvzunlxlkw4f9m6nep00spqtmrlvfgynmrq5q7r0mjnf84mnk78ytza3sunyvqs3llehvfjuwvk338d69t2qqag5yl";
    println!("🎯 Expected recipient: {}", EXPECTED_RECIPIENT);

    // Extract sender address from inputs
    let inputs = tx_json.get("inputs").and_then(|v| v.as_array());
    let sender_address = match inputs.and_then(|inputs| inputs.first()) {
        Some(input) => {
            match input.get("address").and_then(|v| v.as_str()) {
                Some(addr) => {
                    println!("👤 Found sender address: {}", addr);
                    addr.to_string()
                },
                None => {
                    eprintln!("❌ No sender address found in first input");
                    return HttpResponse::BadRequest().body("No sender address found in the transaction");
                }
            }
        },
        None => {
            eprintln!("❌ No inputs found in transaction");
            return HttpResponse::BadRequest().body("No inputs found in the transaction");
        }
    };

    // Extract outputs to find recipient and amount - UPDATED FOR TATUM ADA API
    let outputs = tx_json.get("outputs").and_then(|v| v.as_array());
    let mut total_lovelace = 0u64;
    let mut found_recipient = false;

    match outputs {
        Some(outputs_array) => {
            for output in outputs_array {
                if let Some(address) = output.get("address").and_then(|v| v.as_str()) {
                    println!("🏠 Found output address: {}", address);
                    if address == EXPECTED_RECIPIENT {
                        found_recipient = true;
                        
                        // Tatum ADA API structure: outputs have direct "value" field
                        if let Some(value) = output.get("value").and_then(|v| v.as_str()) {
                            match value.parse::<u64>() {
                                Ok(lovelace) => {
                                    println!("💰 Found {} lovelace ({} ADA) for recipient", 
                                        lovelace, lovelace as f64 / 1_000_000.0);
                                    total_lovelace = total_lovelace.saturating_add(lovelace);
                                },
                                Err(e) => {
                                    eprintln!("❌ Failed to parse value '{}': {}", value, e);
                                    return HttpResponse::BadRequest().body("Invalid value format in output");
                                }
                            }
                        } else {
                            eprintln!("❌ No value field found in output for recipient");
                            return HttpResponse::BadRequest().body("No value found for recipient address");
                        }
                    }
                }
            }
        },
        None => {
            eprintln!("❌ No outputs found in transaction");
            return HttpResponse::BadRequest().body("No outputs found in the transaction");
        }
    }

    if !found_recipient {
        eprintln!("❌ Expected recipient address not found in transaction outputs");
        return HttpResponse::BadRequest()
            .body(format!("Transaction does not contain expected recipient address ({})", EXPECTED_RECIPIENT));
    }

    if total_lovelace == 0 {
        eprintln!("❌ No lovelace amount found for the expected recipient");
        return HttpResponse::BadRequest()
            .body("No amount found for the expected recipient address");
    }

    println!("✅ Transaction validation successful!");
    println!("📊 Summary - Sender: {}, Recipient: {}, Amount: {} lovelace", 
        sender_address, EXPECTED_RECIPIENT, total_lovelace);

    // 4) Clone fields BEFORE moving into blocking closure
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();
    let owner_address_plain = req.owner_address.clone();
    let owner_address_for_closure = owner_address_plain.clone();
    let amount = total_lovelace;

    println!("🔐 Starting proof generation with system: {}", proof_system);

    // 5) Prove (blocking)
    let proof_result = task::spawn_blocking(move || {
        println!("🏗️ Building prover client...");
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(CARDANO_TX_ELF);
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
        let input = CardanoTxInput {
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
    let public_values = match PublicValuesCardanoTx::abi_decode(public_bytes) {
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
    let response = CardanoTxResponse {
        total_amount: public_values.total_lovelace,
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

#[post("/prove-litecoin-transaction")]
async fn prove_litecoin_transaction(req: web::Json<LiteCoinTxRequest>) -> impl Responder {
    println!("🔍 Received Litecoin transaction proof request: {:?}", req);

    // 1) Fetch transaction details using Tatum
    let tx_details = match fetch_litecoin_tx(&req.tx_hash).await {
        Ok(details) => {
            println!("✅ Successfully fetched Litecoin transaction from Tatum");
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
    let tx_json = &tx_details;

    // 3) Verify recipient address and extract amount
    const EXPECTED_RECIPIENT: &str = "ltc1qqhvj3grzewmqjp3sehjqjacg2pwkukxmqp84sr";
    println!("🎯 Expected recipient: {}", EXPECTED_RECIPIENT);

    // Extract sender address from inputs (Tatum Litecoin structure)
    let inputs = tx_json.get("inputs").and_then(|v| v.as_array());
    let sender_address = match inputs.and_then(|inputs| inputs.first()) {
        Some(input) => {
            // In Tatum Litecoin API, sender address is in coin.address field
            match input.get("coin").and_then(|coin| coin.get("address")).and_then(|v| v.as_str()) {
                Some(addr) => {
                    println!("👤 Found sender address: {}", addr);
                    addr.to_string()
                },
                None => {
                    eprintln!("❌ No sender address found in input coin field");
                    return HttpResponse::BadRequest().body("No sender address found in the transaction");
                }
            }
        },
        None => {
            eprintln!("❌ No inputs found in transaction");
            return HttpResponse::BadRequest().body("No inputs found in the transaction");
        }
    };

    // Extract outputs to find recipient and amount (Tatum Litecoin structure)
    let outputs = tx_json.get("outputs").and_then(|v| v.as_array());
    let mut total_litecoin = 0u64;
    let mut found_recipient = false;

    match outputs {
        Some(outputs_array) => {
            for output in outputs_array {
                if let Some(address) = output.get("address").and_then(|v| v.as_str()) {
                    println!("🏠 Found output address: {}", address);
                    if address == EXPECTED_RECIPIENT {
                        found_recipient = true;
                        
                        // In Tatum Litecoin API, value is a string in LTC format (e.g., "0.07467455")
                        if let Some(value) = output.get("value").and_then(|v| v.as_str()) {
                            match value.parse::<f64>() {
                                Ok(ltc_amount) => {
                                    // Convert LTC to satoshi (1 LTC = 100,000,000 satoshi)
                                    let satoshi = (ltc_amount * 100_000_000.0) as u64;
                                    println!("💰 Found {} LTC ({} satoshi) for recipient", 
                                        ltc_amount, satoshi);
                                    total_litecoin = total_litecoin.saturating_add(satoshi);
                                },
                                Err(e) => {
                                    eprintln!("❌ Failed to parse LTC value '{}': {}", value, e);
                                    return HttpResponse::BadRequest().body("Invalid LTC value format in output");
                                }
                            }
                        } else {
                            eprintln!("❌ No value field found in output for recipient");
                            return HttpResponse::BadRequest().body("No value found for recipient address");
                        }
                    }
                }
            }
        },
        None => {
            eprintln!("❌ No outputs found in transaction");
            return HttpResponse::BadRequest().body("No outputs found in the transaction");
        }
    }

    if !found_recipient {
        eprintln!("❌ Expected recipient address not found in transaction outputs");
        return HttpResponse::BadRequest()
            .body(format!("Transaction does not contain expected recipient address ({})", EXPECTED_RECIPIENT));
    }

    if total_litecoin == 0 {
        eprintln!("❌ No litecoin amount found for the expected recipient");
        return HttpResponse::BadRequest()
            .body("No amount found for the expected recipient address");
    }

    println!("✅ Transaction validation successful!");
    println!("📊 Summary - Sender: {}, Recipient: {}, Amount: {} satoshi ({} LTC)", 
        sender_address, EXPECTED_RECIPIENT, total_litecoin, total_litecoin as f64 / 100_000_000.0);

    // 4) Clone fields BEFORE moving into blocking closure
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();
    let owner_address_plain = req.owner_address.clone();
    let owner_address_for_closure = owner_address_plain.clone();
    let amount = total_litecoin;

    println!("🔐 Starting proof generation with system: {}", proof_system);

    // 5) Prove (blocking)
    let proof_result = task::spawn_blocking(move || {
        println!("🏗️ Building prover client...");
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(LITECOIN_TX_ELF);
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
        let input = LiteCoinHoldingsInput {
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
    let public_values = match PublicValuesLiteCoinHoldings::abi_decode(public_bytes) {
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
    let response = LiteCoinTxResponse {
        total_amount: public_values.total_litecoin,
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

#[post("/prove-bitcoincash-transaction")]
async fn prove_bitcoincash_transaction(req: web::Json<BitcoinCashTxRequest>) -> impl Responder {
    println!("🔍 Received Bitcoin Cash transaction proof request: {:?}", req);

    // 1) Fetch transaction details using Tatum
    let tx_details = match fetch_bch_tx(&req.tx_hash).await {
        Ok(details) => {
            println!("✅ Successfully fetched Bitcoin Cash transaction from Tatum");
            details
        },
        Err(e) => {
            eprintln!("❌ Failed to fetch transaction details: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Transaction fetch failed: {}", e));
        }
    };

    // Check for API errors first
    if let Some(error) = tx_details.get("error") {
        eprintln!("🚫 Tatum API returned error: {}", error);
        return HttpResponse::BadRequest()
            .body(format!("Tatum API error: {}", error));
    }

    let tx_json = &tx_details;

    // Expected recipient address
    const EXPECTED_RECIPIENT: &str = "qqv2smfg2yd2u3e3dt0ype63cw06lqcl3c0jlhw8v3";
    println!("🎯 Expected recipient: {}", EXPECTED_RECIPIENT);

    // ENHANCED SENDER ADDRESS EXTRACTION WITH BCH FORMAT
    let inputs = tx_json.get("vin").and_then(|v| v.as_array());
    let sender_address = match inputs.and_then(|inputs| inputs.first()) {
        Some(input) => {
            // Method 1: Try prevout.scriptPubKey.addresses (standard UTXO)
            if let Some(addr) = input.get("prevout")
                .and_then(|prevout| prevout.get("scriptPubKey"))
                .and_then(|script| script.get("addresses"))
                .and_then(|addresses| addresses.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str()) 
            {
                println!("👤 Found sender address from prevout: {}", addr);
                addr.to_string() // Already in bitcoincash: format
            }
            // Method 2: Extract from scriptSig.asm (MAIN METHOD FOR RAW TRANSACTIONS)
            else if let Some(asm) = input.get("scriptSig")
                .and_then(|sig| sig.get("asm"))
                .and_then(|a| a.as_str())
            {
                if let Some(addr) = extract_address_from_scriptsig_asm(asm) {
                    println!("👤 Successfully derived sender address from scriptSig: {}", addr);
                    addr
                } else {
                    println!("⚠️ Could not derive address from scriptSig ASM, using fallback");
                    "Unknown Sender".to_string()
                }
            }
            else {
                println!("⚠️ Could not determine sender address from any method");
                "Unknown Sender".to_string()
            }
        },
        None => {
            eprintln!("❌ No inputs found in transaction");
            return HttpResponse::BadRequest().body("No inputs found in the transaction");
        }
    };

    // Extract outputs to find recipient and amount
    let outputs = tx_json.get("vout").and_then(|v| v.as_array());
    let mut total_bitcoincash = 0u64;
    let mut found_recipient = false;

    match outputs {
        Some(outputs_array) => {
            for output in outputs_array {
                if let Some(script_pub_key) = output.get("scriptPubKey") {
                    if let Some(addresses) = script_pub_key.get("addresses").and_then(|v| v.as_array()) {
                        for address in addresses {
                            if let Some(addr_str) = address.as_str() {
                                let clean_addr = addr_str
                                    .strip_prefix("bitcoincash:")
                                    .unwrap_or(addr_str);
                                
                                if clean_addr == EXPECTED_RECIPIENT {
                                    found_recipient = true;
                                    
                                    if let Some(value) = output.get("value").and_then(|v| v.as_f64()) {
                                        let satoshi = (value * 100_000_000.0) as u64;
                                        println!("💰 Found {} BCH ({} satoshi) for recipient", 
                                            value, satoshi);
                                        total_bitcoincash = total_bitcoincash.saturating_add(satoshi);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        None => {
            eprintln!("❌ No outputs found in transaction");
            return HttpResponse::BadRequest().body("No outputs found in the transaction");
        }
    }

    if !found_recipient {
        return HttpResponse::BadRequest()
            .body(format!("Transaction does not contain expected recipient address"));
    }

    if total_bitcoincash == 0 {
        return HttpResponse::BadRequest()
            .body("No amount found for the expected recipient address");
    }

    println!("✅ Transaction validation successful!");
    println!("📊 Summary - Sender: {}, Recipient: {}, Amount: {} satoshi ({} BCH)", 
        sender_address, EXPECTED_RECIPIENT, total_bitcoincash, total_bitcoincash as f64 / 100_000_000.0);

    // Clone fields for proof generation
    let tx_hash = req.tx_hash.clone();
    let proof_system = req.proof_system.clone();
    let sender_address_clone = sender_address.clone();
    let owner_address_plain = req.owner_address.clone();
    let owner_address_for_closure = owner_address_plain.clone();
    let amount = total_bitcoincash;

    // Generate proof in blocking task
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::builder().network().build();
        let (pk, vk) = client.setup(BITCOINCASH_TX_ELF);

        let mut stdin = SP1Stdin::new();

        let txid_decoded = hex::decode(&tx_hash)
            .map_err(|e| anyhow!("Invalid tx hash: {}", e))?;
        let txid_bytes: [u8; 32] = txid_decoded
            .try_into()
            .map_err(|e| anyhow!("Invalid tx hash length: {:?}", e))?;

        let input = BitcoinCashHoldingsInput {
            txid: txid_bytes,
            recipient_address: EXPECTED_RECIPIENT.to_string(),
            sender_address: sender_address_clone,
            owner_address: owner_address_for_closure,
            tx_hash: tx_hash.clone(),
            amount,
        };

        stdin.write(&input);

        let builder = client.prove(&pk, &stdin);
        let builder = match proof_system.as_str() {
            "groth16" => builder.mode(SP1ProofMode::Groth16),
            "plonk" => builder.mode(SP1ProofMode::Plonk),
            _ => return Err(anyhow!("Invalid proof system: {}", proof_system)),
        };

        let builder = builder.strategy(FulfillmentStrategy::Hosted);
        let proof = builder.run()?;
        
        Ok((proof, vk, tx_hash))
    })
    .await;

    // Handle proof result
    let (proof, vk, tx_hash_from_proof) = match proof_result {
        Ok(Ok((proof, vk, tx_hash))) => (proof, vk, tx_hash),
        Ok(Err(e)) => {
            return HttpResponse::InternalServerError()
                .body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    // Decode public values
    let public_bytes = proof.public_values.as_slice();
    let public_values = match PublicValuesBitcoinCashHoldings::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    // Build response
    let response = BitcoinCashTxResponse {
        total_amount: public_values.total_bitcoin_cash,
        sender_address,
        owner_address: owner_address_plain,
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
//This function fetches XRP transaction details from the XRP Ledger using its public API.
async fn fetch_xrp_tx(tx_hash: &str) -> Result<Value, Box<dyn Error>> {
    // Tatum XRP Mainnet API endpoint
    let url = format!("https://api.tatum.io/v3/xrp/transaction/{}", tx_hash);
    
    println!("🔍 Querying Tatum XRP Mainnet API: {}", url);
    
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("x-api-key", "t-68b034bfb63d86a61dd9f14e-1fd455540285454a99724f2b") // Your API key
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

async fn fetch_bch_tx(tx_hash: &str) -> Result<Value, Box<dyn Error>> {
    // Tatum Bitcoin Cash API endpoint
    let url = format!("https://api.tatum.io/v3/bcash/transaction/{}", tx_hash);
    println!("🔍 Querying Tatum Bitcoin Cash API: {}", url);
    
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("x-api-key", "t-68b034bfb63d86a61dd9f14e-1fd455540285454a99724f2b")
        .header("accept", "application/json")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;
    
    println!("📡 Response status: {}", resp.status());
    
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }
    
    let json: Value = resp.json().await?;
    println!("📥 Tatum Bitcoin Cash response: {}", serde_json::to_string_pretty(&json)?);
    
    // Return the FULL JSON data, not just the address
    Ok(json)
}

async fn fetch_cardano_tx(tx_hash: &str) -> Result<Value, Box<dyn Error>> {
    // Tatum Cardano API endpoint - using the correct ADA endpoint
    let url = format!("https://api.tatum.io/v3/ada/transaction/{}", tx_hash);

    println!("🔍 Querying Tatum Cardano API: {}", url);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("x-api-key", "t-68b034bfb63d86a61dd9f14e-1fd455540285454a99724f2b") // Your API key
        .header("accept", "application/json")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;

    println!("📡 Response status: {}", resp.status());

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }
    

 

    let json: Value = resp.json().await?;
    println!("📥 Tatum Cardano response: {}", serde_json::to_string_pretty(&json)?);

    Ok(json)
}




async fn xrp_balance_fetch(address: &str) -> Result<u64, Box<dyn Error>> {
    let url = format!("https://api.tatum.io/v3/xrp/account/{}/balance", address);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("x-api-key", "t-68b034bfb63d86a61dd9f14e-1fd455540285454a99724f2b") // Your API key
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

async fn fetch_litecoin_tx(tx_hash: &str) -> Result<Value, Box<dyn Error>> {
    // Tatum Litecoin API endpoint
    let url = format!("https://api.tatum.io/v3/litecoin/transaction/{}", tx_hash);

    println!("🔍 Querying Tatum Litecoin API: {}", url);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("x-api-key", "t-68b034bfb63d86a61dd9f14e-1fd455540285454a99724f2b") // Your API key
        .header("accept", "application/json")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;

    println!("📡 Response status: {}", resp.status());

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch transaction: {}", resp.status()).into());
    }

    let json: Value = resp.json().await?;
    println!("📥 Tatum Litecoin response: {}", serde_json::to_string_pretty(&json)?);

    Ok(json)
}


// Function to extract Bitcoin Cash address from scriptSig asm
// Function to extract Bitcoin Cash address from scriptSig asm
fn extract_address_from_scriptsig_asm(asm: &str) -> Option<String> {
    println!("🔍 Analyzing scriptSig ASM: {}", asm);
    
    let parts: Vec<&str> = asm.split(' ').collect();
    
    for part in parts.iter().rev() {
        let clean_pubkey = part.split('[').next().unwrap_or(part);
        
        if clean_pubkey.len() < 60 || clean_pubkey.starts_with("30") {
            continue;
        }
        
        println!("🔑 Trying to decode pubkey: {}", clean_pubkey);
        
        if let Ok(pubkey_bytes) = hex::decode(clean_pubkey) {
            if pubkey_bytes.len() == 33 || pubkey_bytes.len() == 65 {
                if let Ok(secp_pubkey) = SecpPublicKey::from_slice(&pubkey_bytes) {
                    let btc_pubkey = BitcoinPublicKey::new(secp_pubkey);
                    let address = Address::p2pkh(&btc_pubkey, Network::Bitcoin);
                    let legacy_addr = address.to_string();
                    
                    println!("✅ Generated legacy address: {}", legacy_addr);
                    
                    if let Some(bch_addr) = convert_to_cashaddr(&legacy_addr) {
                        println!("✅ Converted to BCH address: {}", bch_addr);
                        return Some(bch_addr);
                    }
                }
            }
        }
    }
    
    println!("❌ Could not extract address from scriptSig ASM");
    None
}

// Convert legacy Bitcoin address to Bitcoin Cash format
fn convert_to_cashaddr(legacy_addr: &str) -> Option<String> {
    match from_legacy(legacy_addr, "bitcoincash") {
        Ok(bch_addr) => {
            // Remove the "bitcoincash:" prefix if present
            let clean_addr = bch_addr.strip_prefix("bitcoincash:").unwrap_or(&bch_addr);
            Some(clean_addr.to_string())
        },
        Err(e) => {
            println!("❌ Failed to convert to cashaddr: {:?}", e);
            None
        }
    }
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
            .service(prove_cardano_transaction)
            .service(prove_litecoin_transaction)
            .service(prove_bitcoincash_transaction)
    })
    .workers(1)
    .bind(("0.0.0.0", 4000))?
    .run()
    .await
}


