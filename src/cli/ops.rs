use crate::block::{BlockHeader, BlockType};
use crate::client::rpc_client::RpcClient;
use crate::crypto::KeyPair;
use crate::rpc::types::{SubmitBurnParams, SubmitMintParams};
use crate::wallet::WalletManager;
use chrono::Utc;

pub async fn handle_mint_command(
    vault_id: String,
    amount: u64,
    asset: String,
    collateral_asset: String,
    collateral_amount: u64,
    proof: String,
    oracle_sig: String,
    owner: String,
    rpc_url: Option<String>,
) {
    // 1. Setup RPC
    let url = rpc_url.unwrap_or_else(|| "http://localhost:8899".to_string());
    let client = RpcClient::new(url);

    // 2. Get Wallet for Signing (Header integrity)
    // We assume the local user "owner" is signing the request.
    let manager = WalletManager::load("client_wallets.json");
    let wallet = match manager.get_wallet(&owner) {
        Some(w) => w,
        None => {
            println!("Error: Wallet '{}' not found", owner);
            return;
        }
    };
    let mnemonic = match &wallet.mnemonic {
        Some(m) => m,
        None => {
            println!("Error: Wallet has no mnemonic");
            return;
        }
    };
    let keypair = match KeyPair::from_mnemonic(mnemonic) {
        Ok(kp) => kp,
        Err(e) => {
            println!("Error restoring keys: {}", e);
            return;
        }
    };

    // 3. Get Chain State
    let node_info = match client.get_node_info().await {
        Ok(info) => info,
        Err(e) => {
            println!("Error fetching node info: {}", e);
            return;
        }
    };
    let head_hash = node_info["head_hash"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let height = node_info["height"].as_u64().unwrap_or(0);

    // 4. Construct Header to Sign
    let mut header = BlockHeader {
        index: height,
        block_type: BlockType::Mint {
            vault_id: vault_id.clone(),
            collateral_asset: collateral_asset.clone(),
            collateral_amount,
            compass_asset: asset.clone(),
            mint_amount: amount,
            owner: owner.clone(),
            tx_proof: proof.clone(),
            oracle_signature: oracle_sig.clone(),
            fee: 0, // Default fee
        },
        proposer: owner.clone(),
        signature_hex: String::new(),
        prev_hash: head_hash,
        hash: String::new(),
        timestamp: Utc::now().timestamp() as u64,
    };

    header.hash = header.calculate_hash();
    let signature = keypair.sign_hex(header.hash.as_bytes());

    // 5. Submit
    let params = SubmitMintParams {
        vault_id,
        collateral_asset,
        collateral_amount,
        compass_asset: asset,
        mint_amount: amount,
        owner,
        tx_proof: proof,
        oracle_signature: oracle_sig,
        fee: 0,
        signature,
    };

    match client.submit_mint(params).await {
        Ok(tx_hash) => println!("Mint Submitted! Tx Hash: {}", tx_hash),
        Err(e) => println!("Mint Failed: {}", e),
    }
}

pub async fn handle_burn_command(
    vault_id: String,
    amount: u64,
    asset: String,
    dest_addr: String,
    from: String,
    rpc_url: Option<String>,
) {
    let url = rpc_url.unwrap_or_else(|| "http://localhost:8899".to_string());
    let client = RpcClient::new(url);

    let manager = WalletManager::load("client_wallets.json");
    let wallet = match manager.get_wallet(&from) {
        Some(w) => w,
        None => {
            println!("Error: Wallet '{}' not found", from);
            return;
        }
    };
    let mnemonic = match &wallet.mnemonic {
        Some(m) => m,
        None => {
            println!("Error: Wallet has no mnemonic");
            return;
        }
    };
    let keypair = match KeyPair::from_mnemonic(mnemonic) {
        Ok(kp) => kp,
        Err(e) => {
            println!("Error restoring keys: {}", e);
            return;
        }
    };

    let node_info = match client.get_node_info().await {
        Ok(info) => info,
        Err(e) => {
            println!("Error fetching node info: {}", e);
            return;
        }
    };
    let head_hash = node_info["head_hash"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let height = node_info["height"].as_u64().unwrap_or(0);

    let mut header = BlockHeader {
        index: height,
        block_type: BlockType::Burn {
            vault_id: vault_id.clone(),
            compass_asset: asset.clone(),
            burn_amount: amount,
            redeemer: from.clone(),
            destination_address: dest_addr.clone(),
            fee: 0,
        },
        proposer: from.clone(),
        signature_hex: String::new(),
        prev_hash: head_hash,
        hash: String::new(),
        timestamp: Utc::now().timestamp() as u64,
    };

    header.hash = header.calculate_hash();
    let signature = keypair.sign_hex(header.hash.as_bytes());

    let params = SubmitBurnParams {
        vault_id,
        compass_asset: asset,
        burn_amount: amount,
        redeemer: from,
        destination_address: dest_addr,
        fee: 0,
        signature,
    };

    match client.submit_burn(params).await {
        Ok(tx_hash) => println!("Burn Submitted! Tx Hash: {}", tx_hash),
        Err(e) => println!("Burn Failed: {}", e),
    }
}
