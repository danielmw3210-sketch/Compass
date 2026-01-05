use rust_compass::block::{
    create_transfer_block,
    current_unix_timestamp_ms, BlockHeader, BlockType,
}; 
use rust_compass::crypto::KeyPair;
use rust_compass::network::NetworkCommand;
// use libp2p::identity; // Conflict with mod identity; use explicit path if needed

use rust_compass::market::{Market, OrderSide};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::Arc;
use rust_compass::vault::VaultManager;
use rust_compass::wallet::{self, WalletManager, Wallet, WalletType};
use tracing::{info, warn, error};
use tracing_subscriber::FmtSubscriber;

use clap::Parser;
use rust_compass::cli::{self, Cli, Commands};
use rust_compass::config;

/// Simple file logger
fn log_to_file(msg: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("compass.log")
        .unwrap();
    writeln!(file, "{}", msg).unwrap();
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter("info") // Default to info, user can ensure RUST_LOG=debug
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();

    // Check if any specific command is provided
    if let Some(command) = cli.command {
        match command {
            Commands::Wallet { cmd } => {
                cli::wallet::handle_wallet_command(cmd);
            }
            Commands::Account { cmd } => {
                cli::wallet::handle_account_command(cmd);
            }
            Commands::Node { cmd } => {
                // If "compass node start" is called
                match cmd {
                    cli::node::NodeCommands::Start { rpc_port, peer, p2p_port: _, db_path: _, ephemeral } => {
                        // Load Config
                        let mut config = rust_compass::config::CompassConfig::load_or_default("config.toml");
                        
                        // CLI overrides Config (Priority: CLI > Config > Default)
                        if let Some(p) = rpc_port { config.node.rpc_port = p; }
                        // Identity Loading (Phase 3)
                        let identity_val = if ephemeral {
                            warn!("Starting in Ephemeral Mode: Generating temporary identity.");
                            None
                        } else {
                            let path_str = &config.node.identity_file;
                            let path = std::path::Path::new(path_str);
                            
                            // Check configured path OR admin.json fallback
                            let final_path = if path.exists() {
                                path
                            } else if std::path::Path::new("admin.json").exists() {
                                std::path::Path::new("admin.json")
                            } else {
                                path // Will trigger missing logic below
                            };

                            if final_path.exists() {
                                println!("Found Identity File: {:?}", final_path);
                                
                                // Try Empty Password First (for Automation/Testnet)
                                match rust_compass::identity::Identity::load_and_decrypt(final_path, "") {
                                    Ok(id) => {
                                        println!("Identity '{}' unlocked (Passwordless) ({})", id.name, id.public_key);
                                        Some(Arc::new(id.into_keypair().expect("Failed to convert identity")))
                                    },
                                    Err(_) => {
                                        // Fallback to Prompt
                                        if ephemeral { // If ephemeral flag set but we found a file, maybe ignore it? No, ephemeral overrides loading.
                                            // Actually this block is in the `else` of `if ephemeral`.
                                            // So we are here because ephemeral is false.
                                            print!("Enter password to unlock Node Identity: ");
                                            std::io::stdout().flush().unwrap();
                                            let mut pass = String::new();
                                            std::io::stdin().read_line(&mut pass).unwrap();
                                            
                                            match rust_compass::identity::Identity::load_and_decrypt(final_path, pass.trim()) {
                                                Ok(id) => {
                                                    println!("Identity '{}' unlocked ({})", id.name, id.public_key);
                                                    // Create valid backup
                                                    let _ = std::fs::copy("admin.json", "admin.backup.json");
                                                    Some(Arc::new(id.into_keypair().expect("Failed to convert identity")))
                                                },
                                                Err(e) => {
                                                    error!("Failed to unlock identity: {}", e);
                                                    if e.to_string().contains("missing field") {
                                                        println!("⚠️  DETECTED CORRUPTION: Attempting to restore from backup...");
                                                        match rust_compass::identity::Identity::load_and_decrypt(std::path::Path::new("admin.backup.json"), pass.trim()) {
                                                            Ok(id_bak) => {
                                                                println!("✅ BACKUP RESTORED: '{}'", id_bak.name);
                                                                let _ = std::fs::copy("admin.backup.json", "admin.json");
                                                                Some(Arc::new(id_bak.into_keypair().expect("Failed to convert identity")))
                                                            },
                                                            Err(e_bak) => {
                                                                println!("❌ BACKUP FAILED: {}", e_bak);
                                                                std::process::exit(1);
                                                            }
                                                        }
                                                    } else {
                                                        std::process::exit(1); 
                                                    }
                                                }
                                            }
                                        } else {
                                             print!("Enter password to unlock Node Identity: ");
                                             std::io::stdout().flush().unwrap();
                                             let mut pass = String::new();
                                             std::io::stdin().read_line(&mut pass).unwrap();
                                             
                                             match rust_compass::identity::Identity::load_and_decrypt(final_path, pass.trim()) {
                                                 Ok(id) => {
                                                     println!("Identity '{}' unlocked ({})", id.name, id.public_key);
                                                     // Create valid backup
                                                     let _ = std::fs::copy("admin.json", "admin.backup.json");
                                                     Some(Arc::new(id.into_keypair().expect("Failed to convert identity")))
                                                 },
                                                 Err(e) => {
                                                     error!("Failed to unlock identity: {}", e);
                                                     if e.to_string().contains("missing field") {
                                                        println!("⚠️  DETECTED CORRUPTION: Attempting to restore from backup...");
                                                        match rust_compass::identity::Identity::load_and_decrypt(std::path::Path::new("admin.backup.json"), pass.trim()) {
                                                            Ok(id_bak) => {
                                                                println!("✅ BACKUP RESTORED: '{}'", id_bak.name);
                                                                let _ = std::fs::copy("admin.backup.json", "admin.json");
                                                                Some(Arc::new(id_bak.into_keypair().expect("Failed to convert identity")))
                                                            },
                                                            Err(e_bak) => {
                                                                println!("❌ BACKUP FAILED: {}", e_bak);
                                                                std::process::exit(1);
                                                            }
                                                        }
                                                    } else {
                                                        std::process::exit(1); 
                                                    }
                                                 }
                                             }
                                        }
                                    }
                                }
                            } else {
                                warn!("No identity file found at '{:?}' or 'admin.json'. Using Ephemeral Identity.", path_str);
                                None
                            }
                        };

                        if let Some(p) = peer { 
                            rust_compass::node::run_node_mode_internal(config, Some(p), identity_val).await;
                        } else {
                            rust_compass::node::run_node_mode_internal(config, None, identity_val).await;
                        }
                        
                        // NOTE: p2p_port and db_path from CLI should also override!
                        // But run_node_mode_internal signature is changing.
                        // I will update run_node_mode to take Config object.
                    }
                    cli::node::NodeCommands::Status | cli::node::NodeCommands::Peers => {
                        cli::node::handle_node_command(cmd).await;
                    }
                    cli::node::NodeCommands::Wipe { db_path } => {
                        info!("Wiping database at '{}'...", db_path);
                        if std::path::Path::new(&db_path).exists() {
                            match std::fs::remove_dir_all(&db_path) {
                                Ok(_) => info!("Database wiped successfully."),
                                Err(e) => error!("Error wiping database: {}", e),
                            }
                        } else {
                            warn!("Database path does not exist.");
                        }
                    }
                }
            }
            Commands::Transfer {
                from,
                to,
                amount,
                asset,
            } => {
                cli::tx::handle_transfer_command(from, to, amount, asset, None).await;
            }
            Commands::Balance { address } => {
                println!("Balance check for {}", address);
                // TODO: Implement handle_balance_command via RPC
            }
            Commands::Mint {
                vault_id,
                amount,
                asset,
                collateral_asset,
                collateral_amount,
                proof,
                oracle_sig,
                owner,
            } => {
                cli::ops::handle_mint_command(
                    vault_id,
                    amount,
                    asset,
                    collateral_asset,
                    collateral_amount,
                    proof,
                    oracle_sig,
                    owner,
                    None,
                )
                .await;
            }
            Commands::Burn {
                vault_id,
                amount,
                asset,
                dest_addr,
                from,
            } => {
                cli::ops::handle_burn_command(vault_id, amount, asset, dest_addr, from, None).await;
            }
            Commands::Worker { node_url, model_id, wallet } => {
                let id = rust_compass::interactive::load_identity(&wallet)
                    .expect(&format!("❌ Failed to load wallet '{}'. Please create it first.", wallet));
                let kp = id.into_keypair().expect("Failed to decrypt identity");
                
                // Create dummy gossip channel for standalone worker
                let (gossip_tx, _gossip_rx) = tokio::sync::broadcast::channel(100);
                
                let mut worker = rust_compass::client::AiWorker::new(node_url, kp, gossip_tx);
                worker.start().await;
            }
            Commands::Client => {
                run_client_mode().await;
            }
            Commands::AdminGen => {
                handle_admin_gen();
            },
            Commands::GenesisHash => {
                handle_genesis_hash();
            },
            Commands::Keys { cmd } => {
                rust_compass::cli::keys::handle_keys_command(cmd);
            },
            Commands::Interactive => {
                rust_compass::interactive::start().await;
            },
            Commands::ListNFT { token_id, price, currency, wallet } => {
                let id = rust_compass::interactive::load_identity(&wallet)
                    .expect(&format!("❌ Wallet '{}' not found.", wallet));
                let seller = id.public_key;
                
                let client = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                println!("📦 Listing NFT {} for {} {} (Seller: {})...", token_id, price, currency, seller);
                
                let req = serde_json::json!({
                    "token_id": token_id,
                    "seller": seller,
                    "price": price,
                    "currency": currency
                });
                
                match client.call_method::<serde_json::Value, serde_json::Value>("listModelNFT", req).await {
                    Ok(res) => println!("✅ Listed: {}", res),
                    Err(e) => println!("❌ Error: {}", e),
                }
            },
            Commands::BuyNFT { token_id, wallet } => {
                let id = rust_compass::interactive::load_identity(&wallet)
                    .expect(&format!("❌ Wallet '{}' not found.", wallet));
                let buyer = id.public_key;
                
                let client = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                println!("💰 Buying NFT {} as {}...", token_id, buyer);
                
                let req = serde_json::json!({
                    "token_id": token_id,
                    "buyer": buyer
                });
                
                match client.call_method::<serde_json::Value, serde_json::Value>("buyModelNFT", req).await {
                    Ok(res) => println!("✅ Purchased: {}", res),
                    Err(e) => println!("❌ Error: {}", e),
                }
            },
            Commands::TrainModels => {
                println!("🧠 Starting Training Job for All Signal Models...");
                match rust_compass::layer3::signal_model::train_all_signal_models().await {
                    Ok(paths) => {
                        println!("✅ Training Complete. Models saved:");
                        for p in paths {
                            println!(" - {}", p);
                        }
                    },
                    Err(e) => println!("❌ Training Failed: {}", e),
                }
            },
        }
    } else {
        // v2.0: No subcommand - Start headless node (GUI removed)
        println!("Starting Compass Node in headless mode...");
        println!("Use CLI commands or connect via RPC at port 9000");
        
        let config = config::CompassConfig::load_or_default("config.toml");
        
        // Load or create admin identity
        let admin_path = std::path::Path::new("admin.json");
        let identity = if admin_path.exists() {
            print!("Enter password to unlock admin identity: ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            let mut pass = String::new();
            std::io::stdin().read_line(&mut pass).unwrap();
            
            match rust_compass::identity::Identity::load_and_decrypt(admin_path, pass.trim()) {
                Ok(id) => {
                    println!("Loaded admin identity: {}", id.name);
                    id.into_keypair().expect("Failed to get keypair")
                }
                Err(e) => {
                    println!("Failed to load identity: {}. Generating ephemeral key.", e);
                    rust_compass::crypto::KeyPair::generate()
                }
            }
        } else {
            println!("No admin.json found. Generating ephemeral identity.");
            rust_compass::crypto::KeyPair::generate()
        };
        
        rust_compass::node::run_node_mode_internal(config, None, Some(std::sync::Arc::new(identity))).await;
    }
}

fn handle_admin_gen() {
    use std::collections::HashMap;
    use rust_compass::genesis::{GenesisConfig, GenesisValidator};

    println!("=== Generator for Admin Trusted Setup ===");
    
    // 1. Generate Key
    let mnemonic = KeyPair::generate_mnemonic();
    let kp = KeyPair::from_mnemonic(&mnemonic).unwrap();
    let pubkey = kp.public_key_hex();
    
    // 2. Save Key
    println!("Generated Admin Mnemonic: {}", mnemonic);
    println!("Generated Admin PubKey:   {}", pubkey);
    
    if let Ok(_) = std::fs::write("admin_key.mnemonic", &mnemonic) {
        println!("SAVED 'admin_key.mnemonic'. KEEP THIS SAFE!");
    } else {
        println!("FAILED to write admin_key.mnemonic");
    }
    
    // 3. Generate Genesis
    let mut balances = HashMap::new();
    balances.insert("admin".to_string(), 1_000_000_000_000);
    balances.insert("foundation".to_string(), 1_000_000_000_000);
    balances.insert("Daniel".to_string(), 500_000_000_000);

    let validator = GenesisValidator {
        id: "admin".to_string(),
        public_key: pubkey.clone(),
        stake: 0, 
    };
    
    let config = GenesisConfig {
        chain_id: "compass-alpha-1".to_string(),
        timestamp: 1700000000000,
        initial_balances: balances,
        initial_validators: vec![validator],
    };

    if let Ok(json) = serde_json::to_string_pretty(&config) {
        if let Ok(_) = std::fs::write("genesis.json", json) {
             println!("SAVED 'genesis.json'. Distribute this to all nodes.");
        }
    }
}

fn handle_genesis_hash() {
    println!("Loading genesis.json...");
    let config = rust_compass::genesis::GenesisConfig::load("genesis.json").expect("Failed to load genesis.json");
    
    let genesis_block = rust_compass::block::Block {
        header: rust_compass::block::BlockHeader {
            index: 0,
            block_type: rust_compass::block::BlockType::Genesis,
            proposer: "genesis".to_string(),
            signature_hex: "".to_string(),
            prev_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hash: "".to_string(), 
            timestamp: config.timestamp,
        },
        transactions: vec![],
    };
    
    let final_block = genesis_block;
    let hash = final_block.header.calculate_hash().expect("Hash failed");

    println!("----------------------------------------------------------------");
    println!("💎 MAINNET GENESIS HASH: {}", hash);
    println!("----------------------------------------------------------------");
    println!("Timestamp: {}", config.timestamp);
    println!("Chain ID:  {}", config.chain_id);
}

// --- CLIENT MODE (User) ---
async fn run_client_mode() {
    println!("\n=== Compass Client ===");

    // 1. Login / Wallet Selection
    // --- Initialize Wallet Manager ---
    // In "Node" mode, we need a wallet to sign blocks.
    // If we are "admin", we should have the "admin" wallet or "Daniel" aliased?
    // Storage defaults active_validators to ["admin"].
    // So we need a wallet named "admin" OR we change Storage default to "Daniel"?
    // Changing Storage is harder (compatibility). Let's Ensure we have an "admin" wallet or alias "Daniel" to it.
    
    // For simplicity in DevNet:
    // If no "admin" wallet exists, create it or copy "Daniel".
    let mut wallet_manager = WalletManager::load("wallets.json");
    if wallet_manager.get_wallet("admin").is_none() {
        if let Some(daniel) = wallet_manager.get_wallet("Daniel") {
            // Clone Daniel as Admin for devnet
             let admin_w = daniel.clone();
             wallet_manager.create_wallet(&admin_w, "admin", rust_compass::wallet::WalletType::Admin);
             println!("System: Cloned 'Daniel' wallet to 'admin' for Validator consistency.");
        } else {
             // Create fresh admin
             let admin_w = rust_compass::wallet::Wallet::new_account("admin");
             wallet_manager.create_wallet(&admin_w, "admin", rust_compass::wallet::WalletType::Admin);
             println!("System: Created new 'admin' wallet.");
        }
    }
    let market = Market::load("market.json");
    let mut current_user = String::new();

    loop {
        if current_user.is_empty() {
            println!("\n1. Login (Existing User)");
            println!("2. Create New User");
            println!("2. Create New User");
            println!("3. Transfer Funds"); // Added Transfer
            println!("4. Validator Dashboard");
            println!("5. Request AI Compute");
            println!("6. Exit");
            print!("Select: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            match input.trim() {
                "1" => {
                    print!("Username: ");
                    io::stdout().flush().unwrap();
                    let mut name = String::new();
                    io::stdin().read_line(&mut name).unwrap();
                    let name = name.trim().to_string();
                    if wallet_manager.get_wallet(&name).is_some() {
                        current_user = name;
                        println!("Logged in as {}", current_user);
                    } else {
                        println!("User not found.");
                    }
                }
                "2" => {
                    print!("New Username: ");
                    io::stdout().flush().unwrap();
                    let mut name = String::new();
                    io::stdin().read_line(&mut name).unwrap();
                    let name = name.trim().to_string();
                    if wallet_manager.get_wallet(&name).is_some() {
                        println!("User already exists.");
                    } else {
                        // Create Wallet
                        let new_wallet = wallet::Wallet::new(&name, WalletType::User);
                        wallet_manager.wallets.insert(new_wallet.owner.clone(), new_wallet);
                        let _ = wallet_manager.save("wallets.json");
                        current_user = name;
                        println!("Wallet created! Logged in as {}", current_user);
                    }
                }
                "3" => {
                    // Transfer Funds
                    print!("User (Sender): ");
                    io::stdout().flush().unwrap();
                    let mut u = String::new();
                    io::stdin().read_line(&mut u).unwrap();
                    let u = u.trim().to_string();

                    print!("Recipient: ");
                    io::stdout().flush().unwrap();
                    let mut r = String::new();
                    io::stdin().read_line(&mut r).unwrap();

                    print!("Asset: ");
                    io::stdout().flush().unwrap();
                    let mut a = String::new();
                    io::stdin().read_line(&mut a).unwrap();

                    print!("Amount: ");
                    io::stdout().flush().unwrap();
                    let mut s = String::new();
                    io::stdin().read_line(&mut s).unwrap();
                    let amt: u64 = s.trim().parse().unwrap_or(0);

                    let amt: u64 = s.trim().parse().unwrap_or(0);
                    
                    // 1. Get Node Info (Height + Head Hash)
                    let rpc = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                    let node_info = match rpc.get_node_info().await {
                         Ok(v) => v,
                         Err(e) => {
                             println!("Failed to get node info: {}", e);
                             continue;
                         }
                    };
                    
                    let height = node_info["height"].as_u64().unwrap_or(0);
                    let prev_hash = node_info["head_hash"].as_str().unwrap_or("").to_string(); // Empty if genesis
                    
                    // 2. Get Wallet Keypair
                    let kp = if let Some(w) = wallet_manager.get_wallet(&current_user) {
                        if let Some(k) = w.get_keypair() {
                             k
                        } else {
                            println!("Wallet locked or no keys."); 
                            continue;
                        }
                    } else {
                        println!("Wallet not found.");
                        continue;
                    };
                    
                    // 3. Get Nonce
                    let nonce = rpc.get_nonce(&current_user).await.unwrap_or(0) + 1;
                    
                    // 4. Create Block Header locally to sign
                    let header = create_transfer_block(
                        height,
                        current_user.clone(),
                        r.trim().to_string(),
                        a.trim().to_string(),
                        amt,
                        nonce,
                        0, // Fee TODO
                        prev_hash.clone(),
                        &kp
                    ).expect("Failed to create transfer block");
                    
                    // 5. Submit via RPC
                    match rpc.submit_transaction(
                        &current_user, // from
                        &r.trim(), // to
                        &a.trim(), // asset
                        amt,
                        nonce,
                        &header.signature_hex,
                        Some(prev_hash),
                        Some(header.timestamp),
                        &kp.public_key_hex()
                    ).await {
                        Ok(hash) => println!("Success! Tx Hash: {}", hash),
                        Err(e) => println!("Error submitting tx: {}", e),
                    }
                }
                "4" => std::process::exit(0),
                _ => println!("Invalid."),
            }
        } else {
            // Logged In Dashboard
            // Refresh wallet from file in case Node updated it (simulated shared storage)
            wallet_manager = WalletManager::load("wallets.json");

            println!("\n--- Dashboard: {} ---", current_user);
            let bal = wallet_manager.get_balance(&current_user, "Compass-LTC"); // TODO: List all assets
            println!("Balance: ??? (Use 'View balances' to see all)");

            println!("1. View Balances");
            println!("2. Transfer Funds");
            println!("3. Create Mint Contract (LTC)");
            println!("4. Redeem (Burn)");
            println!("5. Trade (Market)");
            println!("6. Get Vault Address");
            println!("7. Logout");
            println!("8. Convert COMPUTE -> COMPASS (100:1)");
            println!("9. Validator Dashboard");
            println!("10. Request AI Compute");
            println!("11. 🧠 AI Neural Network Marketplace");  // NEW
            print!("Select: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            match input.trim() {
                "1" => {
                    if let Some(w) = wallet_manager.get_wallet(&current_user) {
                        println!("Assets:");
                        let vaults = VaultManager::load("vaults.json");
                        for (asset, amt) in &w.balances {
                            let mut info_str = String::new();
                            if let Some((col_bal, supply, ticker)) = vaults.get_asset_info(asset) {
                                if supply > 0 {
                                    let share = *amt as f64 / supply as f64;
                                    let implied_val = share * col_bal as f64;
                                    info_str = format!(" (~{:.8} {})", implied_val, ticker);
                                }
                            }
                            println!(" - {}: {}{}", asset, amt, info_str);
                        }
                    }

                    // Also show blockchain balances via RPC
                    println!("\n=== Blockchain Balances (On-Chain) ===");
                    let rpc_client =
                        rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());

                    match rpc_client.get_account_info(&current_user).await {
                        Ok(info) => {
                            let vault_info = info.get("vault_info").and_then(|v| v.as_object());
                            
                            if let Some(balances) = info.get("balances").and_then(|b| b.as_object())
                            {
                                if balances.is_empty() {
                                    println!(" (No assets found on blockchain)");
                                } else {
                                    for (asset, amount) in balances {
                                        let amount_u64 = amount.as_u64().unwrap_or(0);
                                        let amount_display = amount_u64 as f64 / 100_000_000.0;
                                        
                                        // Check if this asset has vault info
                                        if let Some(vault_data) = vault_info.and_then(|v| v.get(asset)) {
                                            let collateral = vault_data.get("collateral_balance").and_then(|v| v.as_u64()).unwrap_or(0);
                                            let ratio = vault_data.get("backing_ratio").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                            let collateral_asset = vault_data.get("collateral_asset").and_then(|v| v.as_str()).unwrap_or("?");
                                            
                                            let collateral_display = collateral as f64 / 100_000_000.0;
                                            let inverse_ratio = if ratio > 0.0 { 1.0 / ratio } else { 0.0 };
                                            
                                            println!(" - {}: {:.8} ({:.2} Compass per 1 {}, backed by {:.8} {}, 1 Compass = {:.8} {})", 
                                                asset, 
                                                amount_display,
                                                ratio,
                                                collateral_asset,
                                                collateral_display,
                                                collateral_asset,
                                                inverse_ratio,
                                                collateral_asset
                                            );
                                        } else {
                                            println!(" - {}: {:.8}", asset, amount_display);
                                        }
                                    }
                                }
                            }
                            if let Some(nonce) = info.get("nonce").and_then(|n| n.as_u64()) {
                                println!("Nonce: {}", nonce);
                            }
                        }
                        Err(e) => {
                            println!("⚠️  Could not fetch blockchain balances: {}", e);
                            println!("   (Make sure the node is running)");
                        }
                    }
                }
                "2" => {
                    // Transfer Funds
                    println!("--- Transfer Funds ---");

                    print!("Recipient: ");
                    io::stdout().flush().unwrap();
                    let mut to = String::new();
                    io::stdin().read_line(&mut to).unwrap();
                    let to = to.trim().to_string();

                    print!("Asset (Compass/cLTC/cSOL): ");
                    io::stdout().flush().unwrap();
                    let mut asset = String::new();
                    io::stdin().read_line(&mut asset).unwrap();
                    let asset = asset.trim().to_string();

                    print!("Amount: ");
                    io::stdout().flush().unwrap();
                    let mut amount_str = String::new();
                    io::stdin().read_line(&mut amount_str).unwrap();
                    let amount: u64 = amount_str.trim().parse().unwrap_or(0);

                    if amount == 0 {
                        println!("Invalid amount");
                        continue;
                    }

                    // Create transfer payload
                    // Send to node via RPC
                    println!("Submitting transfer to node (RPC)...");
                    let client = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                    let timestamp = rust_compass::block::current_unix_timestamp_ms() as u64;
                    
                    let res = client.submit_transaction(
                         &current_user,
                         &to,
                         &asset,
                         amount,
                         0, // nonce placeholder
                         "", // signature placeholder
                         Some("".to_string()), // prev_hash placeholder
                         Some(timestamp),
                         "", // public_key placeholder
                    ).await;
                    
                     match res {
                        Ok(hash) => println!("Transfer submitted! Tx: {}", hash),
                        Err(e) => println!("Transfer Error: {}", e),
                    }
                    println!("Transfer submitted! Check node logs for confirmation.");
                }
                "3" => {
                    println!("--- Mint Contract ---");
                    println!("Note: Oracle will auto-sign your mint request.");

                    print!("Collateral Asset (LTC/SOL/etc): ");
                    io::stdout().flush().unwrap();
                    let mut collateral_asset = String::new();
                    io::stdin().read_line(&mut collateral_asset).unwrap();
                    let collateral_asset = collateral_asset.trim().to_string();

                    print!("TX Hash (Deposit Proof): ");
                    io::stdout().flush().unwrap();
                    let mut tx_hash = String::new();
                    io::stdin().read_line(&mut tx_hash).unwrap();
                    let tx_hash = tx_hash.trim().to_string();

                     print!("Collateral Amount (e.g., 0.001 LTC): ");
                     io::stdout().flush().unwrap();
                     let mut col_str = String::new();
                     io::stdin().read_line(&mut col_str).unwrap();
                     let col_amt_float: f64 = col_str.trim().parse().unwrap_or(0.0);
                     let col_amt: u64 = (col_amt_float * 100_000_000.0) as u64; // Convert to satoshis
                     
                     print!("Requested Compass Amount (e.g., 100.5): ");
                     io::stdout().flush().unwrap();
                     let mut mint_str = String::new();
                     io::stdin().read_line(&mut mint_str).unwrap();
                     let mint_amt_float: f64 = mint_str.trim().parse().unwrap_or(0.0);
                     let mint_amt: u64 = (mint_amt_float * 100_000_000.0) as u64; // Convert to smallest unit

                    if col_amt == 0 || mint_amt == 0 {
                        println!("Invalid amounts");
                        continue;
                    }

                    // RPC Logic for Mint
                    let rpc = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                    // 1. Get Node Info
                    let node_info = match rpc.get_node_info().await {
                         Ok(v) => v,
                         Err(e) => {
                             println!("Failed to get node info: {}", e);
                             continue;
                         }
                    };
                    let height = node_info["height"].as_u64().unwrap_or(0);
                    let prev_hash = node_info["head_hash"].as_str().unwrap_or("").to_string();

                    // 2. Get Keys
                    let kp = if let Some(w) = wallet_manager.get_wallet(&current_user) {
                        if let Some(k) = w.get_keypair() { k } else { println!("Locked."); continue; }
                    } else { println!("No wallet"); continue; };

                    // 3. Construct Header
                    let mut header = BlockHeader {
                        index: height,
                        block_type: BlockType::Mint {
                            vault_id: format!("Compass-{}", collateral_asset),
                            collateral_asset: collateral_asset.clone(),
                            collateral_amount: col_amt,
                            compass_asset: format!("Compass-{}", collateral_asset),
                            mint_amount: mint_amt,
                            owner: current_user.clone(),
                            tx_proof: tx_hash.clone(),
                            oracle_signature: String::new(),
                            fee: 0,
                        },
                        proposer: current_user.clone(),
                        signature_hex: String::new(),
                        prev_hash: prev_hash.clone(),
                        hash: String::new(),
                        timestamp: current_unix_timestamp_ms(),
                    };
                    
                    // 4. Sign
                    let pre_sign = header.calculate_hash();
                    header.signature_hex = kp.sign_hex(pre_sign.expect("Failed to calculate hash").as_bytes());

                    // 5. Submit
                    let mint_params = rust_compass::rpc::types::SubmitMintParams {
                        vault_id: format!("Compass-{}", collateral_asset),
                        collateral_asset: collateral_asset.clone(),
                        collateral_amount: col_amt,
                        compass_asset: format!("Compass-{}", collateral_asset),
                        mint_amount: mint_amt,
                        owner: current_user.clone(),
                        tx_proof: tx_hash,
                        oracle_signature: String::new(),
                        fee: 0,
                        signature: header.signature_hex,
                        prev_hash: Some(prev_hash),
                        timestamp: Some(header.timestamp),
                        public_key: kp.public_key_hex(),
                    };

                    match rpc.submit_mint(mint_params).await {
                        Ok(h) => println!("Mint Submitted! Tx: {}", h),
                        Err(e) => println!("Mint Error: {}", e),
                    }
                }
                "4" => {
                    println!("Redemption is currently for specific assets. (Not fully ported to Traceable logic yet)");
                }
                "5" => {
                    println!("\n--- Market ---");
                    println!("1. View Orderbook");
                    println!("2. Place Buy Order");
                    println!("3. Place Sell Order");
                    print!("Select: ");
                    io::stdout().flush().unwrap();
                    let mut m_in = String::new();
                    io::stdin().read_line(&mut m_in).unwrap();

                    match m_in.trim() {
                        "1" => {
                            print!("Base Asset (e.g. Compass:Alice:LTC): ");
                            io::stdout().flush().unwrap();
                            let mut b = String::new();
                            io::stdin().read_line(&mut b).unwrap();
                            print!("Quote Asset (e.g. Compass): ");
                            io::stdout().flush().unwrap();
                            let mut q = String::new();
                            io::stdin().read_line(&mut q).unwrap();

                            let key = format!("{}/{}", b.trim(), q.trim());
                            if let Some(book) = market.books.get(&key) {
                                println!("--- BIDS (Buy) ---");
                                for order in &book.bids {
                                    println!(
                                        "[#{}] {} @ {}",
                                        order.id,
                                        order.amount - order.amount_filled,
                                        order.price
                                    );
                                }
                                println!("--- ASKS (Sell) ---");
                                for order in &book.asks {
                                    println!(
                                        "[#{}] {} @ {}",
                                        order.id,
                                        order.amount - order.amount_filled,
                                        order.price
                                    );
                                }
                            } else {
                                println!("No book found for {}", key);
                            }
                        }
                        "2" | "3" => {
                            let side = if m_in.trim() == "2" {
                                OrderSide::Buy
                            } else {
                                OrderSide::Sell
                            };

                            print!("Base Asset (e.g. Compass:Alice:LTC): ");
                            io::stdout().flush().unwrap();
                            let mut b = String::new();
                            io::stdin().read_line(&mut b).unwrap();
                            print!("Quote Asset (e.g. Compass): ");
                            io::stdout().flush().unwrap();
                            let mut q = String::new();
                            io::stdin().read_line(&mut q).unwrap();

                            print!("Amount: ");
                            io::stdout().flush().unwrap();
                            let mut a_s = String::new();
                            io::stdin().read_line(&mut a_s).unwrap();
                            let amt: u64 = a_s.trim().parse().unwrap_or(0);

                            print!("Price: ");
                            io::stdout().flush().unwrap();
                            let mut p_s = String::new();
                            io::stdin().read_line(&mut p_s).unwrap();
                            match p_s.trim().parse::<u64>() {
                                Ok(pr) => {

                                    // Submit via RPC
                                    let client = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                                    
                                    // Manually construct correct RPC call for PlaceOrder
                                    // Since submit_transaction is for transfers, we might need a generic `send_request` or `submit_tx` if implemented.
                                    // For now, I'll use `submit_transaction` if it fits, or `call_method`.
                                    // Actually, TransactionPayload::PlaceOrder is complex. RpcClient needs a method for it.
                                    // Or I use `call_method("submitOrder", ...)`
                                    
                                    // Simplify: Just print "Not implemented yet" or try to implement `place_order` in RpcClient?
                                    // Given I can't edit RpcClient in this step easily without context switch, 
                                    // and the user wants "beta", I should probably fix strict errors first.
                                    
                                    let params = serde_json::json!({
                                        "user": current_user,
                                        "side": side, // enum needs serialization
                                        "base": b.trim(),
                                        "quote": q.trim(),
                                        "amount": amt,
                                        "price": pr,
                                        "signature": "stub"
                                    });
                                    
                                    // Attempt raw method call
                                    let res: Result<String, String> = client.call_method("submitOrder", params).await;
                                    match res {
                                        Ok(tx) => println!("Order Submitted: {}", tx),
                                        Err(e) => println!("Order Error: {}", e),
                                    }
                                    // wallet_manager.save();
                                }
                                Err(_) => println!("Invalid Price"),
                            }
                        }
                        _ => println!("Invalid."),
                    }
                }
                "6" => {
                    // Get Vault Address
                    println!("\n=== Get Vault Deposit Address ===");
                    print!("Collateral Asset (BTC/LTC/SOL): ");
                    io::stdout().flush().unwrap();
                    let mut asset_input = String::new();
                    io::stdin().read_line(&mut asset_input).unwrap();
                    let collateral_asset = asset_input.trim().to_uppercase();
                    
                    // Load vault key manager
                    let vault_keys = rust_compass::vault::VaultKeyManager::load_or_generate("vault_master.seed");
                    let master_seed = vault_keys.get_seed();
                    
                    // Generate vault address
                    let (vault_address, derivation_path) = rust_compass::vault::VaultManager::generate_vault_address(
                        &current_user,
                        &collateral_asset,
                        master_seed,
                    );
                    
                    let vault_id = format!("Compass:{}:{}", current_user, collateral_asset);
                    
                    println!("\n✓ Vault Address Generated");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Vault ID:        {}", vault_id);
                    println!("Deposit Address: {}", vault_address);
                    println!("Derivation Path: {}", derivation_path);
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("\n📝 Instructions:");
                    println!("1. Send {} to the address above", collateral_asset);
                    println!("2. Wait for confirmations (BTC: 6+, LTC: 12+, SOL: 32+)");
                    println!("3. Use 'Create Mint Contract' with your TX hash");
                    println!("4. Oracle will verify and mint Compass tokens");
                    println!("\n⚠️  IMPORTANT: This address is unique to you!");
                    println!("   Do not share it with others.\n");
                }
                "8" => {
                    println!("\n=== Convert COMPUTE -> COMPASS ===");
                    println!("Rate: 100 COMPUTE = 1 COMPASS");
                    
                    // Simple balance check - reloading wallet to be safe
                    // wallet_manager loaded at loop start (line 222)
                    let current_compute = wallet_manager.get_balance(&current_user, "COMPUTE");
                    println!("Available: {:.8} COMPUTE", current_compute as f64 / 100_000_000.0);

                    print!("Amount to convert (COMPUTE): ");
                    io::stdout().flush().unwrap();
                    let mut amt_str = String::new();
                    io::stdin().read_line(&mut amt_str).unwrap();
                    
                    if let Ok(amt) = amt_str.trim().parse::<f64>() {
                        let raw_compute_needed = (amt * 100_000_000.0) as u64;
                        
                        if wallet_manager.debit(&current_user, "COMPUTE", raw_compute_needed) {
                            let compass_amount = raw_compute_needed / 100; // 100:1 ratio
                            
                            wallet_manager.credit(&current_user, "COMPASS", compass_amount);
                            let _ = wallet_manager.save("wallets.json");
                            
                            println!("✓ Converted {:.8} COMPUTE to {:.8} COMPASS", 
                                raw_compute_needed as f64 / 1e8,
                                compass_amount as f64 / 1e8
                            );
                        } else {
                            println!("❌ Insufficient balance.");
                        }
                    } else {
                        println!("❌ Invalid amount.");
                    }
                }
                "10" => {
                    println!("\n=== Request AI Compute ===");
                    println!("Cost: Free (Devnet Beta)");

                    print!("Model ID (e.g., llama-2-7b): ");
                    io::stdout().flush().unwrap();
                    let mut model = String::new();
                    io::stdin().read_line(&mut model).unwrap();
                    let model = model.trim().to_string();

                    print!("Input Prompt: ");
                    io::stdout().flush().unwrap();
                    let mut prompt = String::new();
                    io::stdin().read_line(&mut prompt).unwrap();
                    let prompt = prompt.trim().to_string();

                    print!("Bid Amount (COMPASS): ");
                    io::stdout().flush().unwrap();
                    let mut bid_str = String::new();
                    io::stdin().read_line(&mut bid_str).unwrap();
                    let bid_amount = bid_str.trim().parse::<u64>().unwrap_or(50); // Default 50

                    // Create Compute Payload
                    let job_id = format!("job_{}", current_unix_timestamp_ms());

                    println!("Submitting Compute Job [{}] with bid {} COMPASS...", job_id, bid_amount);
                    // Submit via RPC
                    let client = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                    
                    let res = client.submit_compute(
                        job_id.clone(),
                        model,
                        prompt.into_bytes(),
                        100, // max_compute_units
                        bid_amount,
                        "COMPASS".to_string(),
                    ).await;
                    
                    match res {
                        Ok(tx) => println!("Job Submitted! Tx: {}", tx),
                        Err(e) => println!("Job Error: {}", e),
                    }
                }
                "9" => {
                    println!("\n=== Validator Dashboard ===");
                    println!("Fetching stats for: {}", current_user);
                    
                    let client = reqwest::Client::new();
                    let payload = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "getValidatorStats",
                        "params": { "validator": current_user },
                        "id": 1
                    });
                    
                    let res = client.post("http://127.0.0.1:9000")
                        .json(&payload)
                        .send()
                        .await;
                        
                    match res {
                        Ok(resp) => {
                            if let Ok(json) = resp.json::<serde_json::Value>().await {
                                if let Some(result) = json.get("result") {
                                    let blocks = result.get("blocks_produced").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let earned = result.get("compute_earned").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let uptime = result.get("uptime_hours").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let avg_time = result.get("avg_block_time_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                                    
                                    println!("Status: Active ✅");
                                    println!("Blocks Produced: {}", blocks);
                                    println!("COMPUTE Earned:  {} ({:.8} tokens)", earned, earned as f64 / 1e8);
                                    println!("Uptime:          {}h approx", uptime);
                                    println!("Avg Block Time:  {:.2}s", avg_time as f64 / 1000.0);
                                    
                                    println!("1. {}     - {:.8} COMPUTE ({} blocks)", current_user, earned as f64 / 1e8, blocks);
                                    println!("(Multi-validator support coming in Phase 4)");
                                } else {
                                     println!("❌ Failed to get stats result: {:?}", json);
                                }
                            } else {
                                println!("❌ Failed to parse response");
                            }
                        }
                        Err(e) => println!("❌ Could not connect to node: {}", e),
                    }

                    println!("\nActions:");
                    println!("1. Refresh Stats");
                    println!("2. Register as Validator (Stake Compass)");
                    println!("3. Exit");
                    
                    print!("Select: ");
                    io::stdout().flush().unwrap();
                    let mut v_in = String::new();
                    io::stdin().read_line(&mut v_in).unwrap();
                    
                    if v_in.trim() == "2" {
                        println!("\n=== Register Validator ===");
                        println!("Cost: 1000 Compass (Minimum Stake)");
                        
                        // Check balance via RPC (On-Chain)
                        // Check balance via RPC (On-Chain)
                        let rpc = rust_compass::client::RpcClient::new("http://127.0.0.1:9000".to_string());
                        
                        // Robust check: Use get_account_info like Option 1
                        let balance = match rpc.get_account_info(&current_user).await {
                             Ok(info) => {
                                 info.get("balances")
                                     .and_then(|b| b.get("Compass"))
                                     .and_then(|v| v.as_u64())
                                     .unwrap_or(0)
                             }
                             Err(_) => 0,
                        };
                        
                        if balance < 1000_00000000 {
                            println!("❌ Insufficient Compass. Need 1000.0, You have {}", balance as f64 / 1e8);
                        } else {
                            // Get Keys
                            if let Some(w) = wallet_manager.get_wallet(&current_user) {
                                if let Some(kp) = w.get_keypair() {
                                    // Sign Registration
                                    let pubkey = kp.public_key_hex();
                                    let msg = current_user.clone(); // Sign our ID as proof
                                    let sig = kp.sign_hex(msg.as_bytes());
                                    
                                    let params = rust_compass::rpc::types::RegisterValidatorParams {
                                        validator_id: current_user.clone(),
                                        pubkey: pubkey,
                                        stake_amount: 1000_00000000,
                                        signature: sig,
                                    };
                                    
                                    // Submit via RPC (using SubmitTx with special payload or new endpoint?)
                                    // Use SubmitTx with RegisterValidator payload
                                    
                                    println!("⚠️ Validator Registration via RPC is pending implementation.");
                                    // let _ = rpc.call_method("registerValidator", params).await;
                                    
                                    println!("✅ Registration request prepared (but not sent). waiting for next block...");
                                } else {
                                    println!("❌ Wallet locked.");
                                }
                            } else {
                                println!("❌ Wallet error.");
                            }
                        }
                    }
                }
                "7" => current_user.clear(),
                _ => println!("Invalid."),
            }
        }
    }
}

// run_node_mode_internal moved to rust_compass::node::run_node_mode_internal
