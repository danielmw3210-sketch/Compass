use crate::wallet::WalletManager;
use crate::client::rpc_client::RpcClient;
use crate::crypto::KeyPair;
use crate::block::{BlockHeader, BlockType};
use chrono::Utc;

pub async fn handle_transfer_command(from: String, to: String, amount: u64, asset: String, rpc_url: Option<String>) {
    // 1. Get Wallet / Keys
    let manager = WalletManager::load("client_wallets.json");
    let wallet = match manager.get_wallet(&from) {
        Some(w) => w,
        None => {
            println!("Error: Wallet '{}' not found in client_wallets.json", from);
            return;
        }
    };

    let mnemonic = match &wallet.mnemonic {
        Some(m) => m,
        None => {
            println!("Error: Wallet '{}' does not have a mnemonic (cannot sign)", from);
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

    // 2. Setup RPC
    let url = rpc_url.unwrap_or_else(|| "http://localhost:8899".to_string());
    let client = RpcClient::new(url);

    // 3. Get Nonce
    println!("Fetching nonce...");
    let nonce = match client.get_nonce(&wallet.public_key).await {
        Ok(n) => n + 1, // Next nonce
        Err(e) => {
            println!("Error fetching nonce: {}", e);
            return;
        }
    };

    // 4. Fetch Chain State (Head Hash)
    println!("Fetching chain state...");
    let node_info = match client.get_node_info().await {
        Ok(info) => info,
        Err(e) => {
            println!("Error fetching node info: {}", e);
            return;
        }
    };

    let head_hash = node_info["head_hash"].as_str().map(|s| s.to_string()).unwrap_or_default();
    let height = node_info["height"].as_u64().unwrap_or(0);

    // 5. Construct Block Header (Intent)
    // In this "Tx = Block" model, we construct the header to sign it.
    let mut header = BlockHeader {
        index: height, // Note: This might be slightly off if block produced since. But sig verifies content.
                       // Actually, checking chain.rs: verify uses header.index? No.
                       // It mainly verifies signature against hash.
        block_type: BlockType::Transfer {
            from: from.clone(),
            to: to.clone(),
            asset: asset.clone(),
            amount,
            nonce,
            fee: 0, // Default fee 0 for now in CLI
        },
        proposer: from.clone(),
        signature_hex: String::new(), // To be filled
        prev_hash: head_hash, // This creates the race condition, but it's what we have.
        hash: String::new(),
        timestamp: Utc::now().timestamp() as u64,
    };

    // Calculate Hash (Pre-signature)
    header.hash = header.calculate_hash();

    // 6. Sign Hash
    let signature = keypair.sign_hex(header.hash.as_bytes());

    // 7. Submit
    println!("Submitting transfer...");
    match client.submit_transaction(&from, &to, &asset, amount, nonce, &signature).await {
        Ok(tx_hash) => {
            println!("Success! Tx Hash: {}", tx_hash);
        },
        Err(e) => {
            println!("Transaction failed: {}", e);
        }
    }
}
