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
        "getPeers" => handle_get_peers(state.clone()).await,
        "getVaultAddress" => handle_get_vault_address(req.params).await,
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
                fee: tx.fee,
            },
            proposer: tx.owner.clone(),
            signature_hex: tx.signature.clone(),
            prev_hash,
            hash: String::new(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        header.hash = header.calculate_hash();
        let tx_hash = header.hash.clone();

        // Mine locally
        let res = chain_lock.append_mint(header, &tx.owner);
        (tx_hash, res)
    }; // Lock dropped here

    match result {
        Ok(_) => {
            // Broadcast
            let msg =
                crate::network::NetMessage::SubmitTx(crate::network::TransactionPayload::Mint(tx));
            crate::network::broadcast_message(state.peer_manager.clone(), msg).await;

            Ok(serde_json::json!({
                "status": "accepted_and_mined",
                "tx_hash": tx_hash
            }))
        }
        Err(e) => Err(RpcError {
            code: -32003,
            message: format!("Failed to append mint: {}", e),
        }),
    }
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

    let (tx_hash, result) = {
        let mut chain_lock = state.chain.lock().unwrap();
        let prev_hash = chain_lock.head_hash().unwrap_or_default();

        let mut header = BlockHeader {
            index: chain_lock.height,
            block_type: BlockType::Burn {
                vault_id: tx.vault_id.clone(),
                compass_asset: tx.compass_asset.clone(),
                burn_amount: tx.burn_amount,
                redeemer: tx.redeemer.clone(),
                destination_address: tx.destination_address.clone(),
                fee: tx.fee,
            },
            proposer: tx.redeemer.clone(),
            signature_hex: tx.signature.clone(),
            prev_hash,
            hash: String::new(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        header.hash = header.calculate_hash();
        let tx_hash = header.hash.clone();

        let res = chain_lock.append_burn(header, &tx.redeemer);
        (tx_hash, res)
    };

    match result {
        Ok(_) => {
            let msg =
                crate::network::NetMessage::SubmitTx(crate::network::TransactionPayload::Burn(tx));
            crate::network::broadcast_message(state.peer_manager.clone(), msg).await;

            Ok(serde_json::json!({
                "status": "accepted_and_mined",
                "tx_hash": tx_hash
            }))
        }
        Err(e) => Err(RpcError {
            code: -32003,
            message: format!("Failed to append burn: {}", e),
        }),
    }
}

/// Handle getBalance(wallet_id, asset)
async fn handle_get_balance(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: GetBalanceParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();
    let balance = chain_lock
        .storage
        .get_balance(&params.wallet_id, &params.asset)
        .unwrap_or(0);

    Ok(serde_json::json!({ "balance": balance }))
}

/// Handle getNonce(wallet_id)
async fn handle_get_nonce(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let wallet_id: String = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();
    let nonce = chain_lock.storage.get_nonce(&wallet_id).unwrap_or(0);

    Ok(serde_json::json!({ "nonce": nonce }))
}

/// Handle getChainHeight()
async fn handle_get_chain_height(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain_lock = chain.lock().unwrap();
    let height = chain_lock.height;
    Ok(serde_json::json!({ "height": height }))
}

/// Handle getAccountInfo(wallet_id)
async fn handle_get_account_info(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let wallet_id: String = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();

    // Collect balances for common assets
    let mut balances = std::collections::HashMap::new();
    
    // Check hardcoded assets
    for asset in &["Compass", "cLTC", "cSOL", "cBTC"] {
        if let Ok(balance) = chain_lock.storage.get_balance(&wallet_id, asset) {
            if balance > 0 {
                balances.insert(asset.to_string(), balance);
            }
        }
    }
    
    // Check for vault-based assets (Compass:Owner:Collateral format)
    // Iterate through vault manager to find assets owned by this wallet
    let mut vault_info = std::collections::HashMap::new();
    for (asset_name, vault) in &chain_lock.vault_manager.vaults {
        // Asset format: "Compass:Owner:Collateral"
        // Check if this asset belongs to the wallet_id
        if asset_name.contains(&format!("Compass:{}:", wallet_id)) {
            if let Ok(balance) = chain_lock.storage.get_balance(&wallet_id, asset_name) {
                if balance > 0 {
                    balances.insert(asset_name.clone(), balance);
                }
            }
            
            // Add vault backing info
            vault_info.insert(asset_name.clone(), serde_json::json!({
                "collateral_balance": vault.collateral_balance,
                "minted_supply": vault.minted_supply,
                "collateral_asset": vault.collateral_asset,
                "backing_ratio": if vault.minted_supply > 0 {
                    vault.minted_supply as f64 / vault.collateral_balance as f64
                } else {
                    0.0
                }
            }));
        }
    }

    let nonce = chain_lock.storage.get_nonce(&wallet_id).unwrap_or(0);

    Ok(serde_json::json!({
        "wallet_id": wallet_id,
        "balances": balances,
        "vault_info": vault_info,
        "nonce": nonce,
    }))
}

/// Get vault deposit address for a user/asset combination
async fn handle_get_vault_address(
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let owner = params
        .get("owner")
        .and_then(|v| v.as_str())
        .ok_or(RpcError {
            code: -32602,
            message: "Missing 'owner' parameter".to_string(),
        })?;

    let asset = params
        .get("asset")
        .and_then(|v| v.as_str())
        .ok_or(RpcError {
            code: -32602,
            message: "Missing 'asset' parameter".to_string(),
        })?;

    // Load vault key manager
    let vault_keys = crate::vault::VaultKeyManager::load_or_generate("vault_master.seed");
    let master_seed = vault_keys.get_seed();

    // Generate vault address
    let (vault_address, derivation_path) = crate::vault::VaultManager::generate_vault_address(
        owner,
        asset,
        master_seed,
    );

    let vault_id = format!("Compass:{}:{}", owner, asset);

    Ok(serde_json::json!({
        "vault_id": vault_id,
        "deposit_address": vault_address,
        "derivation_path": derivation_path,
        "asset": asset,
        "owner": owner,
        "instructions": {
            "step1": format!("Send {} to {}", asset, vault_address),
            "step2": "Wait for confirmations (BTC: 6+, LTC: 12+, SOL: 32+)",
            "step3": "Submit mint request with transaction hash",
            "step4": "Oracle will verify and mint Compass tokens"
        }
    }))
}

/// Handle submitTransaction(from, to, asset, amount, nonce, signature)
async fn handle_submit_transaction(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    #[derive(serde::Deserialize)]
    struct TxParams {
        from: String,
        to: String,
        asset: String,
        amount: u64,
        nonce: u64,
        signature: String,
        #[serde(default)]
        fee: u64,
    }

    let tx: TxParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let (tx_hash, result) = {
        let mut chain_lock = state.chain.lock().unwrap();

        // Basic validation
        let sender_nonce = chain_lock.storage.get_nonce(&tx.from).unwrap_or(0);

        // This validation logic is repeated in `append_transfer` usually, but we check nonce here early.
        // Actually, if we return early, we must not have result?
        // Let's defer nonce check to `append_transfer` if possible, or check here logic.
        // Assuming simpler logic:
        if tx.nonce != sender_nonce + 1 {
            // Return error immediately
            return Err(RpcError {
                code: -32000,
                message: format!(
                    "Invalid nonce: expected {}, got {}",
                    sender_nonce + 1,
                    tx.nonce
                ),
            });
        }

        // Build BlockHeader using BlockType::Transfer
        let prev_hash = chain_lock.head_hash().unwrap_or_default();

        let mut header = BlockHeader {
            index: chain_lock.height, // height is next index
            block_type: BlockType::Transfer {
                from: tx.from.clone(),
                to: tx.to.clone(),
                asset: tx.asset.clone(),
                amount: tx.amount,
                nonce: tx.nonce,
                fee: tx.fee,
            },
            proposer: tx.from.clone(),
            signature_hex: tx.signature.clone(),
            prev_hash,
            hash: String::new(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // Compute canonical hash
        header.hash = header.calculate_hash();
        let tx_hash = header.hash.clone();

        // 1. Process Locally (Mine)
        let res = chain_lock.append_transfer(header, &tx.from);
        (tx_hash, res)
    };

    match result {
        Ok(_) => {
            // 2. Broadcast
            let msg = crate::network::NetMessage::SubmitTx(
                crate::network::TransactionPayload::Transfer {
                    from: tx.from,
                    to: tx.to,
                    asset: tx.asset,
                    amount: tx.amount,
                    nonce: tx.nonce,
                    signature: tx.signature,
                },
            );
            crate::network::broadcast_message(state.peer_manager.clone(), msg).await;

            Ok(serde_json::json!({
                "status": "accepted_and_mined",
                "tx_hash": tx_hash
            }))
        }
        Err(e) => Err(RpcError {
            code: -32003,
            message: format!("Failed to process tx: {}", e),
        }),
    }
}

/// Handle getBlock(height)
async fn handle_get_block(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: GetBlockParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();
    if let Ok(Some(block)) = chain_lock.storage.get_block_by_height(params.height) {
        Ok(serde_json::json!(block.header))
    } else {
        Err(RpcError {
            code: -32601,
            message: "Block not found".to_string(),
        })
    }
}

/// Handle getLatestBlocks(count)
async fn handle_get_latest_blocks(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: GetLatestBlocksParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();
    let current_height = chain_lock.height;
    let start_height = if current_height > params.count as u64 {
        current_height - params.count as u64
    } else {
        0
    };

    let mut blocks = Vec::new();
    for h in (start_height..current_height).rev() {
        if let Ok(Some(block)) = chain_lock.storage.get_block_by_height(h) {
            blocks.push(block.header);
        }
    }

    Ok(serde_json::json!({ "blocks": blocks }))
}

/// Handle getTransactionStatus(tx_hash)
async fn handle_get_transaction_status(
    chain: Arc<Mutex<Chain>>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params: GetTxStatusParams = serde_json::from_value(params).map_err(|e| RpcError {
        code: -32602,
        message: format!("Invalid params: {}", e),
    })?;

    let chain_lock = chain.lock().unwrap();
    // In this model, tx_hash is block hash for transfers
    if let Ok(Some(_block)) = chain_lock.storage.get_block(&params.tx_hash) {
        Ok(serde_json::json!({ "status": "Confirmed" }))
    } else {
        Ok(serde_json::json!({ "status": "Unknown" }))
    }
}

/// Handle getNodeInfo()
async fn handle_get_node_info(chain: Arc<Mutex<Chain>>) -> Result<serde_json::Value, RpcError> {
    let chain_lock = chain.lock().unwrap();
    let info = NodeInfo {
        height: chain_lock.height,
        head_hash: chain_lock.head_hash(),
        version: "1.2.0".to_string(),
        peer_count: 0, // Placeholder
    };
    Ok(serde_json::json!(info))
}
