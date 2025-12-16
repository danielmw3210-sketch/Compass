use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use crate::chain::Chain;
use crate::layer3::compute::{ComputeJob, ComputeJobStatus};
use crate::network::{NetworkCommand, NetMessage};

/// OracleScheduler fetches live prices and creates ComputeJobs for inference
pub struct OracleScheduler {
    pub chain: Arc<Mutex<Chain>>,
    pub creator_pubkey: String,
    pub cmd_tx: mpsc::Sender<NetworkCommand>,
    pub client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize)]
struct BinanceTicker {
    symbol: String,
    price: String,
}

impl OracleScheduler {
    pub fn new(chain: Arc<Mutex<Chain>>, creator_pubkey: String, cmd_tx: mpsc::Sender<NetworkCommand>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { chain, creator_pubkey, cmd_tx, client }
    }

    pub async fn start(self) {
        info!("🔮 Oracle Scheduler Started (BTC, ETH, SOL, LTC)");
        
        // Wait for node to warm up
        tokio::time::sleep(Duration::from_secs(5)).await;

        let tickers = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];

        loop {
            // 1. Oracle Job Loop
            for ticker in &tickers {
                // Fetch Sequence (30 steps) for LSTM
                if let Some(sequence) = self.fetch_historical_sequence(ticker).await {
                    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let job_id = format!("ORACLE_{}_{}", ticker, timestamp);
                    
                    // Serialize sequence as input
                    let input_data = serde_json::to_vec(&sequence).unwrap_or_default();
                    
                    let job = ComputeJob {
                        job_id: job_id.clone(),
                        creator: self.creator_pubkey.clone(),
                        model_id: format!("signal_{}_v2", ticker.to_lowercase().replace("usdt", "")),
                        max_compute_units: 200, // Heavier inference
                        reward_amount: 100, // Higher reward
                        status: ComputeJobStatus::Pending,
                        worker_id: None,
                        result_hash: None,
                        verifiers: Vec::new(),
                        verification_status: ComputeJobStatus::Pending,
                        timestamp,
                        completed_at: None,
                        compute_rate: 0,
                        started_at: None,
                        min_duration: 1, // Fast task
                        inputs: input_data,
                    };
                    
                    // Save to Chain
                    let mut saved = false;
                    if let Ok(c_guard) = self.chain.lock() {
                        if let Err(e) = c_guard.storage.save_compute_job(&job) {
                            error!("Failed to save oracle job: {}", e);
                        } else {
                            info!("   ✨ Created Oracle Job: {} (Reward: 100 COMPUTE)", job_id);
                            saved = true;
                        }
                    }

                    // Broadcast to Network
                    if saved {
                        let msg = NetMessage::ComputeJob(job);
                        let _ = self.cmd_tx.send(NetworkCommand::Broadcast(msg)).await;
                    }
                }
            }
            
            // Stagger Oracle requests
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 2. Self-Learning Loop
            // Every 60 minutes (approx 60 iterations), issue a Training Job to update the model
            static mut TICK_COUNTER: u64 = 0;
            unsafe {
                TICK_COUNTER += 1;
                // Changed for rapid testing: Every 2 ticks (approx 1 min)
                if TICK_COUNTER % 2 == 0 {
                    let tickers = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];
                    
                    for ticker in tickers {
                        let ticker_short = ticker.replace("USDT", "").to_lowercase();
                        info!("🧠 Self-Learning: Initiating {}-AI Training Job...", ticker_short.to_uppercase());
                        
                        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        let job_id = format!("TRAIN_{}_{}", ticker_short.to_uppercase(), timestamp);
                        
                        // We encode the config in the inputs or just use model_id convention
                        let config = serde_json::json!({
                            "ticker": ticker,
                            "epochs": 100 // Short bursts of training
                        });
                        
                        let job = ComputeJob {
                            job_id: job_id.clone(),
                            creator: self.creator_pubkey.clone(),
                            model_id: format!("train_{}_v1", ticker_short), // Dynamic Model ID
                            max_compute_units: 5000, 
                            reward_amount: 500, 
                            status: ComputeJobStatus::Pending,
                            worker_id: None,
                            result_hash: None,
                            verifiers: Vec::new(),
                            verification_status: ComputeJobStatus::Pending,
                            timestamp,
                            completed_at: None,
                            compute_rate: 0,
                            started_at: None,
                            min_duration: 60, 
                            inputs: serde_json::to_vec(&config).unwrap_or_default(), 
                        };

                        let mut saved = false;
                        if let Ok(c_guard) = self.chain.lock() {
                            if let Err(e) = c_guard.storage.save_compute_job(&job) {
                                error!("Failed to save training job: {}", e);
                            } else {
                                info!("   🎓 Created {} Training Job: {} (Reward: 500 COMPUTE)", ticker_short.to_uppercase(), job_id);
                                saved = true;
                            }
                        }

                        // Broadcast to Network
                        if saved {
                            let msg = NetMessage::ComputeJob(job);
                            let _ = self.cmd_tx.send(NetworkCommand::Broadcast(msg)).await;
                            info!("   📡 Broadcasted {} Training Job to P2P Network", ticker_short.to_uppercase());
                        }
                    }
                }
                
                // 3. Prediction Verification Loop (every tick)
                self.verify_pending_predictions().await;
            }
        }
    }

    /// Verify predictions that are old enough (5 minutes by default)
    async fn verify_pending_predictions(&self) {
        use crate::layer3::price_oracle::{PriceOracle, TradingSignal, EpochConfig, ModelEpochState};
        
        // Get pending predictions
        let pending = {
            if let Ok(chain) = self.chain.lock() {
                chain.storage.get_pending_verifications(300).unwrap_or_default() // 5 min delay
            } else {
                return;
            }
        };
        
        if pending.is_empty() {
            return;
        }
        
        info!("🔍 Verifying {} pending predictions...", pending.len());
        
        for mut pred in pending {
            // Fetch actual price from Binance
            match PriceOracle::fetch_binance_price(&pred.ticker).await {
                Ok(actual_price) => {
                    // Verify the prediction
                    pred.verify(actual_price);
                    
                    let is_correct = pred.is_correct.unwrap_or(false);
                    let icon = if is_correct { "✅" } else { "❌" };
                    
                    info!(
                        "{} Prediction {}: Predicted ${:.2} ({:?}) → Actual ${:.2} ({:?})",
                        icon,
                        pred.id,
                        pred.predicted_price,
                        pred.predicted_signal,
                        actual_price,
                        pred.actual_signal
                    );
                    
                    // Save updated prediction
                    if let Ok(chain) = self.chain.lock() {
                        let _ = chain.storage.save_prediction(&pred);
                        
                        // Update epoch state
                        self.update_epoch_state(&chain.storage, &pred.ticker, &pred.model_id, is_correct);
                    }
                }
                Err(e) => {
                    warn!("Failed to verify prediction {}: {}", pred.id, e);
                }
            }
        }
    }
    
    /// Update epoch state after a prediction is verified
    fn update_epoch_state(&self, storage: &crate::storage::Storage, ticker: &str, model_id: &str, is_correct: bool) {
        use crate::layer3::price_oracle::{EpochConfig, ModelEpochState};
        
        // DIAGNOSTIC: Confirm this function is being called
        info!("🔧 UPDATE_EPOCH_STATE CALLED: {}:{} (correct={})", ticker, model_id, is_correct);
        
        // Get or create epoch state
        let mut epoch_state = storage.get_epoch_state(ticker, model_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                info!("📊 Creating new epoch state for {}:{}", ticker, model_id);
                ModelEpochState::new(model_id, ticker, EpochConfig::default())
            });
        
        // Record the prediction result
        epoch_state.record_prediction(is_correct);
        
        // Check if we should mint an NFT
        if epoch_state.should_mint() {
            info!(
                "🎉 READY TO MINT NFT! {}:{} (Epoch {}, {:.1}% accuracy)",
                ticker,
                model_id,
                epoch_state.epochs_completed,
                epoch_state.overall_accuracy() * 100.0
            );
            info!("   💡 TIP: Wait for higher epochs & accuracy to increase NFT value!");
            info!("   💰 Current estimated value: {} COMPASS", 
                (1000 + (epoch_state.overall_accuracy() * 1000.0) as u64 + (epoch_state.epochs_completed as u64 * 50))
            );
        }
        
        // DIAGNOSTIC: Log what we're about to save
        info!(
            "🔧 ATTEMPTING SAVE: {}:{} - Epoch {}, {} predictions, {:.1}% accuracy",
            ticker,
            model_id,
            epoch_state.current_epoch,
            epoch_state.total_predictions,
            epoch_state.overall_accuracy() * 100.0
        );
        
        // Save epoch state with retry logic
        let mut retries = 3;
        let mut last_error = None;
        
        while retries > 0 {
            match storage.save_epoch_state(&epoch_state) {
                Ok(_) => {
                    info!(
                        "✅ Saved epoch state: {}:{} (Epoch {}/{}, {}/{} correct, {:.1}% accuracy)",
                        ticker,
                        model_id,
                        epoch_state.current_epoch,
                        epoch_state.epochs_completed,
                        epoch_state.total_correct,
                        epoch_state.total_predictions,
                        epoch_state.overall_accuracy() * 100.0
                    );
                    return; // Success!
                }
                Err(e) => {
                    last_error = Some(e);
                    retries -= 1;
                    
                    if retries > 0 {
                        warn!(
                            "⚠️ Failed to save epoch state for {}:{}, retrying... ({} attempts left)",
                            ticker,
                            model_id,
                            retries
                        );
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
        
        // If we got here, all retries failed
        if let Some(e) = last_error {
            error!(
                "❌ CRITICAL: Failed to save epoch state after 3 attempts: {}",
                e
            );
            error!(
                "   Lost epoch data: {}:{} (Epoch {}, {:.1}% accuracy, {} predictions)",
                ticker,
                model_id,
                epoch_state.current_epoch,
                epoch_state.overall_accuracy() * 100.0,
                epoch_state.total_predictions
            );
        }
    }

    /// Fetches historical market data (30 sequence steps) for LSTM input
    async fn fetch_historical_sequence(&self, symbol: &str) -> Option<Vec<Vec<f64>>> {
        let url = "https://api.binance.com/api/v3/klines";
        // Fetch 1000 5m candles for Signal Model (needs 200+ for SMA200)
        let params = [
            ("symbol", symbol),
            ("interval", "5m"),
            ("limit", "1000"), 
        ];

        match self.client.get(url).query(&params).send().await {
            Ok(resp) => match resp.json::<Vec<serde_json::Value>>().await {
                Ok(data) => {
                    // Extract [[Close, Volume], [Close, Volume], ...]
                    let sequence: Vec<Vec<f64>> = data.iter().filter_map(|kline| {
                        // kline is [time, open, high, low, close, volume, ...]
                        // Close = index 4, Volume = index 5
                        let close = kline.get(4)?.as_str()?.parse::<f64>().ok()?;
                        let volume = kline.get(5)?.as_str()?.parse::<f64>().ok()?;
                        Some(vec![close, volume])
                    }).collect();
                    
                    if sequence.len() >= 200 {
                        info!("   📈 Fetched {}-step sequence for {} (Last: ${:.2})", sequence.len(), symbol, sequence.last()?[0]);
                        Some(sequence)
                    } else {
                        warn!("   ⚠️ Insufficient data for {}: Got {}/200+", symbol, sequence.len());
                        None
                    }
                }
                Err(e) => {
                    error!("   ❌ Failed to parse sequence data: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("   ❌ Failed to fetch sequence data: {}", e);
                None
            }
        }
    }
}
