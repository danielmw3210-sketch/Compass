use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use crate::chain::Chain;
use crate::layer3::compute::{ComputeJob, ComputeJobStatus};
use crate::layer3::paper_trading::TradingPortfolio;
use crate::network::{NetworkCommand, NetMessage};

/// OracleScheduler fetches live prices and creates ComputeJobs for inference
pub struct OracleScheduler {
    pub chain: Arc<Mutex<Chain>>,
    pub creator_pubkey: String,
    pub cmd_tx: mpsc::Sender<NetworkCommand>,
    pub client: reqwest::Client,
    pub paper_portfolio: Arc<Mutex<TradingPortfolio>>,  // Paper trading portfolio
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
        
        // Initialize paper trading portfolio (Load from DB or Default)
        let paper_portfolio = {
            let locked_chain = chain.lock().unwrap();
            match locked_chain.storage.get_portfolio() {
                Ok(Some(p)) => Arc::new(Mutex::new(p)),
                _ => Arc::new(Mutex::new(TradingPortfolio::new(10000.0)))
            }
        };
        
        Self { chain, creator_pubkey, cmd_tx, client, paper_portfolio }
    }

    pub async fn start(self) {
        info!("🔮 Oracle Scheduler Started (BTC, ETH, SOL, LTC)");
        info!("📊 Multi-Timeframe Forecasting: 5m, 30m, 1h, 3h, 6h, 24h");
        info!("💰 Paper Trading: $10,000 starting capital");
        
        // Wait for node to warm up
        tokio::time::sleep(Duration::from_secs(5)).await;

        let tickers = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];
        let mut tick_counter: u64 = 0;

        loop {
            tick_counter += 1;
            
            // 1. Create Multi-Timeframe Predictions (every 30 seconds)
            info!("🔮 Creating multi-timeframe predictions...");
            for ticker in &tickers {
                self.create_multi_timeframe_predictions(ticker).await;
            }
            
            // 2. Verify Ready Predictions (every minute)
            if tick_counter % 2 == 0 {
                info!("✅ Checking predictions ready for verification...");
                self.verify_ready_predictions().await;
                
                // Display portfolio summary every 10 cycles (~5 minutes)
                if tick_counter % 10 == 0 {
                    if let Ok(portfolio) = self.paper_portfolio.lock() {
                        info!("\n{}", portfolio.get_summary());
                    }
                }
            }
            
            // 3. Oracle Compute Jobs (original functionality - every 30 seconds)
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
            
            // Wait 30 seconds before next cycle
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
        let mut epoch_state = storage.get_epoch_state("admin", ticker, model_id)
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

    /// Fetches historical market data (OHLCV) for enhanced signal model
    /// Returns Vec<[Open, High, Low, Close, Volume]>
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
                    // Extract full OHLCV: [[Open, High, Low, Close, Volume], ...]
                    let sequence: Vec<Vec<f64>> = data.iter().filter_map(|kline| {
                        // kline format: [time, open, high, low, close, volume, ...]
                        let open = kline.get(1)?.as_str()?.parse::<f64>().ok()?;
                        let high = kline.get(2)?.as_str()?.parse::<f64>().ok()?;
                        let low = kline.get(3)?.as_str()?.parse::<f64>().ok()?;
                        let close = kline.get(4)?.as_str()?.parse::<f64>().ok()?;
                        let volume = kline.get(5)?.as_str()?.parse::<f64>().ok()?;
                        Some(vec![open, high, low, close, volume])
                    }).collect();
                    
                    if sequence.len() >= 200 {
                        info!("   📈 Fetched {}-candle OHLCV sequence for {} (Last Close: ${:.2})", 
                            sequence.len(), symbol, sequence.last()?[3]);
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
    
    /// Create predictions for all timeframes for a given ticker
    pub async fn create_multi_timeframe_predictions(&self, ticker: &str) {
        use crate::layer3::price_oracle::{PredictionRecord, PredictionTimeframe, TradingSignal};
        
        let timeframes = PredictionTimeframe::all();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Fetch OHLCV data
        let sequence = match self.fetch_historical_sequence(ticker).await {
            Some(seq) => seq,
            None => {
                warn!("Failed to fetch data for {}, skipping predictions", ticker);
                return;
            }
        };
        
        for timeframe in timeframes {
            // Get model ID for this timeframe
            let ticker_short = ticker.to_lowercase().replace("usdt", "");
            let model_id = format!("signal_{}_{}", ticker_short, timeframe.model_suffix());
            
            // TODO: Run inference with timeframe-specific model
            // For now, use simple price-based prediction
            let last_prices: Vec<f64> = sequence.iter().map(|row| row[3]).collect();
            let current_price = last_prices.last().copied().unwrap_or(0.0);
            
            // Simple momentum-based prediction (placeholder until models trained)
            let recent_change = if last_prices.len() >= 10 {
                (last_prices[last_prices.len()-1] - last_prices[last_prices.len()-10]) / last_prices[last_prices.len()-10]
            } else {
                0.0
            };
            
            let predicted_signal = if recent_change > 0.0005 {
                TradingSignal::Buy
            } else if recent_change < -0.0005 {
                TradingSignal::Sell
            } else {
                TradingSignal::Hold
            };
            
            let predicted_price = current_price * (1.0 + recent_change * timeframe.candles() as f64 / 10.0);
            
            // Get current epoch for this ticker/timeframe combination
            let epoch_key = format!("{}_{}", ticker, timeframe.model_suffix());
            let current_epoch = {
                let chain = self.chain.lock().unwrap();
                chain.storage
                    .get_epoch_state("admin", ticker, &model_id) // Use "admin" for system/oracle predictions
                    .ok()
                    .flatten()
                    .map(|s| s.current_epoch)
                    .unwrap_or(1)
            };
            
            // Create prediction record
            let prediction = PredictionRecord::new(
                ticker,
                &model_id,
                predicted_price,
                predicted_signal.clone(),  // Clone to avoid move
                0.75, // Confidence
                current_epoch,
                timeframe.clone(),
            ).with_owner("admin"); // System predictions owned by admin
            
            // Save prediction
            if let Ok(chain) = self.chain.lock() {
                match chain.storage.save_prediction(&prediction) {
                    Ok(_) => {
                        info!(
                            "📊 {} {} Prediction: ${:.2} ({:?}) → Verify at {}",
                            ticker,
                            timeframe.display(),
                            predicted_price,
                            predicted_signal,
                            chrono::DateTime::from_timestamp(prediction.target_timestamp as i64, 0)
                                .map(|dt| dt.format("%H:%M").to_string())
                                .unwrap_or_else(|| "??:??".to_string())
                        );
                        
                        // Execute paper trade based on signal
                        if predicted_signal != crate::layer3::price_oracle::TradingSignal::Hold {
                            use crate::layer3::paper_trading::{PaperTrade, TradingSignal as PTSignal};
                            
                            let pt_signal = match predicted_signal {
                                crate::layer3::price_oracle::TradingSignal::Buy => PTSignal::Buy,
                                crate::layer3::price_oracle::TradingSignal::Sell => PTSignal::Sell,
                                _ => PTSignal::Hold,
                            };
                            
                            let trade = PaperTrade::new(
                                ticker,
                                &model_id,
                                timeframe.display(),
                                pt_signal,
                                current_price,
                                1000.0,  // $1000 position size
                                &prediction.id,
                            );
                            
                            // Lock Portfolio and update (No re-locking chain, use existing guard)
                            if let Ok(mut portfolio) = self.paper_portfolio.lock() {
                                match portfolio.open_trade(trade) {
                                    Ok(_) => {
                                        info!("💰 Opened {} paper trade: {} @ ${}", 
                                            if predicted_signal == crate::layer3::price_oracle::TradingSignal::Buy { "LONG" } else { "SHORT" },
                                            ticker, current_price);
                                            
                                        // Save to DB using the ALREADY LOCKED 'chain' guard
                                        let _ = chain.storage.save_portfolio(&portfolio);
                                    }
                                    Err(e) => {
                                        // Silent warning for 'already open' to reduce log spam
                                        tracing::debug!("Skipped paper trade: {}", e);
                                    }
                                }
                            }
                            
                            // SAVE LATEST SIGNAL FOR BOT
                            // Use existing 'chain' lock
                            let key = format!("latest_signal:{}", ticker);
                            let signal_data = serde_json::json!({
                                "ticker": ticker,
                                "signal": match predicted_signal {
                                    crate::layer3::price_oracle::TradingSignal::Buy => "BUY",
                                    crate::layer3::price_oracle::TradingSignal::Sell => "SELL",
                                    _ => "HOLD"
                                },
                                "price": predicted_price,
                                "timestamp": current_time,
                                "model_id": model_id,
                                "confidence": 0.75
                            });
                            let _ = chain.storage.put(&key, &signal_data); // Use generic Put
                        }
                    }
                    Err(e) => error!("Failed to save {} {} prediction: {}", ticker, timeframe.display(), e),
                }
            }
        }
    }
    
    /// Verify predictions that have reached their target timestamp
    pub async fn verify_ready_predictions(&self) {
        use crate::layer3::price_oracle::PriceOracle;
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Get all pending predictions
        // TODO: Implement get_all_predictions in storage
        // For now, we'll rely on predictions being saved and verified via RPC handlers
        let pending_predictions: Vec<crate::layer3::price_oracle::PredictionRecord> = vec![]; 
        
        /*
        let pending_predictions = {
            let chain = self.chain.lock().unwrap();
            chain.storage.get_all_predictions().unwrap_or_default()
        };
        */
        
        for mut pred in pending_predictions {
            // Skip if already verified
            if pred.verification_time.is_some() {
                continue;
            }
            
            // Check if target time has been reached
            if pred.target_timestamp <= current_time {
                // Fetch actual price
                match PriceOracle::fetch_binance_price(&pred.ticker).await {
                    Ok(actual_price) => {
                        let ticker = pred.ticker.clone();
                        let timeframe_display = pred.timeframe.display();
                        let pred_price = pred.predicted_price;
                        let pred_signal = pred.predicted_signal.clone();
                        
                        pred.verify(actual_price);
                        
                        let is_correct = pred.is_correct.unwrap_or(false);
                        let actual_signal = pred.actual_signal.clone().unwrap();
                        
                        info!(
                            "✅ {} {} Verified: Pred=${:.2} Act=${:.2} ({}) Signal: {:?}→{:?}",
                            ticker,
                            timeframe_display,
                            pred_price,
                            actual_price,
                            if is_correct { "✅ CORRECT" } else { "❌ WRONG" },
                            pred_signal,
                            actual_signal
                        );
                        
                        // Close paper trade
                        if let Ok(mut portfolio) = self.paper_portfolio.lock() {
                            match portfolio.close_trade(&ticker, actual_price) {
                                Ok(_) => {
                                    if let Some(last_trade) = portfolio.closed_trades.last() {
                                        let pnl = last_trade.pnl.unwrap_or(0.0);
                                        let pnl_pct = last_trade.pnl_percentage.unwrap_or(0.0);
                                        info!(
                                            "💵 Closed {} trade: P&L = ${:.2} ({:.1}%) | Balance: ${:.2}",
                                            ticker, pnl, pnl_pct, portfolio.current_balance
                                        );
                                    }
                                    
                                    // Save to DB
                                    if let Ok(chain) = self.chain.lock() {
                                        let _ = chain.storage.save_portfolio(&portfolio);
                                    }
                                }
                                Err(e) => warn!("Failed to close paper trade: {}", e),
                            }
                        }
                        
                        // Save updated prediction & update epoch
                        if let Ok(chain) = self.chain.lock() {
                            let _ = chain.storage.save_prediction(&pred);
                            
                            // Update epoch state for this ticker/timeframe (Namespaced by Owner)
                            let epoch_key = format!("{}_{}", pred.ticker, pred.timeframe.model_suffix());
                            
                            // Load state for the owner of the prediction
                            let mut epoch_state = chain.storage
                                .get_epoch_state(&pred.owner, &pred.ticker, &epoch_key)
                                .unwrap_or(None)
                                .unwrap_or_else(|| {
                                    // If new, create fresh state
                                    crate::layer3::price_oracle::ModelEpochState::new(
                                        &epoch_key,
                                        &pred.ticker,
                                        crate::layer3::price_oracle::EpochConfig::default()
                                    ).with_owner(&pred.owner)
                                });

                            epoch_state.record_prediction(is_correct);
                            
                            // Check for NFT Minting condition
                            if epoch_state.should_mint() && !epoch_state.nft_minted {
                                epoch_state.nft_minted = true;
                                info!("🎉 Model {} Qualified for NFT Minting! (Epoch {}, {:.1}%) [Owner: {}]", 
                                    pred.model_id, epoch_state.epochs_completed, epoch_state.overall_accuracy() * 100.0, pred.owner);
                            }
                            
                            if let Err(e) = chain.storage.save_epoch_state(&epoch_state) {
                                error!("Failed to save epoch state for {}: {}", pred.model_id, e);
                            } else {
                                info!("✅ Saved epoch state: {}:{} (Epoch {}/{}, {:.1}%) [Owner: {}]", 
                                    pred.ticker, pred.model_id, epoch_state.current_epoch, 
                                    epoch_state.epochs_completed, epoch_state.overall_accuracy() * 100.0, pred.owner);
                            }
                        }
                    }
                    Err(e) => warn!("Failed to verify {} {}: {}", pred.ticker, pred.timeframe.display(), e),
                }
            }
        }
    }
}
