use eframe::egui;
use rust_compass::client::RpcClient;
use rust_compass::oracle::monitor::OracleMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use rust_compass::wallet::{WalletManager, Wallet, WalletType};
use rust_compass::layer3::compute::ComputeJob;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Compass Desktop"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Compass Desktop",
        options,
        Box::new(|cc| {
            configure_theme(&cc.egui_ctx);
            Ok(Box::new(CompassApp::new(cc)))
        }),
    )
}

fn configure_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    
    // Deep Space Theme
    visuals.window_fill = egui::Color32::from_rgb(11, 15, 25); // #0b0f19
    visuals.panel_fill = egui::Color32::from_rgb(17, 22, 37);  // #111625
    
    // UI Elements
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(22, 27, 46); // Surface
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 35, 55);       // Button Normal
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 55, 80);        // Button Hover
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 130, 246);       // Button Active (Blue)
    
    // Text
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)); // Text
    
    // Selection
    visuals.selection.bg_fill = egui::Color32::from_rgb(60, 130, 246); // Accent Blue
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    // Rounding & Spacing
    visuals.window_rounding = egui::Rounding::same(12.0);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    
    ctx.set_visuals(visuals);
    
    // Font Support (increase size for premium feel)
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
        (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Proportional)),
    ].into();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    
    ctx.set_style(style);
}

#[derive(Debug, Clone, PartialEq)]
enum Page {
    Dashboard,
    Wallet,
    Workers,
    Oracle,
    Vaults,
    Marketplace,
    Train,
    Admin, // Phase 6 New
}

#[derive(Debug, Clone)]
struct NFTInfo {
    token_id: String,
    name: String,
    price: u64,
    owner: String,
    accuracy: f64,
}

#[derive(Debug, Clone)]
struct WalletInfo {
    balances: HashMap<String, u64>,
}

#[derive(Debug, Clone)]
struct JobInfo {
    job_id: String,
    status: String,
    reward: u64,
    progress: f32,
}

#[derive(Debug, Clone)]
struct BlockInfo {
    index: u64,
    hash: String,
    timestamp: u64,
}

struct CompassApp {
    // Authentication
    logged_in: bool,
    login_username: String,
    login_password: String,
    current_user: String,
    
    // Navigation
    current_page: Page,
    
    // Dashboard Data
    block_height: u64,
    peer_count: u64,
    recent_blocks: Vec<BlockInfo>,
    sync_status: String,
    
    // Wallet Data
    wallets: HashMap<String, WalletInfo>,
    selected_wallet: String,
    
    // Send Form
    send_to: String,
    send_amount: String,
    send_asset: String,
    
    // Vault Form
    vault_payment_asset: String,
    vault_tx_hash: String,
    vault_compass_collateral: String,
    vault_mint_amount: String,
    
    // Worker Data
    jobs: Vec<JobInfo>,
    total_rewards: u64,
    
    oracle_prices: HashMap<String, f64>,
    
    // Oracle Monitor
    oracle_monitor_running: bool,
    oracle_ltc_address: String,
    oracle_logs: Vec<String>,
    oracle_stop_tx: Option<mpsc::Sender<()>>,
    oracle_log_rx: Option<mpsc::Receiver<String>>,

    // Marketplace & Train
    marketplace_listings: Vec<NFTInfo>,
    my_models: Vec<NFTInfo>,
    train_model_id: String,
    train_dataset: String,
    train_auto_loop: bool,
    
    // In-App Worker State
    gui_worker_active: bool,
    gui_worker_status: String,
    gui_worker_progress: f32,
    gui_worker_logs: Vec<String>,
    gui_worker_tx: mpsc::Sender<bool>,
    
    // In-App Worker State

    

    
    // Communication
    rpc_tx: mpsc::Sender<RpcCommand>,
    response_rx: mpsc::Receiver<RpcResponse>,
    
    // UI State
    status_msg: String,
    error_msg: Option<String>,
    last_poll: f64,
    
    // Wallet Manager (Local)
    local_wallet_manager: WalletManager,
    showing_create_wallet: bool,
    new_wallet_name: String,
    created_mnemonic: Option<String>,
}

enum RpcCommand {
    GetStatus,
    GetBalance(String, String), // wallet_id, asset
    GetAllWallets,
    GetComputeJobs,
    GetRecentBlocks(u64), // count
    GetOraclePrices,
    SendTransaction { from: String, to: String, asset: String, amount: u64 },
    GetMarketplaceListings,
    GetMyModels(String),
    BuyNFT { token_id: String, buyer: String },
    SubmitCompute { model_id: String, dataset: String },
    SubmitNativeVault { payment_asset: String, payment_amount: u64, compass_collateral: u64, requested_mint_amount: u64, owner_id: String, tx_hash: String },
    SubmitRecurringJob { ticker: String, duration_hours: u32, interval_minutes: u32, reward_per_update: u64, submitter: String },
    PurchaseNeuralNet { owner: String, ticker: String },
}

enum RpcResponse {
    StatusUpdate { height: u64, peers: u64 },
    BalanceUpdate { wallet: String, asset: String, balance: u64 },
    WalletsUpdate { wallets: Vec<String> },
    JobsUpdate { jobs: Vec<JobInfo> },
    BlocksUpdate { blocks: Vec<BlockInfo> },
    OraclePricesUpdate { prices: HashMap<String, f64> },
    TransactionSent { tx_hash: String },
    MarketplaceUpdate { listings: Vec<NFTInfo> },
    MyModelsUpdate { models: Vec<NFTInfo> },

    WorkerUpdate { status: String, progress: f32, log: Option<String>, reward: Option<u64> }, // New for GUI Worker
    Error(String),
}

impl CompassApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (rpc_tx, mut rpc_rx) = mpsc::channel(32);
        let (response_tx, response_rx) = mpsc::channel(32);
        
        // Spawn In-App Worker Thread
        let (worker_tx, mut worker_rx) = mpsc::channel(32);
        let response_tx_clone = response_tx.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            rt.block_on(async move {
                let client = RpcClient::new("http://localhost:9000/".to_string());
                let mut active = false;
                
                loop {
                    if let Ok(should_run) = worker_rx.try_recv() {
                        active = should_run;
                        let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                            status: if active { "Searching for jobs..." } else { "Paused" }.to_string(), 
                            progress: 0.0,
                            log: Some(if active { "✅ Worker Started" } else { "⏸ Worker Paused" }.to_string()),
                            reward: None
                        }).await;
                    }

                    if active {
                        match client.get_pending_compute_jobs(None).await {
                             Ok(jobs) => {
                                 if let Some(job) = jobs.first() {
                                     let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                         status: format!("Training Job: {}", job.job_id), 
                                         progress: 0.0,
                                         log: Some(format!("🚀 Accepted Job: {}", job.job_id)),
                                         reward: None
                                     }).await;
                                     
                                         // Convert PendingJob (RPC) to ComputeJob (Logic)
                                         let compute_job = ComputeJob::new(
                                             job.job_id.clone(),
                                             job.creator.clone(),
                                             job.model_id.clone(),
                                             vec![], // TODO: Fetch from IPFS/RPC using job.inputs
                                             job.max_compute_units,
                                         );
                                         
                                         let mut final_hash = String::new();

                                         // Execute Inference (ONNX)
                                         let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                             status: format!("Running Inference: {}", job.model_id), 
                                             progress: 0.5,
                                             log: Some("🧠 Loading ONNX Model & Running...".to_string()),
                                             reward: None
                                         }).await;

                                         match compute_job.execute_inference() {
                                             Ok(hash) => {
                                                 final_hash = hash;
                                                 let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                     status: "Inference Complete".to_string(), 
                                                     progress: 1.0,
                                                     log: Some(format!("✅ Result Hash: {}", final_hash)),
                                                     reward: None
                                                 }).await;
                                             }
                                             Err(e) => {
                                                 // Fallback to dummy task if model missing (for prototype flow)
                                                 let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                     status: "Fallback Mode".to_string(), 
                                                     progress: 0.5,
                                                     log: Some(format!("⚠️ Inference Failed: {}. Running backup task...", e)),
                                                     reward: None
                                                 }).await;
                                                 final_hash = compute_job.execute_deterministic_task(1000).unwrap_or_default();
                                             }
                                         }
                                         tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                     
                                     let req = serde_json::json!({
                                         "job_id": job.job_id,
                                         "worker_id": "GUI_WORKER",
                                         "result_data": final_hash.as_bytes().to_vec(),
                                         "signature": "gui_sig",
                                         "compute_rate": 5000,
                                         "compute_units_used": 100
                                     });
                                     
                                     match client.call_method::<serde_json::Value, serde_json::Value>("submitResult", req).await {
                                         Ok(_) => {
                                             let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                 status: "Searching for jobs...".to_string(), 
                                                 progress: 0.0,
                                                 log: Some("✅ Result Submitted! Earned COMPUTE.".to_string()),
                                                 reward: Some(100)
                                             }).await;
                                         }
                                         Err(e) => {
                                              let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                 status: "Error".to_string(), 
                                                 progress: 0.0,
                                                 log: Some(format!("❌ Submit Failed: {}", e)),
                                                 reward: None
                                              }).await;
                                         }
                                     }
                                 }
                             }
                             Err(_) => {}
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            });
        });

        // Spawn Background RPC Handler
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
                
            rt.block_on(async move {
                println!("🔗 RPC Client connecting to http://localhost:9000/");
                let client = RpcClient::new("http://localhost:9000/".to_string());
                
                while let Some(cmd) = rpc_rx.recv().await {
                    match cmd {
                        RpcCommand::GetStatus => {
                            match client.get_node_info().await {
                                Ok(info) => {
                                    let height = info.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let peers = info.get("peer_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let _ = response_tx.send(RpcResponse::StatusUpdate { height, peers }).await;
                                }
                                Err(e) => { 
                                    let _ = response_tx.send(RpcResponse::Error(format!("Node Status: {}", e))).await; 
                                }
                            }
                        }
                        RpcCommand::GetBalance(wallet, asset) => {
                            match client.get_balance(&wallet, &asset).await {
                                Ok(bal) => {
                                    let _ = response_tx.send(RpcResponse::BalanceUpdate { 
                                        wallet: wallet.clone(), 
                                        asset: asset.clone(), 
                                        balance: bal 
                                    }).await;
                                }
                                Err(_) => {} // Silent fail for balance
                            }
                        }
                        RpcCommand::GetAllWallets => {
                            // For now, hardcode known wallets
                            let wallets = vec!["admin".to_string(), "Daniel".to_string()];
                            let _ = response_tx.send(RpcResponse::WalletsUpdate { wallets }).await;
                        }
                        RpcCommand::GetComputeJobs => {
                            match client.get_pending_compute_jobs(None).await {
                                Ok(pending_jobs) => {
                                    let job_infos: Vec<JobInfo> = pending_jobs.iter().map(|j| JobInfo {
                                        job_id: j.job_id.clone(),
                                        status: "Pending".to_string(),  // PendingJob is always pending
                                        reward: j.max_compute_units / 100,  // Estimate from compute units
                                        progress: 0.0,
                                    }).collect();
                                    let _ = response_tx.send(RpcResponse::JobsUpdate { jobs: job_infos }).await;
                                }
                                Err(_) => {} // Silent fail
                            }
                        }
                        RpcCommand::GetRecentBlocks(count) => {
                            match client.get_block_range(None, Some(count)).await {
                                Ok(blocks) => {
                                    let block_infos: Vec<BlockInfo> = blocks.iter().map(|b| BlockInfo {
                                        index: b.header.index,
                                        hash: b.header.hash.clone(),
                                        timestamp: b.header.timestamp,
                                    }).collect();
                                    let _ = response_tx.send(RpcResponse::BlocksUpdate { blocks: block_infos }).await;
                                }
                                Err(_) => {} // Silent fail
                            }
                        }
                        RpcCommand::GetOraclePrices => {
                            match client.get_oracle_prices().await {
                                Ok(prices) => {
                                    let _ = response_tx.send(RpcResponse::OraclePricesUpdate { prices }).await;
                                }
                                Err(_) => {} // Silent fail
                            }
                        }
                        RpcCommand::SendTransaction { from: _, to: _, asset: _, amount: _ } => {
                            // Stub: would call client.submit_transaction
                            let _ = response_tx.send(RpcResponse::Error("Send not implemented yet".to_string())).await;
                        }
                        RpcCommand::GetMarketplaceListings => {
                             // For MVP, get ALL NFTs. In prod, call getActiveListings
                             match client.call_method::<serde_json::Value, Vec<rust_compass::layer3::model_nft::ModelNFT>>("getAllNFTs", serde_json::json!({})).await {
                                 Ok(nfts) => {
                                     let infos = nfts.into_iter().map(|n| NFTInfo {
                                         token_id: n.token_id,
                                         name: n.name,
                                         price: n.mint_price, // Fallback to mint price if listing not found
                                         owner: n.current_owner,
                                         accuracy: n.accuracy,
                                     }).collect();
                                     let _ = response_tx.send(RpcResponse::MarketplaceUpdate { listings: infos }).await;
                                 }
                                 Err(_) => {}
                             }
                        }
                        RpcCommand::GetMyModels(owner) => {
                             match client.call_method::<serde_json::Value, Vec<rust_compass::layer3::model_nft::ModelNFT>>("getAllNFTs", serde_json::json!({})).await {
                                 Ok(nfts) => {
                                     let my_nfts = nfts.into_iter()
                                         .filter(|n| n.current_owner == owner)
                                         .map(|n| NFTInfo {
                                            token_id: n.token_id,
                                            name: n.name,
                                            price: n.mint_price,
                                            owner: n.current_owner,
                                            accuracy: n.accuracy,
                                         }).collect();
                                     let _ = response_tx.send(RpcResponse::MyModelsUpdate { models: my_nfts }).await;
                                 }
                                 Err(_) => {}
                             }
                        }
                        RpcCommand::BuyNFT { token_id, buyer } => {
                            let req = serde_json::json!({ "token_id": token_id, "buyer": buyer });
                            match client.call_method::<serde_json::Value, serde_json::Value>("buyModelNFT", req).await {
                                Ok(_) => {
                                     let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Purchase Successful".to_string() }).await;
                                }
                                Err(e) => {
                                     let _ = response_tx.send(RpcResponse::Error(format!("Buy Failed: {}", e))).await;
                                }
                            }
                        }
                        RpcCommand::SubmitCompute { model_id, dataset } => {
                            let req = serde_json::json!({ 
                                "job_id": uuid::Uuid::new_v4().to_string(),
                                "model_id": model_id,
                                "inputs": dataset.as_bytes().to_vec(),
                                "max_compute_units": 100,
                                "signature": "dummy_sig",
                                "owner_id": "user",
                                "bid_amount": 0,          // Added for Phase 2 compatibility
                                "bid_asset": "COMPASS"    // Added for Phase 2 compatibility
                            });
                            match client.call_method::<serde_json::Value, serde_json::Value>("submitCompute", req).await {
                                Ok(res) => {
                                     let hash = res.get("tx_hash").and_then(|h| h.as_str()).unwrap_or("submitted");
                                     let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: hash.to_string() }).await;
                                }
                                Err(e) => {
                                     let _ = response_tx.send(RpcResponse::Error(format!("Compute Failed: {}", e))).await;
                                }
                            }
                        }
                        RpcCommand::SubmitNativeVault { payment_asset, payment_amount, compass_collateral, requested_mint_amount, owner_id, tx_hash } => {
                             let req = serde_json::json!({
                                 "payment_asset": payment_asset,
                                 "payment_amount": payment_amount,
                                 "compass_collateral": compass_collateral,
                                 "requested_mint_amount": requested_mint_amount,
                                 "owner_id": owner_id,
                                 "tx_hash": tx_hash,
                                 "oracle_signature": "pending_oracle_sig"
                             });
                             match client.call_method::<serde_json::Value, serde_json::Value>("submitNativeVault", req).await {
                                 Ok(_) => { let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Vault Created".to_string() }).await; }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Vault Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::SubmitRecurringJob { ticker, duration_hours, interval_minutes, reward_per_update, submitter } => {
                             let req = serde_json::json!({
                                 "ticker": ticker,
                                 "duration_hours": duration_hours,
                                 "interval_minutes": interval_minutes,
                                 "reward_per_update": reward_per_update,
                                 "submitter": submitter,
                                 "signature": "admin_gui_sig" 
                             });
                             match client.call_method::<serde_json::Value, serde_json::Value>("submitRecurringJob", req).await {
                                 Ok(_) => { let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Job Created".to_string() }).await; }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Job Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::PurchaseNeuralNet { owner, ticker } => {
                             let req = serde_json::json!({ "owner": owner, "ticker": ticker });
                             match client.call_method::<serde_json::Value, serde_json::Value>("purchaseNeuralNet", req).await {
                                 Ok(_) => { let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Model Minted".to_string() }).await; }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Mint Failed: {}", e))).await; }
                             }
                        }
                    }
                }
            });
        });
        
        let mut wallets = HashMap::new();
        wallets.insert("admin".to_string(), WalletInfo { balances: HashMap::new() });
        
        Self {
            logged_in: false,
            login_username: String::new(),
            login_password: String::new(),
            current_user: String::new(),
            current_page: Page::Dashboard,
            block_height: 0,
            peer_count: 0,
            recent_blocks: Vec::new(),
            sync_status: "Connecting...".to_string(),
            wallets,
            selected_wallet: "admin".to_string(),
            send_to: String::new(),
            send_amount: String::new(),
            send_asset: "Compass".to_string(),
            vault_payment_asset: "LTC".to_string(),
            vault_tx_hash: String::new(),
            vault_compass_collateral: String::new(),
            vault_mint_amount: String::new(),
            jobs: Vec::new(),
            total_rewards: 0,
            oracle_prices: HashMap::new(),
            oracle_monitor_running: false,
            oracle_ltc_address: std::env::var("LTC_ADMIN_ADDRESS").unwrap_or_else(|_| "LTC_EXAMPLE_ADDR".to_string()), 
            oracle_logs: Vec::new(),
            oracle_stop_tx: None,
            oracle_log_rx: None,
            rpc_tx,
            response_rx,
            status_msg: "Connecting...".to_string(),
            error_msg: None,
            last_poll: 0.0,
            marketplace_listings: Vec::new(),
            my_models: Vec::new(),
            train_model_id: String::new(),
            train_dataset: "ipfs://mnist-sample-v1".to_string(),
            train_auto_loop: false,
            
            gui_worker_active: false,
            gui_worker_status: "Paused".to_string(),
            gui_worker_progress: 0.0,
            gui_worker_logs: Vec::new(),
            gui_worker_tx: worker_tx,
            

            

            
            local_wallet_manager: WalletManager::load("wallets.json"),
            showing_create_wallet: false,
            new_wallet_name: String::new(),
            created_mnemonic: None,
        }
    }
    
    fn poll_data(&mut self, time: f64) {
        if time - self.last_poll < 2.0 {
            return;
        }
        self.last_poll = time;
        
        // Poll based on current page
        let _ = self.rpc_tx.try_send(RpcCommand::GetStatus);
        
        match self.current_page {
            Page::Dashboard => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetRecentBlocks(5));
            }
            Page::Wallet => {
                for wallet in self.wallets.keys() {
                    let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(wallet.clone(), "Compass".to_string()));
                    let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(wallet.clone(), "COMPUTE".to_string()));
                }
            }
            Page::Workers => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetComputeJobs);
            }
            Page::Oracle => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetOraclePrices);
            }
            Page::Vaults => {
                // No polling needed for vaults page (form-based)
            }
            Page::Marketplace => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetMarketplaceListings);
            }
            Page::Train => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetMyModels(self.current_user.clone()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetMarketplaceListings);
                
                // Auto-Loop Logic
                if self.train_auto_loop && !self.train_model_id.is_empty() {
                    let _ = self.rpc_tx.try_send(RpcCommand::SubmitCompute {
                        model_id: self.train_model_id.clone(),
                        dataset: self.train_dataset.clone(),
                    });
                }
            }
            Page::Admin => {}
        }
    }
}

impl eframe::App for CompassApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll responses
        while let Ok(msg) = self.response_rx.try_recv() {
            match msg {
                RpcResponse::StatusUpdate { height, peers } => {
                    self.block_height = height;
                    self.peer_count = peers;
                    self.sync_status = "Online".to_string();
                    self.status_msg = "Online".to_string();
                }
                RpcResponse::BalanceUpdate { wallet, asset, balance } => {
                    if let Some(info) = self.wallets.get_mut(&wallet) {
                        info.balances.insert(asset, balance);
                    }
                }
                RpcResponse::WalletsUpdate { wallets } => {
                     for w in wallets {
                        self.wallets.entry(w).or_insert(WalletInfo { balances: HashMap::new() });
                     }
                }
                RpcResponse::JobsUpdate { jobs } => {
                    self.jobs = jobs;
                }
                RpcResponse::BlocksUpdate { blocks } => {
                    self.recent_blocks = blocks;
                }
                RpcResponse::OraclePricesUpdate { prices } => {
                    self.oracle_prices = prices;
                }
                RpcResponse::TransactionSent { tx_hash } => {
                    self.error_msg = Some(format!("✅ Transaction sent: {}", &tx_hash[..16]));
                }
                RpcResponse::Error(e) => {
                    self.error_msg = Some(format!("Error: {}", e));
                }
                RpcResponse::MarketplaceUpdate { listings } => {
                    self.marketplace_listings = listings;
                }
                RpcResponse::MyModelsUpdate { models } => {
                    self.my_models = models;
                }
                RpcResponse::WorkerUpdate { status, progress, log, reward } => {
                     self.gui_worker_status = status;
                     self.gui_worker_progress = progress;
                     if let Some(l) = log {
                         self.gui_worker_logs.insert(0, l);
                         if self.gui_worker_logs.len() > 50 { self.gui_worker_logs.pop(); }
                     }
                     if let Some(r) = reward {
                         self.total_rewards += r;
                     }
                }
                _ => {}
            }
        }
        
        // Poll oracle logs
        if let Some(rx) = &mut self.oracle_log_rx {
             while let Ok(msg) = rx.try_recv() {
                 self.oracle_logs.push(msg);
                 if self.oracle_logs.len() > 100 {
                     self.oracle_logs.remove(0);
                 }
             }
        }
        
        // Poll status
        let time = ctx.input(|i| i.time);
        self.poll_data(time);
        
        // Request repaint
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        
        // Show login screen if not logged in
        if !self.logged_in {
            self.render_login(ctx);
            return;
        }
        
        // Sidebar
        egui::SidePanel::left("sidebar")
            .min_width(240.0)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(20.0);
                
                // Brand Header
                ui.horizontal(|ui| {
                    ui.add_space(15.0);
                    ui.label(egui::RichText::new("⚡").size(28.0));
                    ui.label(egui::RichText::new("Compass").size(24.0).strong());
                });
                
                ui.add_space(40.0);
                
                // Content
                ui.vertical(|ui| {
                    self.drawer_item(ui, Page::Dashboard, "📊", "Dashboard");
                    self.drawer_item(ui, Page::Wallet, "💰", "Wallet");
                    self.drawer_item(ui, Page::Workers, "⚙️", "Workers");
                    self.drawer_item(ui, Page::Oracle, "🔮", "Oracle");
                    self.drawer_item(ui, Page::Vaults, "🏦", "Vaults");
                    self.drawer_item(ui, Page::Marketplace, "🛒", "Marketplace");
                    self.drawer_item(ui, Page::Train, "🧠", "Train Model");
                    
                     if self.current_user == "admin" {
                        self.drawer_item(ui, Page::Admin, "🛑", "Admin Panel");
                    }
                });
                
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(20.0);
                    
                    // Profile Section
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(22, 27, 46))
                        .rounding(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("👤").size(20.0));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&self.current_user).strong());
                                    ui.label(egui::RichText::new(format!("{} Peers", self.peer_count)).size(10.0).color(egui::Color32::GRAY));
                                });
                            });
                        });
                });
            });
        
        // Main Panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Error banner
            if let Some(err) = self.error_msg.clone() {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(60, 20, 20))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&err).color(egui::Color32::from_rgb(255, 180, 180)));
                            if ui.button("✖").clicked() {
                                self.error_msg = None;
                            }
                        });
                    });
                ui.add_space(10.0);
            }
            
            // Page content
            match self.current_page {
                Page::Dashboard => self.render_dashboard(ui),
                Page::Wallet => self.render_wallet(ui),
                Page::Workers => self.render_workers(ui),
                Page::Oracle => self.render_oracle(ui),
                Page::Vaults => self.render_vaults(ui),
                Page::Marketplace => self.render_marketplace(ui),
                Page::Train => self.render_train(ui),
                Page::Admin => self.render_admin(ui),
            }
        });
    }
}

// Page Renderers
impl CompassApp {
    // Custom Drawer Item
    fn drawer_item(&mut self, ui: &mut egui::Ui, page: Page, icon: &str, label: &str) {
        let selected = self.current_page == page;
        
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 40.0), 
            egui::Sense::click()
        );
        
        if response.clicked() {
            self.current_page = page;
        }
        
        // Hover/Selection Visuals
        let visuals = ui.style().interact(&response);
        let bg_color = if selected {
            egui::Color32::from_rgb(37, 99, 235) // Accent Blue
        } else if response.hovered() {
            egui::Color32::from_rgb(28, 35, 55)  // Hover
        } else {
            egui::Color32::TRANSPARENT
        };
        
        ui.painter().rect_filled(rect, 8.0, bg_color);
        
        // Icon & Text
        let text_color = if selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) };
        
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(icon).size(18.0).color(text_color));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(label).size(16.0).color(text_color));
            });
        });
        
        ui.add_space(4.0); // Spacing between items
    }

    fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.add_space(20.0);
        
        // Network Stats Grid (Cards)
        ui.columns(3, |columns| {
            // Card 1: Block Height
            columns[0].group(|ui| {
                ui.set_min_height(100.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("📦").size(24.0));
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Block Height").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("#{}", self.block_height)).size(24.0).strong());
                });
            });
            
            // Card 2: Peers
            columns[1].group(|ui| {
                ui.set_min_height(100.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("🔗").size(24.0));
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Active Peers").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("{}", self.peer_count)).size(24.0).strong());
                });
            });
            
            // Card 3: Status
            columns[2].group(|ui| {
                ui.set_min_height(100.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("🟢").size(24.0));
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Network Status").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(&self.sync_status).size(18.0).strong().color(egui::Color32::GREEN));
                });
            });
        });
        
        ui.add_space(30.0);
        
        // Recent Blocks List
        ui.label(egui::RichText::new("Recent Activity").size(18.0).strong());
        ui.add_space(10.0);
        
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(17, 22, 37))
            .rounding(8.0)
            .inner_margin(15.0)
            .show(ui, |ui| {
                if self.recent_blocks.is_empty() {
                    ui.label(egui::RichText::new("Waiting for blocks...").italics().color(egui::Color32::GRAY));
                } else {
                    for block in &self.recent_blocks {
                         ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🧱").size(16.0));
                            ui.label(egui::RichText::new(format!("Block #{}", block.index)).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&block.hash[..12]).monospace().color(egui::Color32::GRAY));
                            });
                         });
                         ui.separator();
                    }
                }
            });
    }
    
    fn render_wallet(&mut self, ui: &mut egui::Ui) {
        ui.heading("Wallet");
        ui.add_space(10.0);
        
        // Wallet selector
        ui.horizontal(|ui| {
            ui.label("Select Wallet:");
            egui::ComboBox::from_label("")
                .selected_text(&self.selected_wallet)
                .show_ui(ui, |ui| {
                    for wallet in self.wallets.keys() {
                        ui.selectable_value(&mut self.selected_wallet, wallet.clone(), wallet);
                    }
                });
        });
        
        ui.add_space(10.0);
        
        // Balance display
        if let Some(wallet) = self.wallets.get(&self.selected_wallet) {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Balances").strong());
                ui.separator();
                if wallet.balances.is_empty() {
                    ui.label("No balances found (Fetching...)");
                } else {
                    for (asset, bal) in &wallet.balances {
                         ui.label(format!("{}: {}", asset, bal));
                    }
                }
            });
        }
        
        ui.add_space(20.0);
        
        // Send form
        ui.group(|ui| {
            ui.label(egui::RichText::new("Send Tokens").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("To:");
                ui.text_edit_singleline(&mut self.send_to);
            });
            
            ui.horizontal(|ui| {
                ui.label("Amount:");
                ui.text_edit_singleline(&mut self.send_amount);
            });
            
            ui.horizontal(|ui| {
                ui.label("Asset:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.send_asset)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.send_asset, "Compass".to_string(), "Compass");
                        ui.selectable_value(&mut self.send_asset, "COMPUTE".to_string(), "COMPUTE");
                    });
            });
            
            if ui.button("Send Transaction").clicked() {
                // Validate and send
                if let Ok(amount) = self.send_amount.parse::<f64>() {
                    let amount_u64 = (amount * 1e8) as u64;
                    let _ = self.rpc_tx.try_send(RpcCommand::SendTransaction {
                        from: self.selected_wallet.clone(),
                        to: self.send_to.clone(),
                        asset: self.send_asset.clone(),
                        amount: amount_u64,
                    });
                } else {
                    self.error_msg = Some("Invalid amount".to_string());
                }
            }
        });
    }
    
    fn render_workers(&mut self, ui: &mut egui::Ui) {
        ui.heading("Compute Workers (DePIN)");
        
        ui.add_space(10.0);
        
        // In-App Worker Control
        ui.group(|ui| {
            ui.heading("🖥️ In-App Worker");
            ui.label("Turn this GUI into a Compute Node. Earn tokens by training models locally.");
            
            ui.add_space(5.0);
            
            if ui.checkbox(&mut self.gui_worker_active, "Enable In-App Worker").changed() {
                 let _ = self.gui_worker_tx.try_send(self.gui_worker_active);
            }
            
            ui.add_space(5.0);
            ui.label(format!("Status: {}", self.gui_worker_status));
            ui.add(egui::ProgressBar::new(self.gui_worker_progress).show_percentage());
            
            ui.collapsing("Worker Logs", |ui| {
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for log in &self.gui_worker_logs {
                        ui.monospace(log);
                    }
                });
            });

            // New Visualization Section
            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("🧠 Last Inference Result").strong());
            if let Some(last_log) = self.gui_worker_logs.last() {
                if last_log.contains("Result Hash") || last_log.contains("Inference Complete") {
                     ui.group(|ui| {
                        ui.label(egui::RichText::new(last_log).color(egui::Color32::GREEN).monospace());
                     });
                } else {
                    ui.label(egui::RichText::new("Waiting for results...").italics());
                }
            }
        });
        
        ui.add_space(20.0);
        ui.separator();

        ui.heading("Workers");
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label(egui::RichText::new("Active Jobs").strong());
            ui.separator();
            
            if self.jobs.is_empty() {
                ui.label("No active jobs");
            } else {
                for job in &self.jobs {
                    ui.horizontal(|ui| {
                        ui.label(&job.job_id);
                        ui.label(&job.status);
                        ui.add(egui::ProgressBar::new(job.progress).text(format!("{}%", (job.progress * 100.0) as u32)));
                        ui.label(format!("{} COMPUTE", job.reward));
                    });
                }
            }
        });
        
        ui.add_space(20.0);
        
        ui.group(|ui| {
            ui.label(egui::RichText::new("Performance").strong());
            ui.separator();
            ui.label(format!("Total Rewards Earned: {} COMPUTE", self.total_rewards));
        });
    }
    
    fn render_oracle(&mut self, ui: &mut egui::Ui) {
        ui.heading("Oracle");
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label(egui::RichText::new("Market Data").strong());
            ui.separator();
            
            if self.oracle_prices.is_empty() {
                ui.label("Loading market data...");
            } else {
                for (ticker, price) in &self.oracle_prices {
                    ui.horizontal(|ui| {
                        ui.label(ticker);
                        ui.label(format!("${:.2}", price));
                    });
                }
            }
        });
        
        ui.add_space(20.0);
        
        // Oracle Monitor
        ui.group(|ui| {
            ui.label(egui::RichText::new("LTC Oracle Monitor").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("LTC Address:");
                ui.text_edit_singleline(&mut self.oracle_ltc_address);
            });
            
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                if self.oracle_monitor_running {
                    if ui.button(egui::RichText::new("🛑 Stop Monitor").color(egui::Color32::RED)).clicked() {
                        self.stop_oracle_monitor();
                    }
                    ui.spinner();
                    ui.label("Running...");
                } else {
                    if ui.button(egui::RichText::new("▶️ Start Monitor").color(egui::Color32::GREEN)).clicked() {
                        self.start_oracle_monitor();
                    }
                }
            });
            
            ui.add_space(10.0);
            ui.label("Logs:");
            
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.set_min_width(500.0);
                    for log in &self.oracle_logs {
                        ui.label(egui::RichText::new(log));
                    }
                });
        });
    }
    
    fn start_oracle_monitor(&mut self) {
        if self.oracle_monitor_running { return; }
        
        let ltc_address = self.oracle_ltc_address.clone();
        let (log_tx, log_rx) = mpsc::channel(100);
        let (stop_tx, stop_rx) = mpsc::channel(1);
        
        self.oracle_log_rx = Some(log_rx);
        self.oracle_stop_tx = Some(stop_tx);
        self.oracle_monitor_running = true;
        
        tokio::spawn(async move {
            let mut monitor = OracleMonitor::new(ltc_address, "oracle.json");
            monitor.run(log_tx, stop_rx).await;
        });
    }
    
    fn stop_oracle_monitor(&mut self) {
        if let Some(tx) = &self.oracle_stop_tx {
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(()).await;
            });
        }
        self.oracle_monitor_running = false;
        self.oracle_stop_tx = None;
    }

    fn render_vaults(&mut self, ui: &mut egui::Ui) {
        ui.heading("Vaults");
        ui.add_space(10.0);
        
        // Create Vault Form
        ui.group(|ui| {
            ui.label(egui::RichText::new("Create New Vault").strong());
            ui.separator();
            
            ui.label("Step 1: Send payment to admin address");
            ui.horizontal(|ui| {
                ui.label("Asset:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.vault_payment_asset)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.vault_payment_asset, "LTC".to_string(), "LTC");
                        ui.selectable_value(&mut self.vault_payment_asset, "BTC".to_string(), "BTC");
                    });
            });
            
            ui.horizontal(|ui| {
                ui.label("TX Hash:");
                ui.text_edit_singleline(&mut self.vault_tx_hash);
            });
            
            ui.add_space(10.0);
            ui.label("Step 2: Set your collateral and mint amount");
            
            ui.horizontal(|ui| {
                ui.label("Lock COMPASS:");
                ui.text_edit_singleline(&mut self.vault_compass_collateral);
                ui.label("tokens");
            });
            
            ui.horizontal(|ui| {
                ui.label("Mint Amount:");
                ui.text_edit_singleline(&mut self.vault_mint_amount);
                ui.label(format!("Compass-{}", self.vault_payment_asset));
            });
            
           // Calculate and show rate
            if let (Ok(collateral), Ok(mint)) = (
                self.vault_compass_collateral.parse::<f64>(),
                self.vault_mint_amount.parse::<f64>()
            ) {
                if mint > 0.0 {
                    let rate = collateral / mint;
                    ui.label(egui::RichText::new(format!("Rate: {:.2} COMPASS per Compass-{}", rate, self.vault_payment_asset))
                        .color(egui::Color32::from_rgb(100, 200, 100)));
                }
            }
            
            ui.add_space(10.0);
            if ui.button("📤 Submit Vault Request").clicked() {
                if let (Ok(collateral), Ok(mint), Ok(amt)) = (
                    self.vault_compass_collateral.parse::<u64>(),
                    self.vault_mint_amount.parse::<u64>(),
                    Ok::<u64, String>(100000000) 
                ) {
                    let _ = self.rpc_tx.try_send(RpcCommand::SubmitNativeVault {
                        payment_asset: self.vault_payment_asset.clone(),
                        payment_amount: amt,
                        compass_collateral: collateral,
                        requested_mint_amount: mint,
                        owner_id: self.current_user.clone(),
                        tx_hash: self.vault_tx_hash.clone()
                    });
                } else {
                    self.error_msg = Some("Invalid numbers".to_string());
                }
            }
        });
        
        ui.add_space(20.0);
        
        // Active Vaults (placeholder)
        ui.group(|ui| {
            ui.label(egui::RichText::new("Active Vaults").strong());
            ui.separator();
            ui.label("No vaults created yet");
            ui.label("(Vault display coming soon)");
        });
    }

    fn render_marketplace(&mut self, ui: &mut egui::Ui) {
        ui.heading("Marketplace");
        ui.add_space(10.0);
        
        // Filter out my own models for buying
        let listings: Vec<_> = self.marketplace_listings.iter()
            .filter(|n| n.owner != self.current_user)
            .collect();
            
        if listings.is_empty() {
            ui.label("No active listings found.");
        } else {
            egui::Grid::new("market_grid").striped(true).show(ui, |ui| {
                ui.label(egui::RichText::new("Model").strong());
                ui.label(egui::RichText::new("Seller").strong());
                ui.label(egui::RichText::new("Accuracy").strong());
                ui.label(egui::RichText::new("Price").strong());
                ui.label("");
                ui.end_row();
                
                for nft in listings {
                    ui.label(&nft.name);
                    ui.label(&nft.owner);
                    ui.label(format!("{:.1}%", nft.accuracy * 100.0));
                    ui.label(format!("{} CP", nft.price));
                    
                    if ui.button("Buy Now").clicked() {
                        let _ = self.rpc_tx.try_send(RpcCommand::BuyNFT { 
                            token_id: nft.token_id.clone(),
                            buyer: self.current_user.clone()
                        });
                    }
                    ui.end_row();
                }
            });
        }
    }

    fn render_train(&mut self, ui: &mut egui::Ui) {
        ui.heading("Train Model");
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.label(egui::RichText::new("Start Training Job").strong());
            ui.separator();
            
            // Model Selector
            ui.horizontal(|ui| {
                ui.label("Select Model:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.train_model_id)
                    .show_ui(ui, |ui| {
                        ui.weak("Foundation Models");
                        let foundation = vec!["gpt-4o", "gpt-4-turbo", "llama-3-70b", "stable-diffusion-xl"];
                        for model in foundation {
                            ui.selectable_value(&mut self.train_model_id, model.to_string(), model);
                        }
                        
                        ui.separator();
                        ui.weak("My Models (NFTs)");
                        if self.my_models.is_empty() {
                            ui.label("No owned model NFTs");
                        }
                        for model in &self.my_models {
                             ui.selectable_value(&mut self.train_model_id, model.token_id.clone(), &model.name);
                        }
                        
                        ui.separator();
                        ui.weak("Public Models (Marketplace)");
                        if self.marketplace_listings.is_empty() {
                            ui.label("No public models found");
                        }
                        for model in &self.marketplace_listings {
                             // deduplicate if in my_models
                             if !self.my_models.iter().any(|m| m.token_id == model.token_id) {
                                ui.selectable_value(&mut self.train_model_id, model.token_id.clone(), &model.name);
                             }
                        }
                    });
            });
            
            ui.add_space(10.0);
            
            // Dataset
            ui.horizontal(|ui| {
                ui.label("Dataset URL/Hash:");
                ui.text_edit_singleline(&mut self.train_dataset);
                
                ui.menu_button("📄 Examples", |ui| {
                    if ui.button("MNIST (Handwriting)").clicked() { self.train_dataset = "ipfs://mnist-sample-v1".to_string(); ui.close_menu(); }
                    if ui.button("CIFAR-10 (Images)").clicked() { self.train_dataset = "ipfs://cifar10-v4".to_string(); ui.close_menu(); }
                    if ui.button("Shakespeare (Text)").clicked() { self.train_dataset = "ipfs://shakespeare-txt".to_string(); ui.close_menu(); }
                });
            });
            
            ui.add_space(20.0);
            
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                 if ui.button(egui::RichText::new("🚀 Launch Training").size(16.0)).clicked() {
                    if !self.train_model_id.is_empty() && !self.train_dataset.is_empty() {
                        let _ = self.rpc_tx.try_send(RpcCommand::SubmitCompute {
                            model_id: self.train_model_id.clone(),
                            dataset: self.train_dataset.clone(),
                        });
                    } else {
                        self.error_msg = Some("Please select a model and dataset".to_string());
                    }
                }
                
                ui.add_space(20.0);
                ui.checkbox(&mut self.train_auto_loop, "🔄 Auto-Loop (Generate Jobs continuously)");
            });

            ui.add_space(20.0);
            ui.separator();
            ui.label(egui::RichText::new("Register New Model").strong());
            ui.horizontal(|ui| {
                 ui.label("Model Ticker:");
                 ui.text_edit_singleline(&mut self.train_model_id); // Reusing field for MVP
                 if ui.button("Mint Model (10,000 CP)").clicked() {
                      let _ = self.rpc_tx.try_send(RpcCommand::PurchaseNeuralNet {
                          owner: self.current_user.clone(),
                          ticker: self.train_model_id.clone()
                      });
                 }
            });
        });
        
        ui.add_space(20.0);
        
        // My Models List
        ui.label(egui::RichText::new("My Models").strong());
        for model in &self.my_models {
            ui.horizontal(|ui| {
                ui.label("🤖");
                ui.label(&model.name);
                ui.label(egui::RichText::new(format!("Acc: {:.1}%", model.accuracy*100.0)).color(egui::Color32::GRAY));
            });
        }
    }
    
    fn render_admin(&mut self, ui: &mut egui::Ui) {
         ui.heading("Admin Panel");
         ui.label(egui::RichText::new("Restricted Access: Node Admin Only").color(egui::Color32::RED));
         ui.add_space(20.0);
         
         ui.group(|ui| {
             ui.label(egui::RichText::new("Create Recurring Oracle Job").strong());
             ui.separator();
             
             ui.horizontal(|ui| {
                 ui.label("Ticker:");
                 ui.text_edit_singleline(&mut self.train_model_id); // Reuse string field temp or add new
             });
             // We need fields for this. Using hardcoded for MVP or reusing fields to save time?
             // Let's reuse 'train_model_id' for Ticker and 'train_dataset' for Duration (parsed)
             // Ideally we add new fields to struct, but for brevity:
             
             if ui.button("Create BTC Oracle Job (24h)").clicked() {
                  let _ = self.rpc_tx.try_send(RpcCommand::SubmitRecurringJob { 
                      ticker: "BTC".to_string(), 
                      duration_hours: 24, 
                      interval_minutes: 1, 
                      reward_per_update: 10, 
                      submitter: self.current_user.clone() 
                  });
             }
         });
    }

    fn render_login(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(200.0);
                
                ui.heading(egui::RichText::new("⚡ Compass Blockchain").size(32.0));
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Desktop Client").size(16.0).color(egui::Color32::GRAY));
                
                ui.add_space(50.0);
                
                // Login form
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(30, 30, 35))
                .rounding(10.0)
                .inner_margin(40.0)
                .show(ui, |ui| {
                    ui.set_max_width(400.0);
                    
                    if self.showing_create_wallet {
                         ui.label(egui::RichText::new("Create New Wallet").size(20.0).strong());
                         ui.add_space(20.0);
                         
                         if let Some(mnemonic) = &self.created_mnemonic {
                             ui.label("✅ Wallet Created!");
                             ui.label(egui::RichText::new("Save these words safely (Mnemonic):").color(egui::Color32::YELLOW));
                             ui.add_space(5.0);
                             ui.label(egui::RichText::new(mnemonic).monospace().background_color(egui::Color32::BLACK));
                             ui.add_space(20.0);
                             
                             if ui.button("Back to Login").clicked() {
                                 self.showing_create_wallet = false;
                                 self.created_mnemonic = None;
                                 self.new_wallet_name.clear();
                             }
                         } else {
                             ui.label("New Wallet Name:");
                             ui.text_edit_singleline(&mut self.new_wallet_name);
                             ui.add_space(20.0);
                             
                             if ui.button(egui::RichText::new("✨ Generate Wallet").size(16.0)).clicked() {
                                 if !self.new_wallet_name.is_empty() {
                                     if self.local_wallet_manager.get_wallet(&self.new_wallet_name).is_some() {
                                         self.error_msg = Some("Wallet name already exists".to_string());
                                     } else {
                                         let wallet = Wallet::new(&self.new_wallet_name, WalletType::User);
                                         if let Some(m) = &wallet.mnemonic {
                                             self.created_mnemonic = Some(m.clone());
                                             self.local_wallet_manager.wallets.insert(wallet.owner.clone(), wallet);
                                             let _ = self.local_wallet_manager.save("wallets.json");
                                             self.error_msg = None;
                                         }
                                     }
                                 } else {
                                     self.error_msg = Some("Name required".to_string());
                                 }
                             }
                             
                             ui.add_space(10.0);
                             if ui.button("Cancel").clicked() {
                                 self.showing_create_wallet = false;
                             }
                         }
                    } else {
                        // Regular Login
                        ui.label(egui::RichText::new("Login").size(20.0).strong());
                        ui.add_space(20.0);
                        
                        ui.label("Wallet Name:");
                        ui.text_edit_singleline(&mut self.login_username);
                        ui.add_space(10.0);
                        
                        ui.label("Password (Optional):");
                        let password_edit = egui::TextEdit::singleline(&mut self.login_password)
                            .password(true);
                        if ui.add(password_edit).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.attempt_login();
                        }
                        
                        ui.add_space(20.0);
                        
                        if ui.button(egui::RichText::new("🔑 Login").size(16.0)).clicked() {
                            self.attempt_login();
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        
                        if ui.button("Create New Wallet").clicked() {
                            self.showing_create_wallet = true;
                            self.error_msg = None;
                        }
                    }
                    
                    if let Some(err) = &self.error_msg {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(err).color(egui::Color32::RED));
                    }
                });
        });
    });
}

    fn attempt_login(&mut self) {
        if self.login_username.is_empty() {
            self.error_msg = Some("Wallet name required".to_string());
            return;
        }
        
        // Check against local wallet manager
        if self.local_wallet_manager.get_wallet(&self.login_username).is_some() || self.login_username == "admin" {
            self.current_user = self.login_username.clone();
            self.selected_wallet = self.login_username.clone();
            self.logged_in = true;
            self.error_msg = None;
            
            // Add to cache if missing
            if !self.wallets.contains_key(&self.current_user) {
                self.wallets.insert(self.current_user.clone(), WalletInfo { balances: HashMap::new() });
            }
        } else {
            self.error_msg = Some("Wallet not found. Please create one.".to_string());
        }
    }
}
