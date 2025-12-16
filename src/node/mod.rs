use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};
use sha2::Digest;

use crate::chain::Chain;
use crate::wallet::{WalletManager, WalletType};
use crate::vault::VaultManager;
use crate::market::Market;
use crate::gulf_stream::manager::CompassGulfStreamManager;
use crate::layer2::Layer2State;
use crate::oracle::OracleService;
use crate::crypto::KeyPair;
use crate::network::{NetMessage, NetworkCommand, PeerManager, TransactionPayload};
use crate::block::{self, BlockType};
use crate::storage::Storage;
pub mod oracle_scheduler;

pub struct CompassNode {
    pub chain: Arc<Mutex<Chain>>,
    pub wallets: Arc<Mutex<WalletManager>>,
    pub vaults: Arc<Mutex<VaultManager>>,
    pub market: Arc<Mutex<Market>>,
    pub gulf_stream: Arc<Mutex<CompassGulfStreamManager>>,
    pub layer2: Arc<Mutex<Layer2State>>,
    pub oracle: Arc<tokio::sync::Mutex<OracleService>>,
    
    // Networking
    pub peer_manager: Arc<Mutex<PeerManager>>,
    pub gossip_tx: tokio::sync::broadcast::Sender<(NetMessage, String)>,
    pub cmd_tx: mpsc::Sender<NetworkCommand>,
    pub cmd_rx: Arc<tokio::sync::Mutex<Option<mpsc::Receiver<NetworkCommand>>>>, // Wrapped for async access if needed
    
    pub p2p_port: u16,
    pub identity: Arc<KeyPair>,
    pub local_libp2p_key: libp2p::identity::Keypair,
    pub db_path: String,
    pub config: crate::config::CompassConfig,
    pub betting_ledger: Arc<Mutex<crate::layer3::betting::BettingLedger>>,
}

impl CompassNode {
    pub async fn new(
        config: crate::config::CompassConfig,
        explicit_identity: Option<Arc<KeyPair>>
    ) -> Self {
    println!("Starting Compass Node...");
    println!("CWD: {:?}", std::env::current_dir().unwrap());
    let p2p_port = config.node.p2p_port;
    let db_path = config.node.db_path.clone();
    
    // Setup Identity
    let admin = if let Some(k) = explicit_identity {
        println!("IDENTITY INJECTION: Using injected identity.");
        k
    } else if std::path::Path::new("admin.json").exists() {
         let cwd = std::env::current_dir().unwrap();
         println!("IDENTITY FOUND: 'admin.json' at {:?}", cwd.join("admin.json"));
         print!("Enter password to unlock Admin Node: ");
         std::io::Write::flush(&mut std::io::stdout()).unwrap();
         let mut pass = String::new();
         std::io::stdin().read_line(&mut pass).unwrap();
         Arc::new(match crate::identity::Identity::load_and_decrypt(std::path::Path::new("admin.json"), pass.trim()) {
             Ok(id) => {
                 println!("IDENTITY UNLOCKED: '{}'", id.name);
                 // Create valid backup
                 let _ = std::fs::copy("admin.json", "admin.backup.json");
                 id.into_keypair().expect("Identity locked")
             },
             Err(e) => {
                 println!("IDENTITY ERROR: Failed to unlock: {}", e);
                 if e.to_string().contains("missing field") {
                     println!("⚠️  DETECTED CORRUPTION: Attempting to restore from backup...");
                     match crate::identity::Identity::load_and_decrypt(std::path::Path::new("admin.backup.json"), pass.trim()) {
                         Ok(id_bak) => {
                             println!("✅ BACKUP RESTORED: '{}'", id_bak.name);
                             let _ = std::fs::copy("admin.backup.json", "admin.json");
                             id_bak.into_keypair().expect("Identity locked")
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
         })
        } else if std::path::Path::new("admin_key.mnemonic").exists() {
            info!("Loading Legacy Admin Key from 'admin_key.mnemonic'");
            let phrase = std::fs::read_to_string("admin_key.mnemonic").unwrap();
            Arc::new(KeyPair::from_mnemonic(phrase.trim()).expect("Invalid mnemonic"))
        } else {
            warn!("No Admin Identity found. Generating Temporary Key (NOT PERSISTED).");
            Arc::new(KeyPair::generate())
        };
        
        info!("IDENTITY: Node Public Key: {}", admin.public_key_hex());

        // --- 1. Storage & Persistence (Initialized First) ---
        info!("Persistence: Opening Sled DB at '{}'...", db_path);
        let storage = Storage::new(&db_path).expect("Failed to open DB");
        let storage_arc = Arc::new(storage); // Wrap for sharing

        // --- 2. Wallets (Migrated to Sled) ---
        // Initialize WalletManager with DB backing
        let mut wallet_manager = WalletManager::new_with_storage(storage_arc.clone());
        
        // MIGRATION: If DB is empty but wallets.json exists, migrate data
        if wallet_manager.wallets.is_empty() && std::path::Path::new("wallets.json").exists() {
             info!("Persistence: ⚠️ Migrating 'wallets.json' to Sled DB...");
             let old_wm = WalletManager::load("wallets.json");
             for (owner, w) in old_wm.wallets {
                 wallet_manager.wallets.insert(owner, w);
             }
             // Save to DB
             let _ = wallet_manager.save(""); 
             info!("Persistence: ✅ Migration Complete.");
        }

        // Ensure "Daniel" exists
        if wallet_manager.get_wallet("Daniel").is_none() {
            let daniel_w = crate::wallet::Wallet::new("Daniel", WalletType::Admin);
            wallet_manager.create_wallet(&daniel_w, "Daniel", WalletType::Admin);
            info!("Created 'Daniel' wallet.");
        }
        // Ensure "admin" exists
        if wallet_manager.get_wallet("admin").is_none() {
             let admin_w = crate::wallet::Wallet::new("admin", WalletType::Admin);
             wallet_manager.create_wallet(&admin_w, "admin", WalletType::Admin);
             info!("System: Created 'admin' wallet.");
        }
        
        let wallets = Arc::new(Mutex::new(wallet_manager));

        // --- Load Other Components (Still JSON for now) ---
        // --- 3. Vaults (Migrated to Sled) ---
        let mut vault_manager = VaultManager::new_with_storage(storage_arc.clone());
        if vault_manager.vaults.is_empty() && std::path::Path::new("vaults.json").exists() {
             info!("Persistence: ⚠️ Migrating 'vaults.json' to Sled DB...");
             let old_vm = VaultManager::load("vaults.json");
             for (k, v) in old_vm.vaults { vault_manager.vaults.insert(k, v); }
             for d in old_vm.processed_deposits { vault_manager.processed_deposits.insert(d); }
             for (k, v) in old_vm.oracle_prices { vault_manager.oracle_prices.insert(k, v); }
             let _ = vault_manager.save(""); 
             info!("Persistence: ✅ Vault Migration Complete.");
        }
        let vaults = Arc::new(Mutex::new(vault_manager));
        // --- Market (Migrated to Sled) ---
        let mut market_struct = Market::new_with_storage(storage_arc.clone());
        if market_struct.books.is_empty() && std::path::Path::new("market.json").exists() {
             info!("Persistence: ⚠️ Migrating 'market.json' to Sled DB...");
             if let Ok(old_m) = std::fs::read_to_string("market.json").and_then(|s| Ok(serde_json::from_str::<Market>(&s).unwrap_or(Market::new()))) {
                 for (k, v) in old_m.books { market_struct.books.insert(k, v); }
                 market_struct.next_order_id = old_m.next_order_id;
                 let _ = market_struct.save("");
                 info!("Persistence: ✅ Market Migration Complete.");
             }
        }
        let market = Arc::new(Mutex::new(market_struct));
        let gulf_stream = Arc::new(Mutex::new(CompassGulfStreamManager::new("Node1".to_string(), 1000)));

        // --- Chain & Layer 2 (Dependent on Storage) ---
        let chain = Arc::new(Mutex::new(Chain::new(storage_arc.clone())));
        
        // Validating Layer 2
        let layer2 = Arc::new(Mutex::new(Layer2State::new(Some(storage_arc.clone()))));
        
        // Genesis Init - ONLY if blockchain is empty
        {
            let mut c = chain.lock().unwrap();
            if c.height == 0 && c.head_hash.is_none() {
                // Fresh blockchain - initialize genesis
                if let Ok(config) = crate::genesis::GenesisConfig::load("genesis.json") {
                    match c.initialize_genesis(&config) {
                        Ok(_) => info!("✅ Genesis block initialized"),
                        Err(e) => warn!("Genesis initialization failed: {}", e),
                    }
                } else {
                    warn!("❌ genesis.json not found and blockchain is empty!");
                    warn!("   Create genesis.json or let node generate it automatically.");
                }
            } else {
                info!("⏭️  Skipping genesis init - blockchain already exists (height: {})", c.height);
            }
        }
        
        // --- Network Setup ---
        let peer_manager = Arc::new(Mutex::new(PeerManager::new(p2p_port)));
        let (gossip_tx, _gossip_rx) = tokio::sync::broadcast::channel(100); // We clone rx where needed
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let local_libp2p_key = libp2p::identity::Keypair::generate_ed25519();

        // --- Oracle Init ---
        let oracle_config = crate::oracle::OracleConfig::default();
        let oracle_keypair = wallets.lock().unwrap()
            .get_wallet("Daniel")
            .and_then(|w| w.get_keypair())
            .expect("Oracle wallet (Daniel) not found");
            
        let oracle = Arc::new(tokio::sync::Mutex::new(OracleService::new(
            oracle_config,
            oracle_keypair,
            Arc::clone(&layer2),
        )));

        // --- Betting (Migrated to Sled) ---
        let mut betting_ledger_struct = crate::layer3::betting::BettingLedger::new_with_storage(storage_arc.clone());
        if betting_ledger_struct.settled_bets.is_empty() && std::path::Path::new("betting.json").exists() {
             info!("Persistence: ⚠️ Migrating 'betting.json' to Sled DB...");
             if let Ok(mut old_bl) = crate::layer3::betting::BettingLedger::load("betting.json") {
                 old_bl.storage = Some(storage_arc.clone());
                 let _ = old_bl.save(""); // Persist to Sled
                 betting_ledger_struct = old_bl;
                 info!("Persistence: ✅ Betting Ledger Migration Complete.");
             }
        }
        let betting_ledger = Arc::new(Mutex::new(betting_ledger_struct));

        Self {
            chain,
            wallets,
            vaults,
            market,
            gulf_stream,
            layer2,
            oracle,
            peer_manager,
            gossip_tx,
            cmd_tx,
            cmd_rx: Arc::new(tokio::sync::Mutex::new(Some(cmd_rx))),

            p2p_port,
            identity: admin,
            local_libp2p_key,
            db_path,
            config: config.clone(),
            betting_ledger,
        }
    }

    pub async fn start(self, rpc_port_val: Option<u16>, peer_val: Option<String>) {
        info!("Starting Compass Node Services...");
        
        let rpc_port = rpc_port_val.unwrap_or(9000);
        let peer_addr = peer_val.clone(); 
        let follower_mode = peer_addr.is_some();

        // 1. P2P Server
        let pm_clone = self.peer_manager.clone();
        let gtx_clone = self.gossip_tx.clone();
        let chain_p2p = self.chain.clone();
        
        
        let genesis_hash = {
             let chain = self.chain.lock().unwrap();
             // Cleanup for Test (User Request): Delete old 24h jobs
             let all_jobs = chain.storage.get_all_recurring_jobs();
             for job in all_jobs {
                 if job.ticker == "FINANCE_ML_V1" {
                     info!("Test Cleanup: Deleting old job {}", job.job_id);
                     let _ = chain.storage.delete_recurring_job(&job.job_id);
                 }
             }
             
             // Job creation will happen in background task below
             info!("🤖 Neural Network Job System will initialize...");
             
             chain.get_genesis_hash().unwrap_or_default()
        };
        
        // Start background task for NN job creation
        if !follower_mode {
            let chain_clone = self.chain.clone();
            let admin_pubkey = self.identity.public_key_hex();
            let network_cmd_tx = self.cmd_tx.clone();
            
            // Start Oracle Scheduler (Real AI Price Oracle)
            let chain_oracle = self.chain.clone();
            let admin_pubkey_oracle = self.identity.public_key_hex();
            
            tokio::spawn(async move {
                use crate::node::oracle_scheduler::OracleScheduler;
                let scheduler = OracleScheduler::new(chain_oracle, admin_pubkey_oracle, network_cmd_tx);
                scheduler.start().await;
            });
        }
        
        let my_gen = genesis_hash.clone();
        let server_key = self.local_libp2p_key.clone();
        let cmd_rx_opt = self.cmd_rx.lock().await.take(); // Take the receiver
        let p2p_port = self.p2p_port;
        let chain_sync = self.chain.clone();
        let cmd_tx_sync = self.cmd_tx.clone();
        
        if let Some(rx) = cmd_rx_opt {
             tokio::spawn(async move {
                crate::network::start_server(p2p_port, pm_clone, gtx_clone, chain_p2p, my_gen, rx, server_key).await;
            });
        }
        
        // P2P Dialing (CLI Peer + Bootnodes)
        let bootnodes = self.config.node.bootnodes.clone();
        if let Some(paddr) = peer_val {
            let tx = self.cmd_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let _ = tx.send(NetworkCommand::Dial(paddr)).await;
            });
        }
        
        if !bootnodes.is_empty() {
             let tx = self.cmd_tx.clone();
             tokio::spawn(async move {
                 tokio::time::sleep(Duration::from_secs(3)).await; // Wait for server to bind
                 for node in bootnodes {
                     info!("🌐 Bootstrapping: Dialing bootnode {}", node);
                     let _ = tx.send(NetworkCommand::Dial(node)).await;
                     tokio::time::sleep(Duration::from_millis(500)).await;
                 }
             });
        }
        
        // Sync Task
        let mut gossip_rx = self.gossip_tx.subscribe();
        let gs_p2p = self.gulf_stream.clone();
        let chain_sync_task = self.chain.clone(); // For logic inside sync
        
        tokio::spawn(async move {
            while let Ok((msg, peer_source)) = gossip_rx.recv().await {
                 match msg {
                    NetMessage::SubmitTx(payload) => {
                        if let Ok(raw_tx) = bincode::serialize(&payload) {
                            let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();
                            gs_p2p.lock().unwrap().add_transaction(tx_hash, raw_tx, 0);
                        }
                    }
                    NetMessage::HeightResponse { height: remote_height } => {
                         let local_height = chain_sync_task.lock().unwrap().height;
                         if remote_height > local_height {
                             let start = local_height + 1;
                             let mut end = remote_height;
                             if end - start > 50 { end = start + 50; }
                             let req = NetMessage::RequestBlocks { start, end };
                             let _ = cmd_tx_sync.send(NetworkCommand::SendRequest { peer: peer_source.clone(), req }).await;
                         }
                    }
                    _ => {}
                }
            }
        });

        // 2. Oracle Betting Loop
        let oracle_loop = self.oracle.clone();
        tokio::spawn(async move {
            println!("🤖 Oracle Betting Bridge Started.");
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let mut o = oracle_loop.lock().await;
                o.process_betting_outcomes().await;
            }
        });

        // 3. RPC Server
        let rpc_chain = self.chain.clone();
        let rpc_pm = self.peer_manager.clone();
        let rpc_gs = self.gulf_stream.clone();
        let rpc_vaults = self.vaults.clone();
        let rpc_layer2 = self.layer2.clone();
        let rpc_betting = self.betting_ledger.clone(); // Pass betting ledger
        let rpc_market = self.market.clone();
        let rpc_wallets = self.wallets.clone();
        let rpc_cmd_tx = self.cmd_tx.clone();
        
        let rpc_identity = self.identity.public_key_hex();
        
        tokio::spawn(async move {
            let server = crate::rpc::RpcServer::new(rpc_chain, rpc_pm, rpc_gs, rpc_vaults, rpc_wallets, rpc_layer2, rpc_betting, rpc_market, rpc_cmd_tx, rpc_port, rpc_identity);
            server.start().await;
        });

        // 4. Transaction Processor
        let gulf_stream = self.gulf_stream.clone();
        let wallets = self.wallets.clone();
        let market = self.market.clone();
        let chain = self.chain.clone();
        let layer2 = self.layer2.clone(); // For NFT usage
        
        tokio::spawn(async move {
            loop {
                let mut txs_to_process = Vec::new();
                {
                    let mut gs = gulf_stream.lock().unwrap();
                    let popped = gs.pop_ready_transactions(5000);
                    for tx in popped { txs_to_process.push(tx); }
                }

                if !txs_to_process.is_empty() {
                    let m_guard = market.lock().unwrap();
                    let mut c_guard = chain.lock().unwrap();
                    
                    for tx in txs_to_process {
                         if let Ok(payload) = bincode::deserialize::<TransactionPayload>(&tx.raw_tx) {
                             match payload {
                                 TransactionPayload::MintModelNFT(params) => {
                                     let mut l2 = layer2.lock().unwrap();
                                     let nft = crate::layer3::model_nft::ModelNFT {
                                         token_id: params.model_id.clone(),
                                         name: params.name,
                                         description: params.description,
                                         creator: params.creator.clone(),
                                         license: crate::layer3::model_nft::LicenseType::Commercial,
                                         rental_status: None,
                                         // Defaults...
                                         accuracy: 0.0, win_rate: 0.0, total_predictions: 0, profitable_predictions: 0, total_profit: 0,
                                         training_samples: 0, training_epochs: 0, final_loss: 0.0, training_duration_seconds: 0,
                                         trained_on_data_hash: "genesis".into(), weights_hash: "pending".into(), weights_uri: "pending".into(),
                                         architecture: "unknown".into(), parent_models: vec![], generation: 0, mint_price: 0,
                                         royalty_rate: 0.05, current_owner: params.creator.clone(), sale_history: vec![],
                                         minted_at: block::current_unix_timestamp_ms(), last_updated: block::current_unix_timestamp_ms(),
                                     };
                                     l2.assets.register_mint(nft.clone(), params.creator);
                                     let _ = l2.save("layer2.json"); 
                                     
                                     // Also save to verified Sled Storage
                                     if let Err(e) = c_guard.storage.save_model_nft(&nft) {
                                         tracing::error!("Failed to save Model NFT to Sled: {}", e);
                                     } else {
                                         println!("✅ Presisted NFT to Chain Storage: {}", params.model_id);
                                     }
                                     
                                     println!("✅ L2: Minted NFT {}", params.model_id);
                                 },
                                 TransactionPayload::Stake(params) => {
                                      let mut l2 = layer2.lock().unwrap();
                                      l2.collateral.stake(params.entity.clone(), params.amount);
                                      let _ = l2.save("layer2.json");
                                      println!("✅ L2: Staked {} for {}", params.amount, params.entity);
                                 },
                                 TransactionPayload::Result(params) => {
                                      // PoUW Logic
                                      let reward = if params.compute_rate > 0 { params.compute_rate / 1000 } else { 1 };
                                      let _ = c_guard.storage.update_balance(&params.worker_id, "COMPUTE", reward);
                                      println!("✅ L1: PoUW Reward {} COMPUTE", reward);
                                 },
                                  // .. other standard txs like Transfer ..
                                 TransactionPayload::Transfer { from, to, amount, nonce: _, signature, public_key, timestamp, prev_hash, .. } => {
                                      // Construct block... check sig... append
                                      // Minimal implementation
                                      if crate::crypto::verify_with_pubkey_hex(from.as_bytes(), &signature, &public_key) { 
                                           let header = crate::block::BlockHeader {
                                                index: c_guard.height,
                                                timestamp,
                                                prev_hash: prev_hash.clone(),
                                                hash: "".into(),
                                                proposer: from.clone(),
                                                signature_hex: signature.clone(),
                                                block_type: BlockType::Transfer { from: from.clone(), to: to.clone(), asset: "Compass".into(), amount, nonce: 0, fee: 0 },
                                           };
                                           let mut h = header;
                                           h.hash = h.calculate_hash().unwrap_or_default();
                                           let _ = c_guard.append_transfer(h, &public_key);
                                      }
                                 },
                                 _ => {}
                             }
                         }
                    }
                    m_guard.save("market.json");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        // 5. PoH Loop
        let admin_kp = self.identity.clone();
        let chain_poh_outer = self.chain.clone();
        // Skip PoH if follower
        if !follower_mode {
            let config_duration = self.config.consensus.slot_duration_ms;
            let target_duration = Duration::from_millis(config_duration);
            
            tokio::spawn(async move {
                use crate::poh_recorder::PoHRecorder;
                let mut poh = PoHRecorder::new(b"COMPASS_GENESIS_SEED".to_vec(), 80_000); // 80k iterations ~ VDF work
                
                info!("PoH Service Started. Target Slot Duration: {}ms", config_duration);
                info!("Initial VDF Difficulty: {} iterations/tick", poh.hashes_per_tick);

                loop {
                    let start = std::time::Instant::now();
                    let chain_poh = chain_poh_outer.clone();
                    let admin_kp_poh = admin_kp.clone();
                    
                    // Run VDF in blocking thread to avoid starvation
                    poh = tokio::task::spawn_blocking(move || {
                        let (start_hash, end_hash) = poh.tick(); // Runs CPU intensive Modular Squaring
                        
                        // Create Block
                        if let Ok(mut c_guard) = chain_poh.lock() {
                            let head_hash = c_guard.head_hash().unwrap_or("0000000000000000000000000000000000000000000000000000000000000000".to_string());
                            let height = c_guard.height;
                            
                            use crate::block::{BlockHeader, BlockType};
                            let header = BlockHeader {
                                index: height,
                                timestamp: crate::block::current_unix_timestamp_ms(),
                                prev_hash: head_hash,
                                hash: "".to_string(),
                                proposer: admin_kp_poh.public_key_hex(),
                                signature_hex: "".to_string(),
                                block_type: BlockType::PoH { 
                                    tick: poh.tick_height,
                                    iterations: poh.hashes_per_tick,
                                    hash: hex::encode(&end_hash),
                                    proof: "".to_string(), // Simplified
                                },
                            };
                            
                            let mut signed_header = header;
                            signed_header.hash = signed_header.calculate_hash().unwrap();
                            let sig = admin_kp_poh.sign(hex::decode(&signed_header.hash).unwrap().as_slice());
                            signed_header.signature_hex = sig.to_string();

                            // Append to Chain
                            let _ = c_guard.append_poh(signed_header, &admin_kp_poh.public_key_hex());
                        }
                        poh
                    }).await.expect("PoH Task failed");

                    let elapsed = start.elapsed();
                    let current_tick = poh.tick_height;
                    
                    if current_tick % 10 == 0 {
                        info!("PoH Tick {} | VDF Time: {:?} | Hash: {}", 
                            current_tick, elapsed, hex::encode(&poh.current_hash).get(0..16).unwrap_or(""));
                    }

                    // Dynamic Adjustment (Simple) or Sleep remainder
                    if elapsed < target_duration {
                        tokio::time::sleep(target_duration - elapsed).await;
                    }
                }
            });
        }
        
        // --- Graceful Shutdown Handler ---
        // Register Ctrl+C handler to flush database before exit
        let chain_flush = self.chain.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            info!("🛑 Shutting down... flushing database");
            if let Ok(c) = chain_flush.lock() {
                if let Err(e) = c.storage.flush() {
                    warn!("Failed to flush database: {}", e);
                } else {
                    info!("✅ Database flushed successfully");
                }
            }
            std::process::exit(0);
        });
        
        // 6. Auto-Trainer (Rust Native)
        // This runs the enhanced Linear Regression model loop natively in the node
        let trainer = crate::trainer::AutoTrainer::new();
        trainer.start().await;

        info!("Node Running. Press Ctrl+C to stop.");
        // Keep main alive
        loop { tokio::time::sleep(Duration::from_secs(60)).await; }
    }
}

// --- Helper for Node Startup (Exposed for Library Use) ---
pub async fn run_node_mode_internal(
    config: crate::config::CompassConfig,
    peer_val: Option<String>, 
    explicit_identity: Option<std::sync::Arc<crate::crypto::KeyPair>>
) {
    let rpc_port = config.node.rpc_port;
    let node = CompassNode::new(config, explicit_identity).await;
    node.start(Some(rpc_port), peer_val).await;
}
