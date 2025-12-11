use super::types::*;
use crate::block::{BlockHeader, BlockType}; // Added BlockType
use crate::chain::Chain;
use crate::rpc::RpcState;
use axum::{debug_handler, extract::State, Json};
use std::sync::{Arc, Mutex}; // Import RpcState

/// Main dispatcher: routes incoming JSON-RPC requests to the correct handler.
#[debug_handler]
pub async fn handle_rpc_request(
    State(state): State<RpcState>,
    Json(req): Json<RpcRequest>,
) -> Json<RpcResponse> {
    println!("RPC Request: method={}, id={}", req.method, req.id);

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
        "getPeers" => handle_get_peers(state.clone()).await,
        "getVaultAddress" => handle_get_vault_address(req.params).await,
        "getValidatorStats" => handle_get_validator_stats(state.chain.clone(), req.params).await,
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
// === Individual Handlers ===
//

/// Handle getPeers()
async fn handle_get_peers(state: RpcState) -> Result<serde_json::Value, RpcError> {
    let pm = state.peer_manager.lock().unwrap();
    let peers: Vec<String> = pm.peers.iter().cloned().collect();

    Ok(serde_json::to_value(GetPeersResponse { peers }).unwrap())
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
        let mut chain_lock = state.chain.lock().unwrap();
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
        header.hash = header.calculate_hash();
        
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
        let payload = crate::network::TransactionPayload::Mint(tx.clone());
        let raw = bincode::serialize(&payload).unwrap();
        
        // Hash
        use sha2::Digest;
        let p_hash = sha2::Sha256::digest(&raw).to_vec();
        
        (p_hash, raw)
    };

    // Add to Gulf Stream
    {
        let mut gs = state.gulf_stream.lock().unwrap();
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
    let payload = crate::network::TransactionPayload::Burn(tx.clone());
    let raw = bincode::serialize(&payload).unwrap();
    
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw).to_vec();

    // Add to Gulf Stream
    {
        let mut gs = state.gulf_stream.lock().unwrap();
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

    let chain = chain.lock().unwrap();
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

    let chain = chain.lock().unwrap();
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

    let chain = chain.lock().unwrap();
    // Use storage to get balance
    let bal = chain.storage.get_balance(&p.wallet_id, &p.asset);
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

    let chain = chain.lock().unwrap();
    let nonce = chain.storage.get_nonce(wallet_id);
    Ok(serde_json::json!({ "nonce": nonce }))
}

/// Handle getChainHeight
async fn handle_get_chain_height(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain = chain.lock().unwrap();
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

    let chain = chain.lock().unwrap();
    // Return mock info or aggregate
    let nonce = chain.storage.get_nonce(wallet_id);
    // TODO: List all balances (Storage needs iteration support)
    
    Ok(serde_json::json!({
        "wallet_id": wallet_id,
        "nonce": nonce,
        "balances": {} // Placeholder
    }))
}

/// Handle getNodeInfo
async fn handle_get_node_info(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain = chain.lock().unwrap();
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
    let tx: SubmitTransferParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // 1. Verify Signature (TODO)
    // 2. Add to Gulf Stream
    // Construct TransactionPayload
    let payload = crate::network::TransactionPayload::Transfer {
        from: tx.from,
        to: tx.to,
        asset: tx.asset,
        amount: tx.amount,
        nonce: 0, // Should be in params!
        signature: tx.signature,
    };
    
    // Serialize
    let raw_tx = bincode::serialize(&payload).unwrap();
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    // Push to Gulf Stream
    {
        let mut gs = state.gulf_stream.lock().unwrap();
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

    let chain = chain.lock().unwrap();
    let stats = chain
        .storage
        .get_validator_stats(&params.validator)
        .unwrap_or_default();

    Ok(serde_json::to_value(stats).unwrap())
}

/// Handle submitCompute(job_id, model_id, inputs, ...)
async fn handle_submit_compute(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let req: SubmitComputeParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    // 1. Construct Transaction Payload
    let payload = crate::network::TransactionPayload::ComputeJob {
        job_id: req.job_id.clone(),
        model_id: req.model_id.clone(),
        inputs: req.inputs.clone(),
        max_compute_units: req.max_compute_units,
    };

    // 2. Add to Local Gulf Stream
    let raw_tx = bincode::serialize(&payload).unwrap();
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    {
        let mut gs = state.gulf_stream.lock().unwrap();
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
    crate::network::broadcast_message(state.peer_manager.clone(), msg).await;

    Ok(serde_json::json!({
        "status": "Submitted",
        "tx_hash": hex::encode(tx_hash)
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
    let payload = crate::network::TransactionPayload::Result {
        job_id: req.job_id,
        worker_id: req.worker_id,
        result_data: req.result_data,
        signature: req.signature,
    };

    // 2. Add to Local Gulf Stream
    let raw_tx = bincode::serialize(&payload).unwrap();
    use sha2::Digest;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();

    {
        let mut gs = state.gulf_stream.lock().unwrap();
        let added = gs.add_transaction(tx_hash.clone(), raw_tx.clone(), 0);
        if !added {
             return Err(RpcError {
                code: -32603,
                message: "Transaction rejected".to_string(),
            });
        }
    }

    // GOSSIP: Broadcast to peers
    let msg = crate::network::NetMessage::SubmitTx(payload);
    crate::network::broadcast_message(state.peer_manager.clone(), msg).await;

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
    
    let mut jobs: Vec<PendingJob> = Vec::new();

    {
        let gs = state.gulf_stream.lock().unwrap();
        // Since GulfStreamManager is private in structure, we need an accessor function or inspect public fields.
        // Assuming we can iterate:
        // Current impl of GulfStreamManager uses `queue: PriorityQueue`.
        // We might need to expose a method in GulfStream to "peek" or "list" transactions without popping.
        
        // HACK: Accessing private `pending_transactions`!
        // The previous error said `transactions` doesn't exist.
        // `pending_transactions` and `processing_transactions` exist.
        for (hash, tx_obj) in &gs.pending_transactions {
             match bincode::deserialize::<crate::network::TransactionPayload>(&tx_obj.raw_tx) {
                Ok(crate::network::TransactionPayload::ComputeJob { job_id, model_id, inputs, max_compute_units }) => {
                    // Filter
                    if let Some(target_model) = &req.model_id {
                        if &model_id != target_model {
                            continue;
                        }
                    }
                    
                    jobs.push(PendingJob {
                        job_id,
                        model_id,
                        inputs,
                        max_compute_units,
                        tx_hash: hex::encode(hash),
                        owner_id: "unknown".to_string(), // TODO: Recover from context or payload
                    });
                },
                _ => {} // Ignore non-compute txs
             }
        }
    }

    Ok(serde_json::to_value(jobs).unwrap())
}
