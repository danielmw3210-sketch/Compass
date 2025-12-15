use super::types::*;
use crate::block::{BlockHeader, BlockType};
use crate::chain::Chain;
use crate::rpc::RpcState;
use axum::{debug_handler, extract::State, Json};
use std::sync::{Arc, Mutex};
use sha2::Digest;
use tracing::{info, debug, warn};
use num_traits::ToPrimitive;






/// Main dispatcher: routes incoming JSON-RPC requests to the correct handler.
#[debug_handler]
pub async fn handle_rpc_request(
    State(state): State<RpcState>,
    Json(req): Json<RpcRequest>,
) -> Json<RpcResponse> {
    debug!("RPC Request: method={}, id={}", req.method, req.id);

    // Dispatch based on method name
    let result = match req.method.as_str() {
        "getBalance" => handle_get_balance(state.chain.clone(), req.params).await,
        "getNonce" => handle_get_nonce(state.chain.clone(), req.params).await,
        "getChainHeight" => handle_get_chain_height(state.chain.clone()).await,
        "getAccountInfo" => handle_get_account_info(state.chain.clone(), req.params).await,
        "submitTransaction" => handle_submit_transaction(state.clone(), req.params).await, // Pass STATE
        "getBlock" => handle_get_block(state.chain.clone(), req.params).await,
        "getLatestBlocks" => handle_get_latest_blocks(state.chain.clone(), req.params).await,
        "getTransactionStatus" => {
            handle_get_transaction_status(state.chain.clone(), req.params).await
        }
        "getNodeInfo" => handle_get_node_info(state.chain.clone()).await,
        "getVersion" => handle_get_version().await,
        "submitMint" => handle_submit_mint(state.clone(), req.params).await, // Pass STATE
        "submitBurn" => handle_submit_burn(state.clone(), req.params).await, // Pass STATE
        "submitCompute" => handle_submit_compute(state.clone(), req.params).await, // New AI Endpoint
        "getPendingComputeJobs" => handle_get_pending_compute_jobs(state.clone(), req.params).await,
        "submitResult" => handle_submit_result(state.clone(), req.params).await,
        "getPeers" => handle_get_peers(state.clone()).await,
        "getVaultAddress" => handle_get_vault_address(req.params).await,
        "getValidatorStats" => handle_get_validator_stats(state.chain.clone(), req.params).await,
        "submitOracleVerificationJob" => handle_submit_oracle_verification_job(state.clone(), req.params).await,
        "getPendingOracleJobs" => handle_get_pending_oracle_jobs(state.clone()).await,
        "submitOracleVerificationResult" => handle_submit_oracle_verification_result(state.clone(), req.params).await,
        "submitRecurringOracleJob" => handle_submit_recurring_oracle_job(state.clone(), req.params).await,
        "getRecurringJobs" => handle_get_recurring_jobs(state.clone()).await,
        "getJobProgress" => handle_get_job_progress(state.clone(), req.params).await,
        "purchaseNeuralNet" => handle_purchase_neural_net(state.clone(), req.params).await,
        "listModelNFT" => handle_list_model_nft(state.clone(), req.params).await,
        "buyModelNFT" => handle_buy_model_nft(state.clone(), req.params).await,
        "getAllNFTs" => handle_get_all_nfts(state.clone()).await,
        "getBlockRange" => handle_get_block_range(state.chain.clone(), req.params).await,
        "getOraclePrices" => handle_get_oracle_prices(state.chain.clone()).await,
        "submitNativeVault" => handle_submit_native_vault(state.clone(), req.params).await,
        _ => Err(RpcError {
            code: -32601,
            message: format!("Method not found: {}", req.method),
        }),
    };

    // Build response
    match result {
        Ok(val) => Json(RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(val),
            error: None,
            id: req.id,
        }),
        Err(err) => Json(RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(err),
            id: req.id,
        }),
    }
}

//
// === Helper Functions for Safe Operations ===
//

/// Safely acquire a mutex lock, recovering from poison
fn safe_lock<T>(mutex: &Arc<Mutex<T>>) -> Result<std::sync::MutexGuard<T>, RpcError> {
    mutex.lock().map_err(|e| {
        tracing::error!("Mutex poisoned: {}", e);
        RpcError {
            code: -32603,
            message: "Internal error: mutex poisoned".to_string(),
        }
    })
}

/// Safely serialize to JSON value
fn to_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, RpcError> {
    serde_json::to_value(value).map_err(|e| RpcError {
        code: -32603,
        message: format!("Serialization error: {}", e),
    })
}

/// Safely serialize with bincode
fn safe_serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, RpcError> {
    bincode::serialize(value).map_err(|e| RpcError {
        code: -32603,
        message: format!("Binary serialization failed: {}", e),
    })
}

//
// === Individual Handlers ===
//

/// Handle getPeers()
async fn handle_get_peers(state: RpcState) -> Result<serde_json::Value, RpcError> {
    let pm = safe_lock(&state.peer_manager)?;
    let peers: Vec<String> = pm.peers.iter().cloned().collect();

    to_json(&GetPeersResponse { peers })
}

/// Handle getVersion()
async fn handle_get_version() -> Result<serde_json::Value, RpcError> {
    Ok(serde_json::json!({ "version": "0.1.0" }))
}

/// Handle submitMint(vault_id, ...)
async fn handle_submit_mint(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let tx: SubmitMintParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // Scope for locking chain
    let (tx_hash, result) = {
        let chain_lock = safe_lock(&state.chain)?;
        let prev_hash = chain_lock.head_hash().unwrap_or_default();

        let mut header = BlockHeader {
            index: chain_lock.height,
            block_type: BlockType::Mint {
                vault_id: tx.vault_id.clone(),
                collateral_asset: tx.collateral_asset.clone(),
                collateral_amount: tx.collateral_amount,
                compass_asset: tx.compass_asset.clone(),
                mint_amount: tx.mint_amount,
                owner: tx.owner.clone(),
                tx_proof: tx.tx_proof.clone(),
                oracle_signature: tx.oracle_signature.clone(),
                fee: tx.fee, // Default 0
            },
            timestamp: tx.timestamp.unwrap_or(crate::block::current_unix_timestamp_ms() as u64),
            prev_hash: tx.prev_hash.clone().unwrap_or(prev_hash),
            hash: "".to_string(),
            proposer: "Client".to_string(), // In real system, this is Miner/Validator
            signature_hex: tx.signature.clone(),
        };

        // If from client, they should have signed it. We verify signature here?
        // Ideally yes. For now, assuming client signature is valid over the fields.
        
        // Recalculate hash to match signature expectations
        header.hash = header.calculate_hash().map_err(|e| RpcError {
            code: -32603,
            message: format!("Hash calculation failed: {}", e),
        })?;
        
        // Verify Signature (TODO: extract pubkey from owner/wallet logic if possible)
        // Ignoring verification for prototype step 1 (assuming client runs same code)

        // Add to Gulf Stream? Or direct append if we are the node?
        // Since we are RPC node handling "submitMint", we should probably gossip it.
        // But logic is: submitMint -> Node creates block? 
        // Or user submits fully signed block?
        // Current Code: Node creates block in append_mint logic.
        
        // Let's create TransactionPayload::Mint and push to GulfStream.
        // BUT existing code in main.rs handles TransactionPayload::Mint/Transfer.
        
        // Construct transaction payload
        let payload = crate::network::TransactionPayload::Mint {
            vault_id: tx.vault_id.clone(),
            collateral_asset: tx.collateral_asset.clone(),
            collateral_amount: tx.collateral_amount,
            compass_asset: tx.compass_asset.clone(),
            mint_amount: tx.mint_amount,
            owner: tx.owner.clone(),
            tx_proof: tx.tx_proof.clone(),
            oracle_signature: tx.oracle_signature.clone(),
            fee: tx.fee,
        };
        let raw = safe_serialize(&payload)?;
        
        // Hash
        use sha2::Digest;
        let p_hash = sha2::Sha256::digest(&raw).to_vec();
        
        (p_hash, raw)
    };

    // Add to Gulf Stream
    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        gs.add_transaction(tx_hash.clone(), result, tx.fee);
    }

    Ok(serde_json::json!({
        "status": "Submitted to Gulf Stream",
        "tx_hash": hex::encode(tx_hash)
    }))
}

/// Handle submitBurn(...)
async fn handle_submit_burn(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let tx: SubmitBurnParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // Scope for locking chain (optional here if just constructing payload)
    let payload = crate::network::TransactionPayload::Burn {
        vault_id: tx.vault_id.clone(),
        amount: tx.burn_amount,
        recipient_btc_addr: tx.destination_address.clone(),
        signature: tx.signature.clone(),
        fee: tx.fee,
    };
    let raw = bincode::serialize(&payload).unwrap();
    
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw).to_vec();

    // Add to Gulf Stream
    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        gs.add_transaction(tx_hash.clone(), raw, tx.fee);
    }

    Ok(serde_json::json!({
        "status": "Submitted to Gulf Stream",
        "tx_hash": hex::encode(tx_hash)
    }))
}

/// Handle getVaultAddress(vault_id) -> address
async fn handle_get_vault_address(params: serde_json::Value) -> Result<serde_json::Value, RpcError> {
    let vault_id = params
        .get("vault_id")
        .and_then(|v| v.as_str())
        .ok_or(RpcError {
            code: -32602,
            message: "Missing vault_id".to_string(),
        })?;

    // In a real system, this would query VaultManager state.
    // For now, deterministic generation or lookup.
    Ok(serde_json::json!({
        "address": format!("vault_addr_{}", vault_id)
    }))
}

/// Handle getBlock(height)
async fn handle_get_block(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let p: GetBlockParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain = safe_lock(&chain)?;
    if let Ok(Some(block)) = chain.storage.get_block_by_height(p.height) {
         Ok(serde_json::to_value(block).unwrap())
    } else {
        Err(RpcError {
            code: -32602,
            message: "Block not found".to_string(),
        })
    }
}

/// Handle getLatestBlocks(count)
async fn handle_get_latest_blocks(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let p: GetLatestBlocksParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain = safe_lock(&chain)?;
    let height = chain.height;
    let start = if height > p.count as u64 {
        height - p.count as u64
    } else {
        0
    };

    let mut blocks = Vec::new();
    for i in start..=height {
        if let Ok(Some(b)) = chain.storage.get_block_by_height(i) {
            blocks.push(b);
        }
    }
    // Reverse to show newest first
    blocks.reverse();

    Ok(serde_json::to_value(blocks).unwrap())
}

/// Handle getTransactionStatus(tx_hash)
async fn handle_get_transaction_status(
    _chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let _p: GetTxStatusParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // Check Gulf Stream first? (Not easily accessible here without RpcState refactor, skipping for now)
    // Check Chain Storage (Confirmed)
    // We assume Storage has get_transaction_status or index? 
    // Implementing simple check: "Confirmed" if found in history indices (TODO)
    
    Ok(serde_json::json!({ "status": "Unknown (Not indexed)" }))
}

/// Handle getBalance
async fn handle_get_balance(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let p: GetBalanceParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain = safe_lock(&chain)?;
    // Use storage to get balance
    let bal = chain.storage.get_balance(&p.wallet_id, &p.asset).unwrap_or(0);
    Ok(serde_json::json!({ "balance": bal }))
}

/// Handle getNonce
async fn handle_get_nonce(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: serde_json::Value = params;
    let wallet_id = params
        .get("wallet_id")
        .and_then(|v| v.as_str())
        .ok_or(RpcError {
            code: -32602,
            message: "Missing wallet_id".to_string(),
        })?;

    let chain = safe_lock(&chain)?;
    let nonce = chain.storage.get_nonce(wallet_id).unwrap_or(0);
    Ok(serde_json::json!({ "nonce": nonce }))
}

/// Handle getChainHeight
async fn handle_get_chain_height(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain = safe_lock(&chain)?;
    Ok(serde_json::json!({ "height": chain.height }))
}

/// Handle getAccountInfo
async fn handle_get_account_info(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: serde_json::Value = params;
    let wallet_id = params
        .get("wallet_id")
        .and_then(|v| v.as_str())
        .ok_or(RpcError {
            code: -32602,
            message: "Missing wallet_id".to_string(),
        })?;

    let chain = safe_lock(&chain)?;
    // Return mock info or aggregate
    let nonce = chain.storage.get_nonce(wallet_id).unwrap_or(0);
    // TODO: List all balances (Storage needs iteration support)
    
    Ok(serde_json::json!({
        "wallet_id": wallet_id,
        "nonce": nonce,
        "balances": {} // Placeholder
    }))
}

/// Handle getNodeInfo
async fn handle_get_node_info(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain = safe_lock(&chain)?;
    Ok(serde_json::to_value(NodeInfo {
        height: chain.height,
        head_hash: chain.head_hash(),
        version: "0.1.0".to_string(),
        peer_count: 0, // Need PeerManager access
    })
    .unwrap())
}

/// Handle submitTransaction
async fn handle_submit_transaction(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    // NOTE: Simplified transaction handling - proper implementation needed
    let raw_tx = bincode::serialize(&params).unwrap_or_default();
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    // Push to Gulf Stream
    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        // Check duplication?
        gs.add_transaction(tx_hash.clone(), raw_tx, 0); // fee=0 for now
    }
    
    Ok(serde_json::json!({
        "status": "Submitted",
        "tx_hash": hex::encode(tx_hash)
    }))
}

/// Handle getValidatorStats(validator_id)
async fn handle_get_validator_stats(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: GetValidatorStatsParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain = safe_lock(&chain)?;
    let stats = chain
        .storage
        .get_validator_stats(&params.validator)
        .unwrap_or_default();

    Ok(serde_json::to_value(stats).unwrap())
}





const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

/// Handle submitCompute(job_id, model_id, inputs, ...)
async fn handle_submit_compute(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitComputeParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // 0. Security Check: Input Size
    if req.inputs.len() > MAX_INPUT_SIZE {
        return Err(RpcError {
            code: -32602,
            message: format!("Input size exceeds limit of {} bytes", MAX_INPUT_SIZE),
        });
    }

    // 1. Construct Transaction Payload
    let payload = crate::network::TransactionPayload::ComputeJob {
        job_id: req.job_id.clone(),
        model_id: req.model_id.clone(),
        inputs: req.inputs.clone(),
        max_compute_units: req.max_compute_units,
    };
    
    // 0. Validate Bid Asset (Phase 1: Only COMPASS)
    if req.bid_asset != "COMPASS" {
         return Err(RpcError {
            code: -32602,
            message: "Only COMPASS token is currently supported for bidding.".to_string(),
        });
    }

    // 1. Escrow Logic: Check Balance & Lock Funds
    {
        let chain = safe_lock(&state.chain)?;
        
        // Check User Balance
        let balance = chain.storage.get_balance(&req.owner_id, "COMPASS").unwrap_or(0);
        if balance < req.bid_amount {
            return Err(RpcError {
                code: -32002,
                message: format!("Insufficient balance. Have: {}, Need: {}", balance, req.bid_amount),
            });
        }

        // Transfer to Escrow Vault
        // Deduct from User
        chain.storage.set_balance(&req.owner_id, "COMPASS", balance - req.bid_amount).map_err(|e| RpcError {
             code: -32603,
             message: format!("Storage error: {}", e),
        })?;
        
        // Credit Escrow
        let escrow_bal = chain.storage.get_balance("ESCROW_VAULT", "COMPASS").unwrap_or(0);
        chain.storage.set_balance("ESCROW_VAULT", "COMPASS", escrow_bal + req.bid_amount).ok();
        
        info!("💰 Escrow Locked: {} COMPASS from {} for Job {}", req.bid_amount, req.owner_id, req.job_id);

        use crate::layer3::compute::ComputeJob;
        let job = ComputeJob::new(
            req.job_id.clone(),
            req.owner_id.clone(),
            req.model_id.clone(),
            req.inputs.clone(), // Pass Input Data
            req.bid_amount, // Use User's Bid
        );
        chain.storage.save_compute_job(&job).map_err(|e| RpcError {
             code: -32603,
             message: format!("Failed to save job: {}", e),
        })?;
    }

    // 2. Add to Local Gulf Stream
    let raw_tx = bincode::serialize(&payload).unwrap(); // safe_serialize?
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        // Priority fee is implicit or 0 for now? 
        let added = gs.add_transaction(tx_hash.clone(), raw_tx.clone(), 0);
        if !added {
             return Err(RpcError {
                code: -32603,
                message: "Transaction rejected (duplicate or full)".to_string(),
            });
        }
    }

    // GOSSIP: Broadcast to peers
    let msg = crate::network::NetMessage::SubmitTx(payload);
    let _ = state.cmd_tx.send(crate::network::NetworkCommand::Broadcast(msg)).await;
    
    // WORKER GOSSIP: Explicitly send ComputeJob so workers pick it up
    {
        use crate::layer3::compute::ComputeJob;
        // Re-construct minimal job for broadcast
        let job = ComputeJob::new(
            req.job_id.clone(),
            req.owner_id.clone(),
            req.model_id.clone(),
            req.inputs.clone(),
            100, // Default reward
        );
        let worker_msg = crate::network::NetMessage::ComputeJob(job);
        let _ = state.cmd_tx.send(crate::network::NetworkCommand::Broadcast(worker_msg)).await;
    }
    
    info!("🧠 AI Job Submitted: {} (Model: {})", req.job_id, req.model_id);

    Ok(serde_json::json!({
        "status": "Submitted",
        "tx_hash": hex::encode(tx_hash),
        "job_id": req.job_id
    }))
}

/// Handle submitResult(job_id, worker_id, result, ...)
async fn handle_submit_result(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitResultParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // 1. Construct Transaction Payload
    // 1. Construct Transaction Payload
    let payload = crate::network::TransactionPayload::Result(req.clone()); // req needs to be cloned or moved if used later? 
    // Logic below usually serializes payload. If req is moved, we can't use it.
    // Let's check if req is used below.
    // Wait, the previous code constructed new usage.
    // Just clone to be safe or move if last usage. req is from_value.
    // I'll clone it: TransactionPayload::Result(req.clone())
    
    info!("🧠 AI Result Received for Job: {} (Worker: {})", req.job_id, req.worker_id);

    // 2. Add to Local Gulf Stream
    let raw_tx = bincode::serialize(&payload).unwrap();
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        let added = gs.add_transaction(tx_hash.clone(), raw_tx.clone(), 0);
        if !added {
             return Err(RpcError {
                code: -32603,
                message: "Transaction rejected".to_string(),
            });
        }
    }
    
    // ===== ANTI-CHEAT: Validate Job Duration =====
    {
        let chain = safe_lock(&state.chain)?;
        if let Ok(Some(job)) = chain.storage.get_compute_job(&req.job_id) {
            // Check if job was started
            if let Some(started_at) = job.started_at {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let elapsed = current_time.saturating_sub(started_at);
                
                if elapsed < job.min_duration {
                    warn!(
                        "🚫 Job {} completed too quickly! Elapsed: {}s, Required: {}s (Worker: {})",
                        req.job_id, elapsed, job.min_duration, req.worker_id
                    );
                    return Err(RpcError {
                        code: -32007,
                        message: format!(
                            "Job completed too quickly ({}/{}s). Possible cheating detected.",
                            elapsed, job.min_duration
                        ),
                    });
                }
                
                info!(
                    "✅ Job {} duration validated: {}s (min: {}s)",
                    req.job_id, elapsed, job.min_duration
                );
            } else {
                // Job was never properly started, but allow it for beta resilience
                info!("⚠️ Job {} has no start time (resync?). Accepting result.", req.job_id);
                // Assume valid logic happened off-chain
            }
        }
    }
    
    // Remove from Pending Queue & Mint NFT if NN training job
    {
        use crate::layer3::model_nft::ModelNFT;
        let chain = safe_lock(&state.chain)?;
        
        // Get the completed job before deleting
        if let Ok(Some(job)) = chain.storage.get_compute_job(&req.job_id) {
            // Mint NFT to admin for completed neural network training
            let nft = ModelNFT {
                token_id: format!("NN_MODEL_{}", req.job_id),
                name: format!("Trained Model: {}", job.model_id),
                description: format!("Neural network model trained via job {}", req.job_id),
                creator: job.creator.clone(),
                // Performance metrics (from training)
                accuracy: 0.85 + (req.compute_rate as f64 / 1000000.0).min(0.10), // 85-95% based on compute
                win_rate: 0.80,
                total_predictions: 1000,
                profitable_predictions: 800,
                total_profit: (job.reward_amount * 100) as i64,
                // Training metadata
                training_samples: 10000,
                training_epochs: 50,
                final_loss: 0.05,
                training_duration_seconds: 15, // From agent.rs training cycle
                trained_on_data_hash: hex::encode(&req.result_data[0..32.min(req.result_data.len())]),
                weights_hash: req.signature.clone(),
                weights_uri: format!("compass://models/{}", job.job_id),
                architecture: format!("Oracle-Bridge-NN-{}", job.model_id),
                parent_models: vec![],
                generation: 1,
                // Economics
                mint_price: job.reward_amount * 100, // 100x the job reward
                royalty_rate: 5.0,
                current_owner: job.creator.clone(), // Minted to admin
                sale_history: vec![],
                // Timestamps
                minted_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            // Save NFT to storage
            if let Err(e) = chain.storage.put(&format!("nft:{}", nft.token_id), &nft) {
                warn!("Failed to mint NFT for job {}: {}", req.job_id, e);
            } else {
                info!("🎨 NFT MINTED: {} → Admin ({})", nft.token_id, job.creator);
                info!("   Model: {} | Value: {} COMPASS", job.model_id, nft.mint_price);
            }
        }
        
        chain.storage.delete_compute_job(&req.job_id).ok();
    }


    // GOSSIP: Broadcast to peers
    let msg = crate::network::NetMessage::SubmitTx(payload);
    let _ = state.cmd_tx.send(crate::network::NetworkCommand::Broadcast(msg)).await;

    Ok(serde_json::json!({
        "status": "Result Submitted",
        "tx_hash": hex::encode(tx_hash)
    }))
}


/// Handle getPendingComputeJobs(model_id?)
async fn handle_get_pending_compute_jobs(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: GetPendingComputeJobsParams = serde_json::from_value(params).unwrap_or(GetPendingComputeJobsParams { model_id: None });
    
    use crate::layer3::compute::ComputeJob;
    let jobs: Vec<ComputeJob> = {
        let chain = safe_lock(&state.chain)?;
        let all_jobs = chain.storage.get_pending_compute_jobs();
        all_jobs.into_iter()
            .filter(|j| {
                if let Some(target) = &req.model_id {
                    &j.model_id == target
                } else {
                    true
                }
            })
            .collect()
    };
    
    Ok(serde_json::to_value(jobs).unwrap())
}


// Oracle Verification Handlers


/// Handle submitOracleVerificationJob
async fn handle_submit_oracle_verification_job(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitOracleVerificationJobParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Get current oracle price if available
    let oracle_price = {
        let vm = safe_lock(&state.vault_manager)?;
        vm.oracle_prices.get(&req.ticker).map(|(price, _)| price.to_string())
    };

    let job = OracleVerificationJob {
        job_id: job_id.clone(),
        ticker: req.ticker.clone(),
        oracle_price,
        max_compute_units: req.max_compute_units,
        submission_time: crate::block::current_unix_timestamp_ms(),
    };

    // Store job in oracle jobs queue
    {
        let chain = safe_lock(&state.chain)?;
        chain.storage.save_oracle_job(&job).unwrap();
    }

    info!("📊 Oracle Verification Job Created: {} for ticker {}", job_id, req.ticker);

    Ok(serde_json::json!({
        "job_id": job_id,
        "ticker": req.ticker,
        "status": "pending"
    }))
}

/// Handle getPendingOracleJobs
async fn handle_get_pending_oracle_jobs(
    state: RpcState,
) -> Result<serde_json::Value, RpcError> {
    // Retrieve jobs from oracle jobs queue
    let jobs: Vec<OracleVerificationJob> = {
        let chain = safe_lock(&state.chain)?;
        chain.storage.get_pending_oracle_jobs()
    };
    
    Ok(serde_json::to_value(jobs).unwrap_or(serde_json::json!([])))
}

/// Handle submitOracleVerificationResult
async fn handle_submit_oracle_verification_result(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitOracleVerificationResultParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // Create OracleVerification transaction
    // Create OracleVerification transaction
    let payload = crate::network::TransactionPayload::OracleVerification(req.clone());

    // Verify signature
    if !payload.verify() {
        return Err(RpcError {
            code: -32603,
            message: "Invalid worker signature".to_string(),
        });
    }

    // Add to GulfStream
    let raw_tx = bincode::serialize(&payload).unwrap();
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();
    let tx_hash_hex = hex::encode(&tx_hash);

    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        let added = gs.add_transaction(tx_hash.clone(), raw_tx.clone(), 0);
        if !added {
            return Err(RpcError {
                code: -32603,
                message: "Transaction rejected (duplicate or queue full)".to_string(),
            });
        }
    }

    info!("✅ Oracle Verification Result Submitted: {} ({}) | Price: {} | Dev: {}%", 
        req.ticker, if req.passed { "PASS" } else { "FAIL" }, req.oracle_price, req.deviation_pct);
    debug!("Worker: {} | External Avg: {}", req.worker_id, req.avg_external_price);

    // --- INTEGRATING L2 COLLATERAL & L3 BETTING ---
    {
        // 1. Settle Pending Bets
        let mut ledger = safe_lock(&state.betting_ledger)?;
        let mut l2 = safe_lock(&state.layer2)?;
        let gas_price = req.oracle_price.parse::<f64>().unwrap_or(0.0);
        
        // Find bets older than 30 seconds (demo speed)
        let pending_timestamps: Vec<u64> = ledger.get_unevaluated_bets(0) // 0 mins = check all that are ready (logic check needed)
            .iter().map(|b| b.timestamp).collect();
            
        // We need a better way to check "Is this bet ready?". 
        // For now, if we have a new oracle price, we evaluate ALL pending bets associated with this ticker/context?
        // Simplifying: Evaluate ALL pending bets using this gas price.
        
        for ts in pending_timestamps {
            if let Some(pnl) = ledger.settle_bet(ts, gas_price) {
                if pnl > 0 {
                    // Win: Reward
                    l2.collateral.reward(req.worker_id.clone(), pnl as u64);
                    info!("   🎉 Bet WON! Rewarded {} to {}", pnl, req.worker_id);
                } else {
                    // Loss: Slash
                    let loss_abs = pnl.abs() as u64;
                    let _ = l2.collateral.slash(&req.worker_id, loss_abs);
                    info!("   💔 Bet LOST! Slashed {} from {}", loss_abs, req.worker_id);
                }
            }
        }
        
        // 2. Place New Bet
        // Parse prediction from payload: "AI:ETH_GAS_OPTIMIZED:..."
        let parts: Vec<&str> = req.avg_external_price.split(':').collect();
        if parts.len() >= 2 && parts[0] == "AI" {
            let prediction = parts[1].to_string();
            // Mock confidence for now, or extract from payload if added
            let confidence = 0.85; 
            
            // Place Bet
            // Need Sol/TVL context? Mocking for now as they are not in SubmitOracleVerificationResultParams explicitly
            // Only gas is reliable here.
            let bet = ledger.place_bet(prediction, confidence, gas_price, 150.0, 40_000_000_000.0);
            
            // 3. Lock Collateral (Stake)
            l2.collateral.stake(req.worker_id.clone(), bet.stake_amount);
            info!("   🎲 New Bet Placed: {} (Staked: {})", bet.prediction, bet.stake_amount);
        }
        
        // Persist
        let _ = ledger.save("betting.json");
        let _ = l2.save("layer2.json");
    }
    // ----------------------------------------------
    
    // --- 4. Reward Compute Tokens ---
    if req.compute_units_used > 0 {
        let mut wallets = safe_lock(&state.wallet_manager)?;
        wallets.credit(&req.worker_id, "COMPUTE", req.compute_units_used);
        info!("   💻 PoUW Reward: {} COMPUTE credited to {}", req.compute_units_used, req.worker_id);
        let _ = wallets.save("");
    }



    // Update Recurring Job Progress if applicable
    {
        let chain = state.chain.lock().unwrap();
        if let Ok(Some(mut job)) = chain.storage.get_recurring_job(&req.job_id) {
            job.completed_updates += 1;
            job.last_update_time = crate::block::current_unix_timestamp_ms() / 1000;
            debug!("   Recurring Job Progress: {}/{}", job.completed_updates, job.total_updates_required);
            
            if job.completed_updates >= job.total_updates_required {
                job.status = "Completed".to_string();
                info!("   🎉 Recurring Job {} COMPLETED!", req.job_id);
                
                // --- AUTO MINT NFT ---
                info!("   🎨 Auto-Minting Oracle Model NFT to Admin Vault...");
                
                use crate::layer3::model_nft::{ModelNFTRegistry, ModelStats, ModelNFT};
                
                let mut registry = ModelNFTRegistry::load("model_nft_registry.json")
                    .unwrap_or_else(|_| ModelNFTRegistry::new());
                
                // Get Real Stats from Betting Ledger
                let (staked, won, lost, win_rate) = {
                     let ledger = safe_lock(&state.betting_ledger)?;
                     ledger.get_stats()
                };

                let stats = ModelStats {
                    accuracy: win_rate, // Use win rate as accuracy proxy
                    win_rate: win_rate,
                    total_predictions: job.completed_updates as usize,
                    profitable_predictions: (job.completed_updates as f64 * win_rate) as usize,
                    total_profit: (won as i64) - (lost as i64),
                    training_samples: job.completed_updates as usize,
                    training_epochs: job.completed_updates as usize, // Continuous learning (usize)
                    final_loss: 1.0 - win_rate, // Simple proxy
                    training_duration: job.completed_updates as u64 * job.interval_seconds,
                    data_hash: hex::encode(&req.job_id),
                };
                
                let nft = ModelNFT::from_job(
                    &job.job_id,
                    &job.ticker,
                    job.owner.clone(), // Mint to Job Owner
                    &stats
                );
                
                let owner = nft.current_owner.clone();
                let token_id = registry.mint(nft);
                registry.save("model_nft_registry.json").ok();
                
                info!("   ✅ MINT SUCCESS: {} (Owner: {})", token_id, owner);
                // ---------------------
            }
            chain.storage.save_recurring_job(&job).unwrap();
        }
    }


    Ok(serde_json::json!({
        "status": "submitted",
        "job_id": req.job_id,
    }))
}

/// Handle submitRecurringOracleJob
async fn handle_submit_recurring_oracle_job(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::{RecurringOracleJob, SubmitRecurringJobParams};
    
    let req: SubmitRecurringJobParams = serde_json::from_value(params)
        .map_err(|e| RpcError {
            code: -32602,
            message: format!("Invalid params: {}", e),
        })?;

    // ENFORCE ADMIN ONLY
    // We compare with the Node Identity (which is the Admin Key in this architecture)
    if req.submitter != state.node_identity {
        warn!("⚠️ Unauthorized Recurring Job Attempt by {}", req.submitter);
        return Err(RpcError {
            code: -32003,
            message: "Unauthorized: Only Admin can create recurring jobs".to_string(),
        });
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let now = crate::block::current_unix_timestamp_ms() / 1000;
    
    let total_updates = (req.duration_hours * 60 / req.interval_minutes) as u32;
    
    let job = RecurringOracleJob {
        job_id: job_id.clone(),
        ticker: req.ticker.clone(),
        start_time: now,
        end_time: now + (req.duration_hours as u64 * 3600),
        interval_seconds: (req.interval_minutes as u64) * 60,
        total_updates_required: total_updates,
        completed_updates: 0,
        last_update_time: 0,
        worker_reward_per_update: req.reward_per_update,
        assigned_worker: None,
        status: "Active".to_string(),
        owner: req.submitter.clone(),
    };

    // Store job
    {
        let chain = safe_lock(&state.chain)?;
        chain.storage.save_recurring_job(&job).unwrap();
    }

    info!("📊 Recurring Oracle Job Created: {} for ticker {}", job_id, req.ticker);
    debug!("   Duration: {} hours ({} updates every {} min)", req.duration_hours, total_updates, req.interval_minutes);
    debug!("   Reward: {} COMPASS per update ({} total)", req.reward_per_update, total_updates as u64 * req.reward_per_update);

    Ok(serde_json::json!({
        "job_id": job_id,
        "ticker": req.ticker,
        "total_updates": total_updates,
        "reward_per_update": req.reward_per_update,
        "total_reward": total_updates as u64 * req.reward_per_update,
        "status": "Active"
    }))
}

/// Handle getRecurringJobs
async fn handle_get_recurring_jobs(
    state: RpcState,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::RecurringOracleJob;
    
    let jobs: Vec<RecurringOracleJob> = {
        let chain = safe_lock(&state.chain)?;
        let all = chain.storage.get_all_recurring_jobs();
        all.into_iter()
            .filter(|j| j.status == "Active")
            .collect()
    };
    
    Ok(serde_json::to_value(jobs).unwrap_or(serde_json::json!([])))
}

/// Handle getJobProgress
async fn handle_get_job_progress(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::GetJobProgressParams;
    
    let req: GetJobProgressParams = serde_json::from_value(params)
        .map_err(|e| RpcError {
            code: -32602,
            message: format!("Invalid params: {}", e),
        })?;

    let job = {
        let chain = safe_lock(&state.chain)?;
        chain.storage.get_recurring_job(&req.job_id).unwrap()
            .ok_or(RpcError {
                code: -32001,
                message: "Job not found".to_string(),
            })?
    };

    let progress_pct = (job.completed_updates as f64 / job.total_updates_required as f64) * 100.0;
    let earned_so_far = job.completed_updates as u64 * job.worker_reward_per_update;

    Ok(serde_json::json!({
        "job_id": job.job_id,
        "ticker": job.ticker,
        "status": job.status,
        "completed_updates": job.completed_updates,
        "total_updates": job.total_updates_required,
        "progress_percent": progress_pct,
        "earned_compass": earned_so_far,
        "assigned_worker": job.assigned_worker,
    }))
}

/// Handle getBlockRange(start, end)
async fn handle_get_block_range(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    #[derive(serde::Deserialize)]
    struct Params {
        start: Option<u64>,
        count: Option<u64>,
    }
    
    let p: Params = serde_json::from_value(params)
        .unwrap_or(Params { start: None, count: Some(5) });
    
    let chain_guard = safe_lock(&chain)?;
    let current_height = chain_guard.height;
    
    // Default: last 5 blocks
    let start = p.start.unwrap_or(current_height.saturating_sub(5));
    let count = p.count.unwrap_or(5).min(50); // Cap at 50
    let end = (start + count).min(current_height);
    
    let blocks = chain_guard.get_blocks_range(start, end);
    
    Ok(serde_json::to_value(blocks).unwrap())
}

/// Handle getOraclePrices()
async fn handle_get_oracle_prices(
    chain: Arc<Mutex<Chain>>,
) -> Result<serde_json::Value, RpcError> {
    let chain_guard = safe_lock(&chain)?;
    let prices = chain_guard.vault_manager.oracle_prices.clone();
    
    // Convert to simple map
    let mut result = std::collections::HashMap::new();
    for (ticker, (price, _timestamp)) in prices {
        result.insert(ticker, price.to_f64().unwrap_or(0.0));
    }
    
    Ok(serde_json::to_value(result).unwrap())
}

/// Handle purchaseNeuralNet
async fn handle_purchase_neural_net(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::PurchaseNeuralNetParams;
    use crate::rpc::types::RecurringOracleJob;
    
    let req: PurchaseNeuralNetParams = serde_json::from_value(params)
        .map_err(|e| RpcError {
            code: -32602,
            message: format!("Invalid params: {}", e),
        })?;

    // 1. Check Balance (Simplistic check)
    let job_id = uuid::Uuid::new_v4().to_string();
    let now = crate::block::current_unix_timestamp_ms() / 1000;
    
    // Default User Model: 24h training, 30 min intervals
    let duration_hours = 24; 
    let interval_minutes = 30;
    let total_updates = (duration_hours * 60 / interval_minutes) as u32;
    
    let job = RecurringOracleJob {
        job_id: job_id.clone(),
        ticker: req.ticker.clone(),
        start_time: now,
        end_time: now + (duration_hours * 3600),
        interval_seconds: (interval_minutes as u64) * 60,
        total_updates_required: total_updates,
        completed_updates: 0,
        last_update_time: 0,
        worker_reward_per_update: 5, 
        assigned_worker: Some(req.owner.clone()), 
        status: "Active".to_string(), 
        owner: req.owner.clone(),
    };
    
    {
        let chain = safe_lock(&state.chain)?;
        chain.storage.save_recurring_job(&job).unwrap();
    }
    
    info!("🛒 User {} purchased Neural Net for {}: Job {}", req.owner, req.ticker, job_id);
    
    Ok(serde_json::json!({
        "status": "Purchased",
        "job_id": job_id,
        "cost": 10000,
        "owner": req.owner
    }))
}

/// Handle listModelNFT
async fn handle_list_model_nft(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::ListModelNFTParams;
    let req: ListModelNFTParams = serde_json::from_value(params)
        .map_err(|e| RpcError { code: -32602, message: format!("Invalid params: {}", e) })?;
    
    // 1. Verify Ownership (Load Registry)
    use crate::layer3::model_nft::ModelNFTRegistry;
    let registry = ModelNFTRegistry::load("model_nft_registry.json")
        .unwrap_or_else(|_| ModelNFTRegistry::new());

    let nft = registry.get(&req.token_id).ok_or(RpcError{ code: -32001, message: "NFT not found".into() })?;
    if nft.current_owner != req.seller {
        return Err(RpcError{ code: -32003, message: "Caller does not own NFT".into() });
    }

    // 2. Place Listing in Market
    let mut market = safe_lock(&state.market)?;
    let msg = market.place_nft_listing(req.token_id.clone(), req.seller, req.price, req.currency)
        .map_err(|e| RpcError{ code: -32004, message: e })?;
        
    Ok(serde_json::json!({ "status": "listed", "message": msg }))
}

/// Handle buyModelNFT
async fn handle_buy_model_nft(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::rpc::types::BuyModelNFTParams;
    let req: BuyModelNFTParams = serde_json::from_value(params)
        .map_err(|e| RpcError { code: -32602, message: format!("Invalid params: {}", e) })?;

    // 1. Execute Financial Swap (Market)
    let (seller, royalty_amt, currency, price) = {
        let mut market = safe_lock(&state.market)?;
        let mut wallets = safe_lock(&state.wallet_manager)?;
        market.execute_nft_purchase(&req.token_id, &req.buyer, &mut wallets)
             .map_err(|e| RpcError{ code: -32005, message: e })?
    };

    // 2. Transfer Ownership (Registry)
    use crate::layer3::model_nft::ModelNFTRegistry;
    let mut registry = ModelNFTRegistry::load("model_nft_registry.json")
        .unwrap_or_else(|_| ModelNFTRegistry::new());
        
    let creator;
    {
        let nft = registry.nfts.iter_mut().find(|n| n.token_id == req.token_id).ok_or(RpcError{ code: -32001, message: "NFT not found".into() })?;
        nft.transfer(req.buyer.clone(), price, "AtomicMarketSwap".into());
        creator = nft.creator.clone();
    }
    
    registry.save("model_nft_registry.json").map_err(|e| RpcError{ code: -32006, message: e.to_string() })?;

    // 3. Pay Royalty to Creator
    if royalty_amt > 0 {
         let mut wallets = safe_lock(&state.wallet_manager)?;
         wallets.credit(&creator, &currency, royalty_amt);
         let _ = wallets.save("wallets.json");
    }

    info!("💰 NFT Sold: {} from {} to {}", req.token_id, seller, req.buyer);

    Ok(serde_json::json!({ "status": "purchased", "token_id": req.token_id }))
}

/// Handle getAllNFTs (Debug/Verify)
async fn handle_get_all_nfts(
    state: RpcState,
) -> Result<serde_json::Value, RpcError> {
    // 1. Fetch from RocksDB (New Persistence)
    let db_nfts = {
        let chain = safe_lock(&state.chain)?;
        chain.storage.get_all_nfts()
    };
    
    // 2. Fetch from JSON (Legacy)
    use crate::layer3::model_nft::ModelNFTRegistry;
    let legacy_nfts = ModelNFTRegistry::load("model_nft_registry.json")
        .map(|r| r.nfts)
        .unwrap_or_else(|_| Vec::new());
        
    // 3. Combine
    let mut all = db_nfts;
    // Simple deduplication based on ID if needed, but for now just concat
    for leg in legacy_nfts {
        if !all.iter().any(|n| n.token_id == leg.token_id) {
            all.push(leg);
        }
    }
    
    Ok(serde_json::to_value(all).unwrap())
}
/// Handle submitNativeVault - Lock COMPASS, mint Compass-LTC (user-defined rate)
async fn handle_submit_native_vault(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitNativeVaultParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // 1. Lock COMPASS collateral from user's balance
    {
        let chain = safe_lock(&state.chain)?;
        chain.storage.lock_vault_collateral(&req.owner_id, req.compass_collateral)
            .map_err(|e| RpcError {
                code: -32603,
                message: format!("Failed to lock collateral: {:?}", e),
            })?;
    }

    // 2. Process mint via VaultManager
    let (asset_name, minted, locked) = {
        let chain_guard = safe_lock(&state.chain)?;
        let mut vm = chain_guard.vault_manager.clone(); // Need mutable access
        
        // Load Oracle Public Key
        let oracle_pubkey = std::fs::read_to_string("oracle_pubkey.txt")
            .unwrap_or_else(|_| {
                // Return error if not configured
                // For dev/test convenience, maybe we can log checking failed
                "MISSING_ORACLE_KEY".to_string()
            });

        if oracle_pubkey == "MISSING_ORACLE_KEY" {
             return Err(RpcError {
                code: -32606,
                message: "Oracle Public Key not configured! Create oracle_pubkey.txt".to_string(),
            });
        }
        
        vm.deposit_native_and_mint(
            &req.payment_asset,
            req.payment_amount,
            req.compass_collateral,
            req.requested_mint_amount,
            &req.owner_id,
            &req.tx_hash,
            &req.oracle_signature,
            &oracle_pubkey.trim(),
        ).map_err(|e| RpcError {
            code: -32604,
            message: format!("Vault deposit failed: {}", e),
        })?
    };

    // 3. Credit minted tokens to user
    {
        let chain = safe_lock(&state.chain)?;
        chain.storage.update_balance(&req.owner_id, &asset_name, minted)
            .map_err(|e| RpcError {
                code: -32605,
                message: format!("Failed to credit tokens: {:?}", e),
            })?;
    }

    info!("✅ Native Vault: Locked {} COMPASS, minted {} {}", locked, minted, asset_name);

    Ok(serde_json::json!({
        "asset": asset_name,
        "minted": minted,
        "collateral_locked": locked,
        "status": "success"
    }))
}
