mod crypto;
mod block;
mod chain;
mod wallet;
mod vdf;
mod vault;
mod oracle;
mod market;
mod gulf_stream; // Added module declaration
mod storage; // Added storage module
mod rpc; // RPC server
mod client; // RPC client

mod cli; // CLI module
mod network;

use network::{NetMessage, TransactionPayload, connect_and_send}; 
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // Added AsyncReadExt and AsyncWriteExt traits
use sha2::Digest; // Import Digest trait
use crypto::KeyPair;
use block::{
    create_poh_block, create_vote_block,
    create_proposal_block, create_transfer_block, current_unix_timestamp_ms,
};

use chain::Chain;
use wallet::{WalletManager, WalletType};
use vault::VaultManager;
use market::{Market, OrderSide};
use gulf_stream::manager::CompassGulfStreamManager;
use std::io::{self, Write};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::fs::OpenOptions;

use clap::Parser;
use cli::{Cli, Commands};

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
    let cli = Cli::parse();
    
    // Check if any specific command is provided
    if let Some(command) = cli.command {
        match command {
            Commands::Wallet { cmd } => {
                cli::wallet::handle_wallet_command(cmd);
            },
            Commands::Account { cmd } => {
                cli::wallet::handle_account_command(cmd);
            },
            Commands::Node { cmd } => {
                // If "compass node start" is called
                match cmd {
                    cli::node::NodeCommands::Start { rpc_port, peer } => {
                        // Start the full node
                        run_node_mode(Some(rpc_port), peer).await;
                    }
                    cli::node::NodeCommands::Status | cli::node::NodeCommands::Peers => {
                        cli::node::handle_node_command(cmd).await;
                    }
                }
            },
             Commands::Transfer { from, to, amount, asset } => {
                 cli::tx::handle_transfer_command(from, to, amount, asset, None).await;
             },
             Commands::Balance { address } => {
                 println!("Balance check for {}", address);
                 // TODO: Implement handle_balance_command via RPC
             },
             Commands::Mint { vault_id, amount, asset, collateral_asset, collateral_amount, proof, oracle_sig, owner } => {
                 cli::ops::handle_mint_command(vault_id, amount, asset, collateral_asset, collateral_amount, proof, oracle_sig, owner, None).await;
             },
             Commands::Burn { vault_id, amount, asset, dest_addr, from } => {
                 cli::ops::handle_burn_command(vault_id, amount, asset, dest_addr, from, None).await;
             }
        }
    } else {
        // No subcommand? Use existing logic:
        // If --node was a flag in old version, now it's a subcommand "node start"
        // If previously it ran client interactive mode by default:
        run_client_mode().await;
    }
}

// --- CLIENT MODE (User) ---
async fn run_client_mode() {
    println!("\n=== Compass Client ===");
    
    // 1. Login / Wallet Selection
    let mut wallet_manager = WalletManager::load("wallets.json");
    let mut market = Market::load("market.json");
    let mut current_user = String::new();

    loop {
        if current_user.is_empty() {
             println!("\n1. Login (Existing User)");
             println!("2. Create New User");
             println!("3. Transfer Funds"); // Added Transfer
             println!("4. Exit");
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
                 },
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
                        wallet_manager.wallets.push(new_wallet);
                        wallet_manager.save("wallets.json");
                        current_user = name;
                        println!("Wallet created! Logged in as {}", current_user);
                    }
                 },
                 "3" => {
                    // Transfer Funds
                    print!("User (Sender): ");
                    io::stdout().flush().unwrap();
                    let mut u = String::new(); io::stdin().read_line(&mut u).unwrap();
                    let u = u.trim().to_string();
                    
                    print!("Recipient: ");
                    io::stdout().flush().unwrap();
                    let mut r = String::new(); io::stdin().read_line(&mut r).unwrap();
                    
                    print!("Asset: ");
                    io::stdout().flush().unwrap();
                    let mut a = String::new(); io::stdin().read_line(&mut a).unwrap();
                    
                    print!("Amount: ");
                    io::stdout().flush().unwrap();
                    let mut s = String::new(); io::stdin().read_line(&mut s).unwrap();
                    let amt: u64 = s.trim().parse().unwrap_or(0);
                    
                    // Create Payload
                    let payload = TransactionPayload::Transfer {
                        from: u,
                        to: r.trim().to_string(),
                        asset: a.trim().to_string(),
                        amount: amt,
                        nonce: 0,
                        signature: "stub_sig".to_string(),
                    };
                    
                    connect_and_send("127.0.0.1:9000", NetMessage::SubmitTx(payload)).await;
                    println!("Transfer Submitted to Network!");
                 },
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
            println!("6. Logout");
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
                     let rpc_client = crate::client::RpcClient::new("http://127.0.0.1:8899".to_string());
                     
                     match rpc_client.get_account_info(&current_user).await {
                         Ok(info) => {
                             if let Some(balances) = info.get("balances").and_then(|b| b.as_object()) {
                                 if balances.is_empty() {
                                     println!(" (No assets found on blockchain)");
                                 } else {
                                     for (asset, amount) in balances {
                                         println!(" - {}: {}", asset, amount);
                                     }
                                 }
                             }
                             if let Some(nonce) = info.get("nonce").and_then(|n| n.as_u64()) {
                                 println!("Nonce: {}", nonce);
                             }
                         },
                         Err(e) => {
                             println!("⚠️  Could not fetch blockchain balances: {}", e);
                             println!("   (Make sure the node is running)");
                         }
                     }
                },
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
                     let payload = TransactionPayload::Transfer {
                         from: current_user.clone(),
                         to,
                         asset,
                         amount,
                         nonce: 0,
                         signature: String::new(), // Will be signed by node
                     };
                     
                     // Send to node
                     println!("Submitting transfer to node...");
                     connect_and_send("127.0.0.1:9000", NetMessage::SubmitTx(payload)).await;
                     println!("Transfer submitted! Check node logs for confirmation.");
                },
                "3" => {
                     println!("--- Mint Contract ---");
                     println!("Note: You must obtain an Oracle Signature from the Node for your deposit first.");
                     
                     print!("TX Hash: ");
                     io::stdout().flush().unwrap();
                     let mut tx = String::new();
                     io::stdin().read_line(&mut tx).unwrap();
                     
                     print!("Collateral Amount (Sats/Units): ");
                     io::stdout().flush().unwrap();
                     let mut col_str = String::new();
                     io::stdin().read_line(&mut col_str).unwrap();
                     let col_amt: u64 = col_str.trim().parse().unwrap_or(0);
                     
                     print!("Requested Compass Amount: ");
                     io::stdout().flush().unwrap();
                     let mut mint_str = String::new();
                     io::stdin().read_line(&mut mint_str).unwrap();
                     let mint_amt: u64 = mint_str.trim().parse().unwrap_or(0);
                     
                     print!("Oracle Signature: ");
                     io::stdout().flush().unwrap();
                     let mut sig = String::new();
                     io::stdin().read_line(&mut sig).unwrap();
                     
                     print!("Oracle Public Key: ");
                     io::stdout().flush().unwrap();
                     let mut pubkey_hex = String::new();
                     io::stdin().read_line(&mut pubkey_hex).unwrap();
                     let admin_pubkey = pubkey_hex.trim().to_string(); // Use user input

                     // Load Vaults
                     let mut vaults = VaultManager::load("vaults.json");
                     
                     // Execute
                     match vaults.deposit_and_mint("LTC", col_amt, mint_amt, &current_user, tx.trim(), sig.trim(), &admin_pubkey) {
                         Ok((asset, minted)) => {
                             println!("Success! Minted {} {}", minted, asset);
                             // Update Wallet
                             wallet_manager.credit(&current_user, &asset, minted);
                             wallet_manager.save("wallets.json");
                             vaults.save("vaults.json");
                         },
                         Err(e) => println!("Mint Failed: {}", e),
                     }
                },
                "4" => {
                     println!("Redemption is currently for specific assets. (Not fully ported to Traceable logic yet)");
                },
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
                            let mut b = String::new(); io::stdin().read_line(&mut b).unwrap();
                            print!("Quote Asset (e.g. Compass): ");
                            io::stdout().flush().unwrap();
                            let mut q = String::new(); io::stdin().read_line(&mut q).unwrap();
                            
                            let key = format!("{}/{}", b.trim(), q.trim());
                            if let Some(book) = market.books.get(&key) {
                                println!("--- BIDS (Buy) ---");
                                for order in &book.bids {
                                    println!("[#{}] {} @ {}", order.id, order.amount - order.amount_filled, order.price);
                                }
                                println!("--- ASKS (Sell) ---");
                                for order in &book.asks {
                                    println!("[#{}] {} @ {}", order.id, order.amount - order.amount_filled, order.price);
                                }
                            } else {
                                println!("No book found for {}", key);
                            }
                        },
                        "2" | "3" => {
                            let side = if m_in.trim() == "2" { OrderSide::Buy } else { OrderSide::Sell };
                            
                            print!("Base Asset (e.g. Compass:Alice:LTC): ");
                            io::stdout().flush().unwrap();
                            let mut b = String::new(); io::stdin().read_line(&mut b).unwrap();
                            print!("Quote Asset (e.g. Compass): ");
                            io::stdout().flush().unwrap();
                            let mut q = String::new(); io::stdin().read_line(&mut q).unwrap();
                            
                            print!("Amount: ");
                            io::stdout().flush().unwrap();
                            let mut a_s = String::new(); io::stdin().read_line(&mut a_s).unwrap();
                            let amt: u64 = a_s.trim().parse().unwrap_or(0);

                            print!("Price: ");
                            io::stdout().flush().unwrap();
                            let mut p_s = String::new(); io::stdin().read_line(&mut p_s).unwrap();
                            match p_s.trim().parse::<u64>() {
                                Ok(pr) => {
                                     let payload = TransactionPayload::PlaceOrder {
                                         user: current_user.clone(),
                                         side,
                                         base: b.trim().to_string(),
                                         quote: q.trim().to_string(),
                                         amount: amt,
                                         price: pr,
                                         signature: "stub_sig".to_string(),
                                     };
                                     connect_and_send("127.0.0.1:9000", NetMessage::SubmitTx(payload)).await;
                                     println!("Order Submitted to Network!");
                                     // market.save(); // No longer saving locally in client
                                     // wallet_manager.save();
                                },
                                Err(_) => println!("Invalid Price"),
                            }
                        },
                        _ => println!("Invalid."),
                    }
                }, 
                "6" => current_user.clear(),
                _ => println!("Invalid."),
            }
        }
    }
}

// --- NODE MODE (Infrastructure) ---
async fn run_node_mode(rpc_port: Option<u16>, peer_val: Option<String>) {
    println!("Starting Compass Node...");
    let port = 9000; // P2P port remains fixed for now, or could be arg.
    let peer_addr = peer_val.clone(); // Use passed peer address
    let follower_mode = peer_addr.is_some();
    
    let rpc_port = rpc_port.unwrap_or(8899);

    if follower_mode {
        println!("Running in FOLLOWER mode - will sync from peer, not generate blocks");
    }


    // --- Setup admin ---
    let admin = Arc::new(KeyPair::new());
    let admin_wallet_id = "admin".to_string();
    let admin_pubkey_hex = admin.public_key_hex();

    // --- Load wallets ---
    let mut wallet_manager = WalletManager::load("wallets.json");
    if wallet_manager.get_wallet("Daniel").is_none() {
        let admin_wallet = wallet::Wallet::new("Daniel", WalletType::Admin);
        wallet_manager.wallets.push(admin_wallet);
        wallet_manager.save("wallets.json");
        println!("Created admin wallet for Daniel");
    }

    // --- Network Setup ---
    let my_p2p_port = 9000;
    let peer_manager = Arc::new(Mutex::new(crate::network::PeerManager::new(my_p2p_port)));
    
    // Channel for P2P messages (Gossip)
    let (gossip_tx, mut gossip_rx) = tokio::sync::broadcast::channel::<crate::network::NetMessage>(100);

    // --- Start P2P Server ---

    
    // Connect to peer if specified
    if let Some(paddr) = peer_val {
        let pm_connect = peer_manager.clone();
        tokio::spawn(async move {
             crate::network::connect_to_peer(&paddr, my_p2p_port, pm_connect).await;
        });
    }

    // --- Load chain ---
    let storage_arc = Arc::new(crate::storage::Storage::new("compass_db"));
    let chain = Arc::new(Mutex::new(Chain::new(storage_arc)));

    // --- Start P2P Server ---
    let pm_clone = peer_manager.clone();
    let gtx_clone = gossip_tx.clone();
    let chain_for_net = Arc::clone(&chain);
    tokio::spawn(async move {
        crate::network::start_server(my_p2p_port, pm_clone, gtx_clone, chain_for_net).await;
    });
    let wallets = Arc::new(Mutex::new(wallet_manager));
    let vaults = Arc::new(Mutex::new(VaultManager::load("vaults.json")));
    let market = Arc::new(Mutex::new(Market::load("market.json"))); // Load Market
    let gulf_stream = Arc::new(Mutex::new(CompassGulfStreamManager::new("Node1".to_string(), 1000)));

    // --- Start RPC Server ---
    let rpc_chain = Arc::clone(&chain);
    let rpc_pm = Arc::clone(&peer_manager);
    let rpc_gs = Arc::clone(&gulf_stream);
    let rpc_port_clone = rpc_port;
    tokio::spawn(async move {
        let server = crate::rpc::RpcServer::new(rpc_chain, rpc_pm, rpc_gs, rpc_port_clone);
        server.start().await;
    });

    // --- Handle Gossip / P2P Messages ---
    let chain_p2p = chain.clone();
    let gs_p2p = gulf_stream.clone();
    
    // Spawn message processor
    tokio::spawn(async move {
        while let Ok(msg) = gossip_rx.recv().await {
            match msg {
                crate::network::NetMessage::SubmitTx(payload) => {
                     println!("[P2P] Received Transaction from Network");
                     let raw_tx = bincode::serialize(&payload).unwrap();
                     let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec(); // Simple hash assuming Vec<u8> key
                     
                     let mut gs = gs_p2p.lock().unwrap();
                     gs.add_transaction(tx_hash, raw_tx, 0); // Priority 0 by default
                     println!("[GulfStream] Queued Tx (Simulated)");
                },
                crate::network::NetMessage::NewPeer { addr } => {
                    println!("[P2P] New Peer Discovered via Gossip: {}", addr);
                    // Add to PeerManager? (Done in handler already, but maybe we connect?)
                },
                _ => {} // Ignore Ping/Pong/etc here
            }
        }
    });

    // --- Genesis Balance Initialization ---
    {
        let chain_lock = chain.lock().unwrap();
        if chain_lock.height == 0 {
            // Initialize genesis balances
            if let Err(e) = chain_lock.storage.set_balance("Daniel", "Compass", 100_000) {
                println!("Warning: Failed to set genesis balance: {}", e);
            }
            if let Err(e) = chain_lock.storage.set_nonce("Daniel", 0) {
                println!("Warning: Failed to set genesis nonce: {}", e);
            }
            println!("Genesis: Initialized Daniel with 100,000 Compass");
        }
    }

    // --- Genesis Mint (from Foundation Reserve) ---
    {
        let mut wallets = wallets.lock().unwrap();
        if wallets.get_balance("Daniel", "Compass") == 0 {
            wallets.credit("Daniel", "Compass", 100_000);
            wallets.save("wallets.json");
            log_to_file("GENESIS: Minted 100,000 Compass to Daniel (Unbacked Foundation Reserve)");
            println!("GENESIS: Minted 100,000 Compass to Daniel");
        }
    }

    // --- Spawn PoH + VDF loop (only if NOT in follower mode) ---
    if !follower_mode {
        let chain = Arc::clone(&chain);
        // let wallets = Arc::clone(&wallets); // Unused in loop now
        let admin = Arc::clone(&admin);
        // let admin_wallet_id = admin_wallet_id.clone(); // Unused in loop

        thread::spawn(move || {
            let mut tick: u64 = 0;
            let mut seed = b"COMPASS_GENESIS_SEED".to_vec();
            
            // Restore state from Chain if exists (Updated for Storage)
            {
                let chain_guard = chain.lock().unwrap();
                let head = chain_guard.head_hash();
                if let Some(h) = head {
                     // Get head block
                     if let Ok(Some(block)) = chain_guard.storage.get_block(&h) {
                          // Simple restore: use head VDF hash as seed, and height as tick
                          // This assumes strictly one PoH block per height (ok for PoH chain)
                          // In reality, we might mix block types.
                          // Ideally we scan backwards for last PoH block.
                          // For prototype, let's just use head logic if it IS a PoH block.
                          if let block::BlockType::PoH { tick: last_tick, hash: ref last_hash, .. } = block.header.block_type {
                                tick = last_tick;
                                if let Ok(decoded_hash) = hex::decode(last_hash) {
                                    seed = decoded_hash;
                                }
                                println!("Restored PoH State: Tick={}, VDF Hash={}", tick, last_hash);
                          }
                     }
                }
            }

            let mut vdf_state = vdf::VDFState::new(seed);
            
            // Target: ~400ms per block (Solana style)
            // Based on logs: ~290k H/s on this machine.
            // 120,000 / 290,000 ~= 0.41s
            let iterations_per_tick = 120_000;

            loop {
                // Run VDF Work (CPU intensive!)
                let start = std::time::Instant::now();
                let start_hash = vdf_state.current_hash.clone();
                let end_hash = vdf_state.execute(iterations_per_tick);
                let duration = start.elapsed();

                {
                    let mut chain = chain.lock().unwrap();

                    // Bootstrap genesis if empty (rarely happens here if we restored, but good for clean start)
                    if chain.head_hash().is_none() {
                         let genesis = create_poh_block(
                            chain.height, // Added index
                            "GENESIS".to_string(), 
                            0, 
                            0, 
                            start_hash.clone(), // initial seed
                            &admin
                        );
                        // Genesis has no prev_hash (GENESIS)
                        // append_poh verifies.
                        // We must be careful about "GENESIS" hash check.
                        // Impl in Chain allows GENESIS if no head.
                        chain.append_poh(genesis, &admin.public_key_hex()).expect("Genesis PoH failed");
                        println!("Genesis PoH block created.");
                    }

                    // PoH tick (signed by admin)
                    tick += 1;
                    let poh = create_poh_block(
                        chain.height, // Added index (current height will be index of new block if we synced properly? No, chain.height IS next index)
                        chain.head_hash().unwrap_or_default(),
                        tick,
                        iterations_per_tick,
                        end_hash.clone(),
                        &admin,
                    );
                    if let Err(e) = chain.append_poh(poh, &admin.public_key_hex()) {
                         println!("PoH Append Error: {}", e);
                         // Don't crash loop, just retry?
                    }
                }

                // Log periodically (outside lock)
                if tick % 10 == 0 || tick == 1 {
                    let chain = chain.lock().unwrap();
                    let millis = duration.as_millis();
                    // Avoid divide by zero
                    let hps = if millis > 0 { (iterations_per_tick as u128 * 1000) / millis } else { 0 };
                    
                    println!("PoH Tick #{}: {} hashes in {}ms (~{} H/s)", 
                        tick, iterations_per_tick, millis, hps);
                    log_to_file(&format!("PoH Tick #{}: {}ms (~{} H/s)", tick, millis, hps));
                    // chain.save_to_json // REMOVED (No need, DB persists instantly)
                }
                
                // Sleep to control block production rate (1 block per second)
                // Calculate how much time to sleep to reach 1 second total
                let elapsed_ms = duration.as_millis() as u64;
                let target_ms = 1000; // 1 second per block
                if elapsed_ms < target_ms {
                    let sleep_ms = target_ms - elapsed_ms;
                    thread::sleep(Duration::from_millis(sleep_ms));
                }
            }
        });
    } else {
        println!("PoH generation DISABLED (follower mode)");
    }

    // --- Oracle / Smart Contract Setup ---
    let oracle_service = Arc::new(tokio::sync::Mutex::new(oracle::OracleService::new(
        "ltc1qunzw2r558tm6ln7fnxhqxqy0mkz8kkdretf75h", 
        admin.clone()
    )));

    // (Old RPC and Network setup removed in favor of P2P integration above)
    // The main loop keeps running...


    // --- Transaction Processor Loop ---
    tokio::spawn({
        let gulf_stream = Arc::clone(&gulf_stream);
        let wallets = Arc::clone(&wallets);
        let market = Arc::clone(&market);
        let chain = Arc::clone(&chain);
        
        async move {
            loop {
                // 1. Get Pending Txs
                let mut txs_to_process = Vec::new();
                {
                    let mut gs = gulf_stream.lock().unwrap();
                    // Simple FIFO for prototype: pop all pending
                    let keys: Vec<Vec<u8>> = gs.pending_transactions.keys().cloned().collect();
                    for k in keys {
                        if let Some(tx) = gs.pending_transactions.get(&k) {
                             txs_to_process.push(tx.clone());
                        }
                        gs.confirm_transaction(&k); // Move to processing/confirmed
                    }
                }

                // 2. Execute Txs
                // 2. Execute Txs
                if !txs_to_process.is_empty() {
                    let mut w_guard = wallets.lock().unwrap();
                    let mut m_guard = market.lock().unwrap();
                    let mut c_guard = chain.lock().unwrap();

                    for tx in txs_to_process {
                         if let Ok(payload) = bincode::deserialize::<TransactionPayload>(&tx.raw_tx) {
                             match payload {
                                 TransactionPayload::Transfer { from, to, asset, amount, nonce, signature } => {
                                     // Construct Block Logic using Chain
                                     let prev_hash = c_guard.head_hash().unwrap_or_default();
                                     let index = c_guard.height;
                                     let timestamp = current_unix_timestamp_ms(); // Use global helper or system time
                                     
                                     let header = crate::block::BlockHeader {
                                         index,
                                         timestamp: timestamp as u64,
                                         prev_hash: prev_hash.clone(),
                                         hash: "".to_string(), // Recalculated
                                         proposer: "Validator1".to_string(), // Placeholder or Miner ID
                                         signature_hex: signature.clone(),
                                         block_type: crate::block::BlockType::Transfer {
                                             from: from.clone(),
                                             to: to.clone(),
                                             asset: asset.clone(),
                                             amount,
                                             nonce, 
                                             fee: 0, 
                                         }
                                     };
                                     
                                     // Calculate Self Hash
                                     let mut header_final = header.clone();
                                     header_final.hash = header_final.calculate_hash();

                                     // Append to Chain (DB)
                                     match c_guard.append_transfer(header_final, &from) {
                                         Ok(_) => println!("EXEC: Mined Transfer Block {} -> {}", from, to),
                                         Err(e) => println!("EXEC: Mining Failed: {}", e),
                                     }
                                 },
                                 TransactionPayload::PlaceOrder { user, side, base, quote, amount, price, .. } => {
                                      println!("EXEC: Place Order for {}", user);
                                      let _ = m_guard.place_order(&user, side, &base, &quote, amount, price, &mut w_guard);
                                 },
                                 TransactionPayload::Mint(_) | TransactionPayload::Burn(_) => {
                                     // Already handled?
                                 }
                             }
                         }
                    }
                    // Save State
                     w_guard.save("wallets.json");
                     m_guard.save("market.json");
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    });



    // --- Sync Loop (Active) ---
    // Only sync if peer address is explicitly provided
    if let Some(peer_addr) = peer_addr {
        let chain = Arc::clone(&chain);
        let wallets = Arc::clone(&wallets); // Need to update state if blocks processed? 
        // Currently append_block doesn't execute txs automatically on history sync?
        // Ideally we re-process, but for prototype just appending to chain is good step 1.
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await; // Sync every 1 second
                
                let my_height = {
                    let chain = chain.lock().unwrap();
                    chain.height
                };

                println!("Sync: Requesting blocks from {} starting at {}", peer_addr, my_height);

                if let Ok(mut stream) = tokio::net::TcpStream::connect(&peer_addr).await {
                     let req = NetMessage::RequestBlocks { start_height: my_height, end_height: my_height + 18 }; // 18 blocks per second
                     let data = bincode::serialize(&req).unwrap();
                     if stream.write_all(&data).await.is_ok() {
                         // Read response
                         let mut buf = vec![0u8; 10_000_000]; // 10MB buffer for large block batches
                         if let Ok(n) = stream.read(&mut buf).await {
                             if n > 0 {
                                 if let Ok(NetMessage::SendBlocks(blocks)) = bincode::deserialize(&buf[..n]) {
                                     println!("Sync: Received {} blocks", blocks.len());
                                     let mut chain = chain.lock().unwrap();
                                     for block in blocks {
                                         // Append downloaded block
                                         if let Err(e) = chain.sync_block(block.header) {
                                            println!("Sync Error: Failed to append block: {}", e);
                                            break;
                                         } else {
                                            println!("Sync: Appended block height {}", chain.height);
                                         }
                                     } 
                                 }
                             }
                         }
                     }
                } else {
                    println!("Sync: Failed to connect to {}", peer_addr);
                }
            }
        });
    } else {
        println!("No peer specified. Running in standalone mode. Use --peer <address> to enable sync.");
    }

    // --- Admin Menu Loop (Main Thread) ---
    loop {
        println!("\n=== Compass Node Admin ===");
        println!("1. Show status");
        println!("2. Create proposal");
        println!("3. Cast vote");
        println!("4. Tally votes");
        println!("5. Exit");
        println!("6. [Admin] Register Vault");
        println!("7. [Admin] Generate Signature for User Contract");
        print!("Select an option: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim();

        match choice {
            "1" => {
                let chain = chain.lock().unwrap();
                let wallets = wallets.lock().unwrap();
                println!("Chain height: {}", chain.height);
                for w in &wallets.wallets {
                    println!("{}: {:?} ({:?})", w.owner, w.balances, w.wallet_type);
                }
            }
            "2" => {
                print!("Enter proposal text: ");
                io::stdout().flush().unwrap();
                let mut text = String::new();
                io::stdin().read_line(&mut text).unwrap();

                print!("Enter deadline (unix ms): ");
                io::stdout().flush().unwrap();
                let mut deadline_str = String::new();
                io::stdin().read_line(&mut deadline_str).unwrap();
                let deadline: u64 = deadline_str.trim().parse().unwrap();

                let mut chain = chain.lock().unwrap();
                let proposal = create_proposal_block(
                    chain.height,
                    admin_wallet_id.clone(),
                    text.trim().to_string(),
                    deadline,
                    chain.head_hash().unwrap_or_default(),
                    |msg| admin.sign_hex(msg),
                    || current_unix_timestamp_ms(),
                    |id| chain.proposal_id_exists(id),
                ).expect("proposal failed");
                chain.append_proposal(proposal, &admin_pubkey_hex).expect("append proposal failed");
                println!("Proposal appended.");
            }
            "3" => {
                print!("Enter proposal ID: ");
                io::stdout().flush().unwrap();
                let mut id_str = String::new();
                io::stdin().read_line(&mut id_str).unwrap();
                let proposal_id: u64 = id_str.trim().parse().unwrap();

                print!("Vote yes/no: ");
                io::stdout().flush().unwrap();
                let mut vote_str = String::new();
                io::stdin().read_line(&mut vote_str).unwrap();
                let choice = vote_str.trim().eq_ignore_ascii_case("yes");

                let voter = KeyPair::new();
                let mut chain = chain.lock().unwrap();
                let vote_block = create_vote_block(
                    chain.height,
                    "Daniel".to_string(),
                    proposal_id,
                    choice,
                    chain.head_hash().unwrap_or_default(),
                    &voter,
                );
                chain.append_vote(vote_block, &voter.public_key_hex()).expect("append vote failed");
                println!("Vote appended.");
            }
            "4" => {
                print!("Enter proposal ID: ");
                io::stdout().flush().unwrap();
                let mut id_str = String::new();
                io::stdin().read_line(&mut id_str).unwrap();
                let proposal_id: u64 = id_str.trim().parse().unwrap();

                let chain = chain.lock().unwrap();
                let (yes, no) = chain.tally_votes(proposal_id);
                println!("Proposal {} tally: YES={} NO={}", proposal_id, yes, no);
            }
            "5" => {
                println!("Shutting down Compass Node...");
                break;
            }
            "6" => {
                print!("Collateral Asset (e.g. SOL): ");
                io::stdout().flush().unwrap();
                let mut col = String::new();
                io::stdin().read_line(&mut col).unwrap();
                
                print!("Compass Asset (e.g. Compass-SOL): ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).unwrap();

                print!("Vault Wallet Address (External Chain): ");
                io::stdout().flush().unwrap();
                let mut addr = String::new();
                io::stdin().read_line(&mut addr).unwrap();
                
                print!("Rate (Compass per 1 Collateral): ");
                io::stdout().flush().unwrap();
                let mut rate_str = String::new();
                io::stdin().read_line(&mut rate_str).unwrap();
                let rate: u64 = rate_str.trim().parse().unwrap_or(0);

                let mut vaults = vaults.lock().unwrap();
                match vaults.register_vault(col.trim(), name.trim(), addr.trim(), rate) {
                    Ok(_) => {
                        println!("Vault Registered!");
                        vaults.save("vaults.json");
                    },
                    Err(e) => println!("Error: {}", e),
                }
            }
            "7" => {
                // Admin Tool to Sign User Requests
                print!("Collateral Ticker (e.g. LTC): ");
                io::stdout().flush().unwrap();
                let mut t = String::new(); io::stdin().read_line(&mut t).unwrap();
                
                print!("Collateral Amount (Sats): ");
                io::stdout().flush().unwrap();
                let mut a = String::new(); io::stdin().read_line(&mut a).unwrap();
                let amt: u64 = a.trim().parse().unwrap_or(0);

                print!("User's Desired Mint Amount: ");
                io::stdout().flush().unwrap();
                let mut m = String::new(); io::stdin().read_line(&mut m).unwrap();
                let mint: u64 = m.trim().parse().unwrap_or(0);
                
                print!("TX Hash: ");
                io::stdout().flush().unwrap();
                let mut tx = String::new(); io::stdin().read_line(&mut tx).unwrap();
                
                print!("User Identity (OwnerID): ");
                io::stdout().flush().unwrap();
                let mut u = String::new(); io::stdin().read_line(&mut u).unwrap();
                
                let sig = admin.sign_hex(
                    format!("DEPOSIT:{}:{}:{}:{}:{}", t.trim(), amt, tx.trim(), mint, u.trim()).as_bytes()
                );
                
                println!("\n=== GENERATED SIGNATURE ===");
                println!("Signature: {}", sig);
                println!("Oracle PubKey: {}", admin_pubkey_hex);
                println!("(Send BOTH to user {})", u.trim());
            }
            _ => println!("Invalid option, try again."),
        }
    }
}