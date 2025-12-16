use eframe::egui;
use rust_compass::client::RpcClient;
use rust_compass::oracle::monitor::OracleMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use rust_compass::wallet::{WalletManager, Wallet, WalletType};
use rust_compass::layer3::compute::ComputeJob;
use rust_compass::rpc::types::TrainingModelInfo;
use rust_compass::layer3::onnx_inference::{ModelRegistry, price_to_signal};
use std::sync::Mutex;

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
    Portfolio, // NEW: Track training & NFTs
    Train,
    Pools, // Phase 5
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

#[derive(Debug, Clone, Default)]
struct EpochStats {
    ticker: String,
    model_id: String,
    current_epoch: u32,
    predictions_in_epoch: u32,
    correct_in_epoch: u32,
    total_predictions: u64,
    total_correct: u64,
    overall_accuracy: f64,
    nft_minted: bool,
}

#[derive(Debug, Clone)]
struct PredictionInfo {
    id: String,
    ticker: String,
    predicted_price: f64,
    actual_price: Option<f64>,
    signal: String,
    correct: Option<bool>,
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
    trainable_models: Vec<TrainingModelInfo>,
    
    // In-App Worker State
    gui_worker_active: bool,
    gui_worker_status: String,
    gui_worker_progress: f32,
    gui_worker_logs: Vec<String>,
    gui_worker_tx: mpsc::Sender<(bool, String)>, // (active, wallet_id)
    
    // Epoch Tracking (Phase 5)
    epoch_stats: HashMap<String, EpochStats>, // ticker -> stats
    prediction_history: Vec<PredictionInfo>,
    
    // Shared Pools (Phase 5)
    model_pools: Vec<rust_compass::layer3::collective::ModelPool>,
    create_pool_name: String,
    create_pool_type: String,
    join_amount: String,
    
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
    PurchasePrediction { ticker: String, buyer_id: String },
    GetTrainableModels,
    GetEpochStats(String), // ticker
    GetPredictionHistory(String, u64), // ticker, limit
    // Shared Pools (Phase 5)
    GetModelPools,
    CreateModelPool { name: String, model_type: String, creator: String },
    JoinPool { pool_id: String, contributor: String, amount: u64 },
    ClaimDividends { pool_id: String, contributor: String },
    MintNFT { ticker: String, model_id: String },
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
    TrainableModelsUpdate { models: Vec<TrainingModelInfo> },
    EpochStatsUpdate { stats: EpochStats },
    PredictionHistoryUpdate { predictions: Vec<PredictionInfo> },
    ModelPoolsUpdate { pools: Vec<rust_compass::layer3::collective::ModelPool> }, // New

    WorkerUpdate { status: String, progress: f32, log: Option<String>, reward: Option<u64> }, // New for GUI Worker
    Error(String),
}

impl CompassApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (rpc_tx, mut rpc_rx) = mpsc::channel(32);
        let (response_tx, response_rx) = mpsc::channel(32);
        
        // Spawn In-App Worker Thread
        let (worker_tx, mut worker_rx): (mpsc::Sender<(bool, String)>, mpsc::Receiver<(bool, String)>) = mpsc::channel(32);
        let response_tx_clone = response_tx.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            rt.block_on(async move {
                let client = RpcClient::new("http://localhost:9000/".to_string());
                let mut active = false;
                let mut worker_wallet = String::new(); // Wallet to receive rewards
                
                // Initialize AI Model Registry
                let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                    status: "Initializing AI...".to_string(), 
                    progress: 0.1,
                    log: Some("🧠 Loading LSTM Neural Networks...".to_string()), 
                    reward: None 
                }).await;

                let mut registry = ModelRegistry::new();
                for ticker in ["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"] {
                    match registry.load_model(ticker) {
                        Ok(_) => {
                             let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                status: "Initializing AI...".to_string(), 
                                progress: 0.1, 
                                log: Some(format!("✅ Loaded Model: {}", ticker)), 
                                reward: None 
                            }).await;
                        }
                        Err(e) => {
                             let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                status: "Initializing AI...".to_string(), 
                                progress: 0.1, 
                                log: Some(format!("⚠️ Model missing for {}: {}", ticker, e)), 
                                reward: None 
                            }).await;
                        }
                    }
                }
                let model_registry = Arc::new(Mutex::new(registry));
                
                loop {
                    if let Ok((should_run, wallet)) = worker_rx.try_recv() {
                        active = should_run;
                        worker_wallet = wallet;
                        let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                            status: if active { "Searching for jobs..." } else { "Paused" }.to_string(), 
                            progress: 0.0,
                            log: Some(if active { format!("✅ Worker Started (Wallet: {})", worker_wallet) } else { "⏸ Worker Paused".to_string() }),
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
                                         println!("DEBUG [GUI Worker]: Accepted job_id='{}', model_id='{}'", job.job_id, job.model_id);
                                         let compute_job = ComputeJob::new(
                                             job.job_id.clone(),
                                             job.creator.clone(),
                                             job.model_id.clone(),
                                             job.inputs.clone(), // Correctly pass inputs from PendingJob
                                             job.max_compute_units,
                                         );
                                         
                                         let mut final_result_data = Vec::new(); // Store bytes to send
                                         let mut final_hash = String::new();
                                         
                                         // Special Handling for Training Job
                                         if job.model_id.starts_with("train_") {
                                              // Extract ticker (train_sol_v1 -> sol)
                                              let parts: Vec<&str> = job.model_id.split('_').collect();
                                              let ticker_short = if parts.len() >= 2 { parts[1] } else { "sol" };
                                              let ticker_upper = ticker_short.to_uppercase();
                                              let binance_ticker = format!("{}USDT", ticker_upper); // e.g. BTCUSDT

                                              let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                 status: format!("Training {} Agent...", ticker_upper), 
                                                 progress: 0.1,
                                                 log: Some(format!("🧠 Native Rust Training Started for {}", binance_ticker)),
                                                 reward: None
                                             }).await;
                                             
                                              // Execute Python Training Script (Option B)
                                              let script_name = format!("scripts/train_{}_agent.py", ticker_short);
                                              
                                              let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                  status: format!("Training {} Agent...", ticker_upper), 
                                                  progress: 0.1,
                                                  log: Some(format!("🐍 Executing Python Script: {}", script_name)),
                                                  reward: None
                                              }).await;
                                              
                                              // Run Python Script
                                              let mut cmd_result = std::process::Command::new("python").arg(&script_name).output();
                                              
                                              // Fallback to 'py' if 'python' is missing (Windows)
                                              if cmd_result.is_err() {
                                                  cmd_result = std::process::Command::new("py").arg(&script_name).output();
                                              }

                                              match cmd_result {
                                                  Ok(output) => {
                                                      if output.status.success() {
                                                          let stdout = String::from_utf8_lossy(&output.stdout);
                                                          let last_line = stdout.lines().last().unwrap_or("Done");
                                                          
                                                          let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                              status: "Training Complete".to_string(), 
                                                              progress: 1.0,
                                                              log: Some(format!("✅ Training Success: {}", last_line)),
                                                              reward: None
                                                          }).await;
                                                          
                                                          // HOT RELOAD MODEL
                                                          if let Ok(mut reg) = model_registry.lock() {
                                                              match reg.load_model(&binance_ticker) {
                                                                  Ok(_) => println!("🔄 Hot-reloaded model for {}", binance_ticker),
                                                                  Err(e) => println!("⚠️ Failed to reload model: {}", e),
                                                              }
                                                          }
                                                          
                                                          final_hash = format!("{}_training_done", ticker_short); 
                                                          final_result_data = format!("minted_{}_via_python", ticker_short).as_bytes().to_vec();
                                                      } else {
                                                          let stderr = String::from_utf8_lossy(&output.stderr);
                                                          let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                              status: "Training Failed".to_string(), 
                                                              progress: 0.0,
                                                              log: Some(format!("❌ Info: {}", stderr)), // stderr often has useful info even on failure
                                                              reward: None
                                                          }).await;
                                                      }
                                                  }
                                                  Err(e) => {
                                                       let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                          status: "Execution Error".to_string(), 
                                                          progress: 0.0,
                                                          log: Some(format!("❌ Failed to spawn python: {}", e)),
                                                          reward: None
                                                      }).await;
                                                  }
                                              }

                                          } else {
                                              // Execute LSTM Inference (Option B)
                                              let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                  status: format!("Running LSTM Inference: {}", job.model_id), 
                                                  progress: 0.5,
                                                  log: Some("🧠 Processing inputs through neural net...".to_string()),
                                                  reward: None
                                              }).await;
    
                                              // Determine ticker from job model_id
                                              let ticker = if job.model_id.contains("btc") { "BTCUSDT" }
                                                  else if job.model_id.contains("eth") { "ETHUSDT" }
                                                  else if job.model_id.contains("ltc") { "LTCUSDT" }
                                                  else { "SOLUSDT" }; // Default

                                              // Parse inputs (Vec<Vec<f64>>)
                                              let sequence: Vec<Vec<f64>> = serde_json::from_slice(&job.inputs).unwrap_or_default();
                                              let current_price = sequence.last().and_then(|c| c.first().cloned()).unwrap_or(0.0);

                                              // Run Prediction
                                              let registry = model_registry.lock().unwrap();
                                              match registry.predict(ticker, &sequence) {
                                                  Ok(predicted_price) => {
                                                      // Convert Price -> Signal
                                                      let signal_val = price_to_signal(current_price, predicted_price, 0.005); // 0.5% Threshold
                                                      
                                                      // Create Proof of Inference (Hash)
                                                      let hash = format!("lstm_{}_{:.2}_{}", ticker, predicted_price, signal_val);
                                                      final_hash = hash.clone();

                                                      // Prepare Result JSON
                                                      let result = serde_json::json!({
                                                          "prediction": signal_val,
                                                          "predicted_price": predicted_price,
                                                          "current_price": current_price,
                                                          "hash": hash
                                                      });
                                                      final_result_data = result.to_string().into_bytes();

                                                      // Log Success
                                                      let signal_str = match signal_val { 0 => "SELL", 1 => "HOLD", 2 => "BUY", _ => "?" };
                                                      let log_msg = format!("🔮 LSTM Validated: {} (${:.2} -> ${:.2})", signal_str, current_price, predicted_price);
                                                      
                                                      let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                          status: "Inference Complete".to_string(), 
                                                          progress: 1.0,
                                                          log: Some(log_msg),
                                                          reward: None
                                                      }).await;
                                                  }
                                                  Err(e) => {
                                                      // Fallback Logic
                                                      let _ = response_tx_clone.send(RpcResponse::WorkerUpdate { 
                                                          status: "Inference Error".to_string(), 
                                                          progress: 0.0,
                                                          log: Some(format!("❌ LSTM Inference Failed: {}", e)),
                                                          reward: None
                                                      }).await;
                                                      
                                                      // Fallback to HOLD
                                                      final_result_data = serde_json::json!({ "prediction": 1, "hash": "fallback" }).to_string().into_bytes();
                                                      final_hash = "fallback".to_string();
                                                  }
                                              }
                                          }
                                      tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                     
                                     let req = serde_json::json!({
                                         "job_id": job.job_id,
                                          "worker_id": worker_wallet.clone(),
                                         "result_data": final_result_data,
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
                        RpcCommand::PurchasePrediction { ticker, buyer_id } => {
                             let req = serde_json::json!({ "ticker": ticker, "buyer_id": buyer_id });
                             match client.call_method::<serde_json::Value, serde_json::Value>("purchasePrediction", req).await {
                                 Ok(res) => { 
                                     // Format the signal for display
                                     let signal = res.get("signal").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
                                     let price = res.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0);
                                     let timestamp = res.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0);
                                     
                                     let msg = format!("🔮 SIGNAL RECEIVED ({}) \nTrend: {}\nTarget: ${:.2}", ticker, signal, price);
                                     // Reuse TransactionSent or add new Response type? 
                                     // Let's use TransactionSent for simplicity as it shows success in status bar
                                     let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: msg }).await; 
                                 }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Buy Signal Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::GetTrainableModels => {
                            match client.call_method::<serde_json::Value, Vec<rust_compass::rpc::types::TrainingModelInfo>>("getTrainableModels", serde_json::json!({})).await {
                                Ok(models) => {
                                    let _ = response_tx.send(RpcResponse::TrainableModelsUpdate { models }).await;
                                }
                                Err(e) => {
                                    let _ = response_tx.send(RpcResponse::Error(format!("Failed to fetch models: {}", e))).await;
                                }
                            }
                        }
                        RpcCommand::GetEpochStats(ticker) => {
                            // Match oracle scheduler format: signal_{ticker_short}_v2
                            let ticker_short = ticker.to_lowercase().replace("usdt", "");
                            let model_id = format!("signal_{}_v2", ticker_short);
                            
                            let req = serde_json::json!({
                                "ticker": ticker,
                                "model_id": model_id
                            });
                            match client.call_method::<serde_json::Value, serde_json::Value>("getModelEpochStats", req).await {
                                Ok(res) => {
                                    let stats = EpochStats {
                                        ticker: ticker.clone(),
                                        model_id: res.get("model_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        current_epoch: res.get("current_epoch").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        predictions_in_epoch: res.get("predictions_in_epoch").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        correct_in_epoch: res.get("correct_in_epoch").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        total_predictions: res.get("total_predictions").and_then(|v| v.as_u64()).unwrap_or(0),
                                        total_correct: res.get("total_correct").and_then(|v| v.as_u64()).unwrap_or(0),
                                        overall_accuracy: res.get("overall_accuracy").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                        nft_minted: res.get("nft_minted").and_then(|v| v.as_bool()).unwrap_or(false),
                                    };
                                    let _ = response_tx.send(RpcResponse::EpochStatsUpdate { stats }).await;
                                }
                                Err(_) => {}
                            }
                        }
                        RpcCommand::GetPredictionHistory(ticker, limit) => {
                            let req = serde_json::json!({
                                "ticker": ticker,
                                "limit": limit
                            });
                            match client.call_method::<serde_json::Value, serde_json::Value>("getPredictionHistory", req).await {
                                Ok(res) => {
                                    let preds: Vec<PredictionInfo> = res.get("predictions")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| arr.iter().filter_map(|p| {
                                            Some(PredictionInfo {
                                                id: p.get("id")?.as_str()?.to_string(),
                                                ticker: p.get("ticker")?.as_str()?.to_string(),
                                                predicted_price: p.get("predicted_price")?.as_f64()?,
                                                actual_price: p.get("actual_price").and_then(|v| v.as_f64()),
                                                signal: p.get("signal").and_then(|v| v.as_str()).unwrap_or("HOLD").to_string(),
                                                correct: p.get("correct").and_then(|v| v.as_bool()),
                                                timestamp: p.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
                                            })
                                        }).collect())
                                        .unwrap_or_default();
                                    let _ = response_tx.send(RpcResponse::PredictionHistoryUpdate { predictions: preds }).await;
                                }
                                Err(_) => {}
                            }
                        }
                        RpcCommand::GetModelPools => {
                            match client.call_method::<serde_json::Value, Vec<rust_compass::layer3::collective::ModelPool>>("getModelPools", serde_json::json!({})).await {
                                Ok(pools) => { let _ = response_tx.send(RpcResponse::ModelPoolsUpdate { pools }).await; }
                                Err(_) => {}
                            }
                        }
                        RpcCommand::CreateModelPool { name, model_type, creator } => {
                             let req = serde_json::json!({ "name": name, "model_type": model_type, "creator": creator });
                             match client.call_method::<serde_json::Value, serde_json::Value>("createModelPool", req).await {
                                 Ok(_) => { let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Pool Created".to_string() }).await; }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Pool Create Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::JoinPool { pool_id, contributor, amount } => {
                             let req = serde_json::json!({ "pool_id": pool_id, "contributor": contributor, "amount": amount });
                             match client.call_method::<serde_json::Value, serde_json::Value>("joinPool", req).await {
                                 Ok(_) => { let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: "Joined Pool".to_string() }).await; }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Join Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::ClaimDividends { pool_id, contributor } => {
                             let req = serde_json::json!({ "pool_id": pool_id, "contributor": contributor });
                             match client.call_method::<serde_json::Value, serde_json::Value>("claimDividends", req).await {
                                 Ok(res) => { 
                                      let amt = res.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
                                      let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: format!("Claimed {} COMPUTE", amt) }).await; 
                                 }
                                 Err(e) => { let _ = response_tx.send(RpcResponse::Error(format!("Claim Failed: {}", e))).await; }
                             }
                        }
                        RpcCommand::MintNFT { ticker, model_id } => {
                            let req = serde_json::json!({ "ticker": ticker, "model_id": model_id });
                            match client.call_method::<serde_json::Value, serde_json::Value>("mintModelNFT", req).await {
                                Ok(res) => {
                                    let message = res.get("message").and_then(|v| v.as_str()).unwrap_or("NFT Minted!");
                                    let _ = response_tx.send(RpcResponse::TransactionSent { tx_hash: message.to_string() }).await;
                                }
                                Err(e) => {
                                    let _ = response_tx.send(RpcResponse::Error(format!("Mint Failed: {}", e))).await;
                                }
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
            trainable_models: Vec::new(),
            
            gui_worker_active: false,
            gui_worker_status: "Paused".to_string(),
            gui_worker_progress: 0.0,
            gui_worker_logs: Vec::new(),
            gui_worker_tx: worker_tx,
            
            // Epoch tracking
            epoch_stats: HashMap::new(),
            prediction_history: Vec::new(),

            // Shared Pools (Phase 5)
            model_pools: Vec::new(),
            create_pool_name: String::new(),
            create_pool_type: "signal-classifier".to_string(),
            join_amount: "100".to_string(),

            
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
        
        // ALWAYS Poll Balance for selected wallet (for header display)
        if !self.selected_wallet.is_empty() {
            let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(self.selected_wallet.clone(), "Compass".to_string()));
            let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(self.selected_wallet.clone(), "COMPUTE".to_string()));
        }
        
        match self.current_page {
            Page::Dashboard => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetRecentBlocks(5));
            }
            Page::Portfolio => {
                // Poll epoch stats for all tickers
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("BTCUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("ETHUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("SOLUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("LTCUSDT".to_string()));
                // Poll user's NFTs
                let _ = self.rpc_tx.try_send(RpcCommand::GetMyModels(self.current_user.clone()));
            }
            Page::Pools => {
                 let _ = self.rpc_tx.try_send(RpcCommand::GetModelPools);
            }

            Page::Wallet => {
                for wallet in self.wallets.keys() {
                    let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(wallet.clone(), "Compass".to_string()));
                    let _ = self.rpc_tx.try_send(RpcCommand::GetBalance(wallet.clone(), "COMPUTE".to_string()));
                }
            }
            Page::Workers => {
                let _ = self.rpc_tx.try_send(RpcCommand::GetComputeJobs);
                // Poll epoch stats for all tickers
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("BTCUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("ETHUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("SOLUSDT".to_string()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetEpochStats("LTCUSDT".to_string()));
                // Poll prediction history
                let _ = self.rpc_tx.try_send(RpcCommand::GetPredictionHistory("BTCUSDT".to_string(), 20));
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
                let _ = self.rpc_tx.try_send(RpcCommand::GetMyModels(self.current_user.clone()));
                let _ = self.rpc_tx.try_send(RpcCommand::GetMarketplaceListings);
                let _ = self.rpc_tx.try_send(RpcCommand::GetTrainableModels);
                
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
                    let display_hash = if tx_hash.len() > 16 { &tx_hash[..16] } else { &tx_hash };
                    self.error_msg = Some(format!("✅ Transaction sent: {}", display_hash));
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
                RpcResponse::TrainableModelsUpdate { models } => {
                    self.trainable_models = models;
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
                RpcResponse::EpochStatsUpdate { stats } => {
                    self.epoch_stats.insert(stats.ticker.clone(), stats);
                }
                RpcResponse::PredictionHistoryUpdate { predictions } => {
                    self.prediction_history = predictions;
                }
                RpcResponse::ModelPoolsUpdate { pools } => {
                    self.model_pools = pools;
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
                    self.drawer_item(ui, Page::Portfolio, "📂", "Portfolio");
                    self.drawer_item(ui, Page::Workers, "⚙️", "Workers");
                    self.drawer_item(ui, Page::Oracle, "🔮", "Oracle");
                    self.drawer_item(ui, Page::Vaults, "🏦", "Vaults");
                    self.drawer_item(ui, Page::Marketplace, "🛒", "Marketplace");
                    self.drawer_item(ui, Page::Train, "🧠", "Train Model");
                    self.drawer_item(ui, Page::Pools, "👥", "Collective Pools"); // Phase 5
                    
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
                Page::Portfolio => self.render_portfolio(ui),
                Page::Workers => self.render_workers(ui),
                Page::Oracle => self.render_oracle(ui),
                Page::Vaults => self.render_vaults(ui),
                Page::Marketplace => self.render_marketplace(ui),
                Page::Train => self.render_train(ui),
                Page::Pools => self.render_pools(ui),
                Page::Admin => {
                self.render_admin(ui);
            }
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
        
        ui.add_space(20.0);

        // Admin Continual Training Status
        ui.group(|ui| {
            ui.horizontal(|ui| {
                 ui.label(egui::RichText::new("🧠 Admin Enhanced Model:").strong().size(16.0));
                 ui.label(egui::RichText::new("ACTIVE (Continual Training)").color(egui::Color32::GREEN).strong().size(16.0));
                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                     ui.label(egui::RichText::new("v1.2.0").monospace().color(egui::Color32::GRAY));
                 });
            });
            ui.label("The Admin Node is running a 24/7 background training loop to improve the 'price_decision_v1' model.");
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
                 let _ = self.gui_worker_tx.try_send((self.gui_worker_active, self.selected_wallet.clone()));
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
        
        // Epoch Progress Panel (Phase 5)
        ui.add_space(20.0);
        ui.separator();
        ui.heading("📊 Model Epoch Progress");
        ui.add_space(10.0);
        
        let tickers = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];
        
        for ticker in tickers {
            let stats = self.epoch_stats.get(ticker).cloned().unwrap_or_default();
            
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{}", ticker)).strong().size(14.0));
                    
                    if stats.nft_minted {
                        ui.label(egui::RichText::new("✅ NFT MINTED").color(egui::Color32::GREEN));
                    } else {
                        ui.label(egui::RichText::new(format!("Epoch {}/10", stats.current_epoch)).color(egui::Color32::YELLOW));
                    }
                });
                
                // Epoch progress bar
                let epoch_progress = stats.predictions_in_epoch as f32 / 10.0;
                ui.add(egui::ProgressBar::new(epoch_progress).text(format!("{}/10 predictions", stats.predictions_in_epoch)));
                
                // Accuracy
                ui.horizontal(|ui| {
                    let accuracy_color = if stats.overall_accuracy >= 0.75 {
                        egui::Color32::GREEN
                    } else if stats.overall_accuracy >= 0.5 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    
                    ui.label("Accuracy:");
                    ui.label(egui::RichText::new(format!("{:.1}%", stats.overall_accuracy * 100.0)).color(accuracy_color).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{}/{} correct", stats.total_correct, stats.total_predictions));
                    });
                });
            });
        }
        
        // Recent Predictions
        ui.add_space(10.0);
        ui.collapsing("Recent Predictions", |ui| {
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                if self.prediction_history.is_empty() {
                    ui.label(egui::RichText::new("No predictions yet...").italics().color(egui::Color32::GRAY));
                } else {
                    for pred in self.prediction_history.iter().take(20) {
                        ui.horizontal(|ui| {
                            let icon = match pred.correct {
                                Some(true) => "✅",
                                Some(false) => "❌",
                                None => "⏳"
                            };
                            ui.label(icon);
                            ui.label(format!("{} ${:.2} ({})", pred.ticker, pred.predicted_price, pred.signal));
                            if let Some(actual) = pred.actual_price {
                                ui.label(format!("→ ${:.2}", actual));
                            }
                        });
                    }
                }
            });
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

        ui.add_space(20.0);

        // Signal Marketplace Section
        ui.group(|ui| {
            ui.label(egui::RichText::new("🛒 Signal Marketplace").strong());
            ui.separator();
            ui.label("Purchase exclusive AI signals powered by the decentralized neural network.");
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("BTCUSDT (1h)").strong());
                ui.label("Price: 5 COMPASS");
                if ui.button("⚡ Buy Signal").clicked() {
                     let _ = self.rpc_tx.try_send(RpcCommand::PurchasePrediction {
                         ticker: "BTCUSDT".to_string(),
                         buyer_id: self.selected_wallet.clone() // Use selected wallet to pay
                     });
                }
            });
            
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ETHUSDT (1h)").strong());
                ui.label("Price: 5 COMPASS");
                if ui.button("⚡ Buy Signal").clicked() {
                     let _ = self.rpc_tx.try_send(RpcCommand::PurchasePrediction {
                         ticker: "ETHUSDT".to_string(),
                         buyer_id: self.selected_wallet.clone()
                     });
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

        fn render_portfolio(&mut self, ui: &mut egui::Ui) {
        ui.heading("📂 Portfolio");
        ui.label(egui::RichText::new("Track your training progress, owned NFTs, and marketplace listings")
            .color(egui::Color32::GRAY));
        ui.add_space(20.0);
        
        // ===== TRAINING EPOCH PROGRESS =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("🧠 Training Progress").size(18.0).strong());
            ui.separator();
            ui.label("Track your models\' progress towards NFT minting (requires 10 epochs with 75%+ accuracy)");
            ui.add_space(10.0);
            
            if self.epoch_stats.is_empty() {
                ui.label(egui::RichText::new("No active training sessions")
                    .color(egui::Color32::GRAY));
                ui.label("💡 Tip: Submitted training jobs will appear here once they start generating predictions");
            } else {
                for (key, stats) in &self.epoch_stats {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&stats.ticker).size(16.0).strong());
                            ui.label(egui::RichText::new(format!("Model: {}", stats.model_id))
                                .size(12.0)
                                .color(egui::Color32::GRAY));
                        });
                        
                        ui.add_space(8.0);
                        
                        // Progress bar to epoch 10
                        let progress = (stats.current_epoch as f32 / 10.0).min(1.0);
                        let accuracy_color = if stats.overall_accuracy >= 0.75 {
                            egui::Color32::from_rgb(34, 197, 94) // Green
                        } else {
                            egui::Color32::from_rgb(251, 146, 60) // Orange
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(format!("Epoch {}/10", stats.current_epoch));
                            ui.add(egui::ProgressBar::new(progress)
                                .desired_width(200.0)
                                .fill(accuracy_color));
                            ui.label(format!("{:.1}% accuracy", stats.overall_accuracy * 100.0));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(format!("Predictions: {}/{} in current epoch", 
                                stats.predictions_in_epoch, 
                                10)); // epochs have 10 predictions each
                            ui.label(format!("Total: {} ({} correct)", 
                                stats.total_predictions,
                                stats.total_correct));
                        });
                        
                        // Show mint button if qualified (10+ epochs, 75%+ accuracy)
                        if stats.current_epoch >= 10 && stats.overall_accuracy >= 0.75 {
                            // Show mint button with estimated value
                            let est_value = 1000 + (stats.overall_accuracy * 1000.0) as u64 + (stats.current_epoch as u64 * 50);
                            
                            ui.horizontal(|ui| {
                                if stats.nft_minted {
                                    ui.label(egui::RichText::new("⚠️  Flag set but no NFT found")
                                        .color(egui::Color32::from_rgb(251, 146, 60))
                                         .size(11.0));
                                } else {
                                    ui.label(egui::RichText::new("🎉 Ready to mint!")
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong());
                                }
                                
                                if ui.button(egui::RichText::new("🎨 Mint NFT")
                                    .size(14.0)
                                    .strong()).clicked() {
                                    let _ = self.rpc_tx.try_send(RpcCommand::MintNFT {
                                        ticker: stats.ticker.clone(),
                                        model_id: stats.model_id.clone(),
                                    });
                                }
                            });
                            
                            ui.label(egui::RichText::new(format!("💰 Est. Value: ~{} COMPASS", est_value))
                                .size(12.0)
                                .color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new("💡 Tip: Train longer for higher value!")
                                .size(11.0)
                                .color(egui::Color32::DARK_GRAY)
                                .italics());
                        }
                    });
                    
                    ui.add_space(10.0);
                }
            }
        });
        
        ui.add_space(20.0);
        
        // ===== OWNED NFTs =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("🎨 My NFTs").size(18.0).strong());
            ui.separator();
            ui.label("Your AI model NFTs - list them on the marketplace to earn from rentals and sales");
            ui.add_space(10.0);
            
            if self.my_models.is_empty() {
                ui.label(egui::RichText::new("You don't own any model NFTs yet")
                    .color(egui::Color32::GRAY));
                ui.label("💡 Tip: Train a model to 10 epochs with 75%+ accuracy to mint your first NFT");
            } else {
                egui::Grid::new("portfolio_nfts").striped(true).num_columns(6).show(ui, |ui| {
                    // Header
                    ui.label(egui::RichText::new("NFT").strong());
                    ui.label(egui::RichText::new("Name").strong());
                    ui.label(egui::RichText::new("Accuracy").strong());
                    ui.label(egui::RichText::new("List Price").strong());
                    ui.label(egui::RichText::new("Status").strong());
                    ui.label(egui::RichText::new("Actions").strong());
                    ui.end_row();
                    
                    for nft in &self.my_models.clone() {
                        // NFT Icon
                        ui.label("🤖");
                        
                        // Name
                        ui.label(&nft.name);
                        
                        // Accuracy
                        let acc_color = if nft.accuracy >= 0.80 {
                            egui::Color32::from_rgb(34, 197, 94)
                        } else if nft.accuracy >= 0.70 {
                            egui::Color32::from_rgb(251, 146, 60)
                        } else {
                            egui::Color32::from_rgb(239, 68, 68)
                        };
                        ui.label(egui::RichText::new(format!("{:.1}%", nft.accuracy * 100.0))
                            .color(acc_color));
                        
                        // List Price (editable)
                        let price_label = if nft.price > 0 {
                            format!("{} COMPASS", nft.price)
                        } else {
                            "Not Listed".to_string()
                        };
                        ui.label(price_label);
                        
                        // Status
                        let status = if nft.price > 0 {
                            egui::RichText::new("🟢 Listed").color(egui::Color32::from_rgb(34, 197, 94))
                        } else {
                            egui::RichText::new("⚫ Unlisted").color(egui::Color32::GRAY)
                        };
                        ui.label(status);
                        
                        // Actions
                        ui.horizontal(|ui| {
                            if nft.price > 0 {
                                if ui.button("📝 Update Price").clicked() {
                                    // TODO: Open price edit dialog
                                }
                                if ui.button("❌ Unlist").clicked() {
                                    let _ = self.rpc_tx.try_send(RpcCommand::BuyNFT {
                                        token_id: nft.token_id.clone(),
                                        buyer: "UNLIST".to_string(), // Special marker
                                    });
                                }
                            } else {
                                if ui.button("📤 List on Marketplace").clicked() {
                                    // TODO: Open listing dialog with price input
                                    // For now, list at default price
                                    let _ = self.rpc_tx.try_send(RpcCommand::BuyNFT {
                                        token_id: nft.token_id.clone(),
                                        buyer: "LIST:1000".to_string(), // price:amount format
                                    });
                                }
                            }
                        });
                        
                        ui.end_row();
                    }
                });
            }
        });
        
        ui.add_space(20.0);
        
        // ===== MARKETPLACE EARNINGS =====
        ui.group(|ui| {
            ui.label(egui::RichText::new("💰 Marketplace Earnings").size(18.0).strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Total Royalties Earned:");
                ui.label(egui::RichText::new("0 COMPUTE").strong()); // TODO: Track this
            });
            
            ui.horizontal(|ui| {
                ui.label("Active Rentals:");
                ui.label(egui::RichText::new("0").strong()); // TODO: Track this
            });
            
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Coming soon: Rental tracking and earnings dashboard")
                .color(egui::Color32::GRAY)
                .size(12.0));
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

            // Available Network Models (Dynamic)
            if !self.trainable_models.is_empty() {
                ui.label(egui::RichText::new("Available Network Campaigns").strong().color(egui::Color32::LIGHT_BLUE));
                 egui::Grid::new("train_models_grid").striped(true).show(ui, |ui| {
                    ui.label(egui::RichText::new("Ticker").strong());
                    ui.label(egui::RichText::new("Description").strong());
                    ui.label(egui::RichText::new("Reward").strong());
                    ui.label(egui::RichText::new("Action").strong());
                    ui.end_row();

                    for model in &self.trainable_models {
                        ui.label(egui::RichText::new(&model.ticker).strong());
                        ui.label(&model.description);
                        ui.label(format!("{} CP", model.reward));
                        
                        if ui.button("Select").clicked() {
                            self.train_model_id = model.model_id.clone();
                            self.train_dataset = "latest_oracle_data".to_string(); // Auto-fill
                        }
                        ui.end_row();
                    }
                });
                ui.add_space(10.0);
                ui.separator();
            }
            
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

         ui.add_space(20.0);

         ui.group(|ui| {
             ui.label(egui::RichText::new("Enhanced Model Training (24h Loop)").strong());
             ui.separator();
             ui.label("Runs a continuous background training session on live BTC data. Automatically updates the 'price_decision_v1' model and mints a new version.");
             
             ui.add_space(10.0);
             
             if ui.button(egui::RichText::new("🚀 Start Training Session (External Window)").size(16.0)).clicked() {
                 // Launch the PowerShell script
                 let _ = std::process::Command::new("powershell")
                     .args(&["-ExecutionPolicy", "Bypass", "-File", "scripts/start_training_session.ps1"])
                     .spawn();
                 
                 self.error_msg = Some("✅ Training Session Launched in new window".to_string());
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

    fn render_pools(&mut self, ui: &mut egui::Ui) {
        ui.heading("👥 Shared Function Model Pools");
        ui.label("Co-own high-performance models and earn royalties.");
        ui.separator();

        // Create Section
        ui.collapsing("Create New Pool", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.create_pool_name);
            });
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.text_edit_singleline(&mut self.create_pool_type);
            });
            if ui.button("Create Pool").clicked() {
                let _ = self.rpc_tx.try_send(RpcCommand::CreateModelPool {
                    name: self.create_pool_name.clone(),
                    model_type: self.create_pool_type.clone(),
                    creator: self.current_user.clone()
                });
            }
        });
        ui.separator();
        
        // Refresh Button
        if ui.button("🔄 Refresh Pools").clicked() {
             let _ = self.rpc_tx.try_send(RpcCommand::GetModelPools);
        }

        // List
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.model_pools.is_empty() {
                ui.label("No active pools found.");
            }
            
            for pool in &self.model_pools {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&pool.name);
                        ui.label(egui::RichText::new(&pool.pool_id).small().weak());
                    });
                    ui.label(format!("Type: {}", pool.model_type));
                    ui.label(format!("Total Staked: {} COMPASS", pool.total_staked));
                    ui.label(format!("Vault Balance: {} COMPUTE", pool.vault_balance));
                    
                    let my_share = pool.get_share(&self.current_user);
                    if my_share > 0.0 {
                        ui.label(egui::RichText::new(format!("Your Share: {:.2}%", my_share * 100.0)).color(egui::Color32::GREEN));
                        if ui.button("💰 Claim Dividends").clicked() {
                             let _ = self.rpc_tx.try_send(RpcCommand::ClaimDividends {
                                 pool_id: pool.pool_id.clone(),
                                 contributor: self.current_user.clone()
                             });
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Stake:");
                            ui.text_edit_singleline(&mut self.join_amount); 
                            if ui.button("Join Pool").clicked() {
                                if let Ok(amt) = self.join_amount.parse::<u64>() {
                                    let _ = self.rpc_tx.try_send(RpcCommand::JoinPool {
                                        pool_id: pool.pool_id.clone(),
                                        contributor: self.current_user.clone(),
                                        amount: amt
                                    });
                                }
                            }
                        });
                    }
                });
            }
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