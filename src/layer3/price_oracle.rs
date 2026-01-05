//! Price Oracle Module
//! Fetches and stores real-time prices from external sources (Binance, etc.)

use serde::{Deserialize, Serialize};

/// Supported trading pairs
pub const SUPPORTED_TICKERS: &[&str] = &["BTCUSDT", "SOLUSDT", "LTCUSDT", "ETHUSDT"];

/// Price data from an oracle source
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PricePoint {
    pub ticker: String,
    pub price: f64,
    pub timestamp: u64,
    pub source: String,
}

/// Live price oracle for a specific asset
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PriceOracle {
    pub ticker: String,
    pub latest_price: f64,
    pub prices_24h: Vec<PricePoint>, // Rolling 24h history
    pub last_updated: u64,
}

impl PriceOracle {
    pub fn new(ticker: &str) -> Self {
        Self {
            ticker: ticker.to_string(),
            latest_price: 0.0,
            prices_24h: Vec::new(),
            last_updated: 0,
        }
    }

    /// Fetch current price from Kraken (US-friendly, Binance is geo-blocked)
    pub async fn fetch_binance_price(ticker: &str) -> Result<f64, String> {
        // Convert ticker format: BTCUSDT -> XXBTZUSD, ETHUSDT -> XETHZUSD
        let kraken_pair = match ticker {
            "BTCUSDT" => "XXBTZUSD",
            "ETHUSDT" => "XETHZUSD",
            "SOLUSDT" => "SOLUSD",
            "LTCUSDT" => "XLTCZUSD",
            _ => return Err(format!("Unsupported ticker: {}", ticker)),
        };
        
        let url = format!(
            "https://api.kraken.com/0/public/Ticker?pair={}",
            kraken_pair
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;
        
        // Check for errors
        if let Some(errors) = json.get("error").and_then(|e| e.as_array()) {
            if !errors.is_empty() {
                return Err(format!("Kraken API error: {:?}", errors));
            }
        }
        
        // Extract price from result
        // Kraken returns: {"result": {"XXBTZUSD": {"c": ["97234.00000", "0.01"]}}}  
        // "c" is the last trade closed [price, volume]
        let result = json.get("result")
            .ok_or("No result field")?;
        
        let ticker_data = result.as_object()
            .and_then(|obj| obj.values().next())
            .ok_or("No ticker data")?;
        
        let price_str = ticker_data.get("c")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|p| p.as_str())
            .ok_or("No price in response")?;
        
        price_str.parse::<f64>()
            .map_err(|e| format!("Price parse error: {}", e))
    }

    /// Fetch all supported prices
    pub async fn fetch_all_prices() -> Vec<(String, Result<f64, String>)> {
        let mut results = Vec::new();
        for ticker in SUPPORTED_TICKERS {
            let price = Self::fetch_binance_price(ticker).await;
            results.push((ticker.to_string(), price));
        }
        results
    }

    /// Update this oracle with a new price
    pub fn update(&mut self, price: f64, source: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.latest_price = price;
        self.last_updated = now;

        // Add to 24h history
        self.prices_24h.push(PricePoint {
            ticker: self.ticker.clone(),
            price,
            timestamp: now,
            source: source.to_string(),
        });

        // Prune old data (keep last 24h = 86400 seconds)
        let cutoff = now.saturating_sub(86400);
        self.prices_24h.retain(|p| p.timestamp > cutoff);
    }

    /// Calculate 24h price change percentage
    pub fn price_change_24h(&self) -> Option<f64> {
        if self.prices_24h.len() < 2 {
            return None;
        }
        let oldest = self.prices_24h.first()?.price;
        let newest = self.latest_price;
        Some(((newest - oldest) / oldest) * 100.0)
    }
}

/// Binance API response
#[derive(Deserialize)]
struct BinanceTickerResponse {
    symbol: String,
    price: String,
}

// ============================================================
// PREDICTION TRACKING
// ============================================================

/// Trading signal classification
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TradingSignal {
    Buy,
    Sell,
    Hold,
}

impl TradingSignal {
    pub fn from_price_change(change_pct: f64) -> Self {
        if change_pct > 2.0 {
            TradingSignal::Buy
        } else if change_pct < -2.0 {
            TradingSignal::Sell
        } else {
            TradingSignal::Hold
        }
    }
}

/// Prediction timeframe for multi-horizon forecasting
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PredictionTimeframe {
    FiveMinutes,
    ThirtyMinutes,
    OneHour,
    ThreeHours,
    SixHours,
    TwentyFourHours,
}

impl PredictionTimeframe {
    /// Get duration in seconds
    pub fn seconds(&self) -> u64 {
        match self {
            Self::FiveMinutes => 300,
            Self::ThirtyMinutes => 1800,
            Self::OneHour => 3600,
            Self::ThreeHours => 10800,
            Self::SixHours => 21600,
            Self::TwentyFourHours => 86400,
        }
    }
    
    /// Get number of 5-minute candles
    pub fn candles(&self) -> usize {
        match self {
            Self::FiveMinutes => 1,
            Self::ThirtyMinutes => 6,
            Self::OneHour => 12,
            Self::ThreeHours => 36,
            Self::SixHours => 72,
            Self::TwentyFourHours => 288,
        }
    }
    
    /// Get model suffix for file naming
    pub fn model_suffix(&self) -> &'static str {
        match self {
            Self::FiveMinutes => "5m",
            Self::ThirtyMinutes => "30m",
            Self::OneHour => "1h",
            Self::ThreeHours => "3h",
            Self::SixHours => "6h",
            Self::TwentyFourHours => "24h",
        }
    }
    
    /// Display name
    pub fn display(&self) -> &'static str {
        match self {
            Self::FiveMinutes => "5min",
            Self::ThirtyMinutes => "30min",
            Self::OneHour => "1hr",
            Self::ThreeHours => "3hr",
            Self::SixHours => "6hr",
            Self::TwentyFourHours => "24hr",
        }
    }
    
    /// All supported timeframes
    pub fn all() -> Vec<Self> {
        vec![
            Self::FiveMinutes,
            Self::ThirtyMinutes,
            Self::OneHour,
            Self::ThreeHours,
            Self::SixHours,
            Self::TwentyFourHours,
        ]
    }
}

/// A single price/signal prediction with verification tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PredictionRecord {
    pub id: String,
    pub ticker: String,
    #[serde(default = "default_owner")]
    pub owner: String, // NEW: Owner of the model making this prediction
    pub model_id: String,
    
    // Prediction data
    pub predicted_price: f64,
    pub predicted_signal: TradingSignal,
    pub confidence: f64,
    pub prediction_time: u64,
    
    // Multi-timeframe support
    pub timeframe: PredictionTimeframe,
    pub target_timestamp: u64,  // When to verify (prediction_time + timeframe.seconds())
    
    // Verification data (filled in later)
    pub actual_price: Option<f64>,
    pub actual_signal: Option<TradingSignal>,
    pub is_correct: Option<bool>,
    pub verification_time: Option<u64>,
    
    // Epoch tracking
    pub epoch: u32,
}

impl PredictionRecord {
    pub fn new(
        ticker: &str,
        model_id: &str,
        predicted_price: f64,
        predicted_signal: TradingSignal,
        confidence: f64,
        epoch: u32,
        timeframe: PredictionTimeframe,  // NEW parameter
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let target_timestamp = now + timeframe.seconds();
        let id = format!("PRED_{}_{}_{}", ticker, timeframe.model_suffix(), now);
        
        Self {
            id,
            ticker: ticker.to_string(),
            owner: "admin".to_string(), // Default to admin for legacy/system predictions
            model_id: model_id.to_string(),
            predicted_price,
            predicted_signal,
            confidence,
            prediction_time: now,
            timeframe,
            target_timestamp,
            actual_price: None,
            actual_signal: None,
            is_correct: None,
            verification_time: None,
            epoch,
        }
    }

    pub fn with_owner(mut self, owner: &str) -> Self {
        self.owner = owner.to_string();
        self
    }

    /// Verify this prediction against actual price
    pub fn verify(&mut self, actual_price: f64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate actual signal based on price change
        let change_pct = ((actual_price - self.predicted_price) / self.predicted_price) * 100.0;
        let actual_signal = TradingSignal::from_price_change(change_pct);

        // Determine if prediction was correct
        // For price: within 1% is considered correct
        // For signal: exact match
        let price_error = ((actual_price - self.predicted_price) / self.predicted_price).abs();
        let _price_correct = price_error < 0.01; // 1% tolerance
        let signal_correct = self.predicted_signal == actual_signal;
        
        // Overall correctness: signal must match (more important)
        let is_correct = signal_correct;

        self.actual_price = Some(actual_price);
        self.actual_signal = Some(actual_signal);
        self.is_correct = Some(is_correct);
        self.verification_time = Some(now);
    }
}

// ============================================================
// EPOCH TRACKING
// ============================================================

/// Configuration for epoch-based training and minting
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EpochConfig {
    pub predictions_per_epoch: u32,  // Default: 10
    pub mint_at_epoch: Option<u32>,  // e.g., mint at epoch 10
    pub min_accuracy_to_mint: f64,   // e.g., 0.75 (75%)
    pub verification_delay_secs: u64, // Default: 300 (5 minutes)
}

impl Default for EpochConfig {
    fn default() -> Self {
        Self {
            predictions_per_epoch: 10,
            mint_at_epoch: Some(10),
            min_accuracy_to_mint: 0.75,
            verification_delay_secs: 300, // 5 minutes
        }
    }
}

/// Tracks epoch progress for a model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelEpochState {
    pub model_id: String,
    pub ticker: String,
    #[serde(default = "default_owner")]
    pub owner: String, // NEW: Owner account
    pub config: EpochConfig,
    
    // Current epoch stats
    pub current_epoch: u32,
    pub predictions_in_epoch: u32,
    pub correct_in_epoch: u32,
    
    // Overall stats
    pub total_predictions: u32,
    pub total_correct: u32,
    pub epochs_completed: u32,
    
    // History
    pub epoch_accuracies: Vec<f64>, // Accuracy per completed epoch
    
    // Minting status
    pub nft_minted: bool,
    pub nft_token_id: Option<String>,
}

fn default_owner() -> String {
    "admin".to_string()
}

impl ModelEpochState {
    pub fn new(model_id: &str, ticker: &str, config: EpochConfig) -> Self {
        Self {
            model_id: model_id.to_string(),
            ticker: ticker.to_string(),
            owner: "admin".to_string(), // Default
            config,
            current_epoch: 1,
            predictions_in_epoch: 0,
            correct_in_epoch: 0,
            total_predictions: 0,
            total_correct: 0,
            epochs_completed: 0,
            epoch_accuracies: Vec::new(),
            nft_minted: false,
            nft_token_id: None,
        }
    }

    pub fn with_owner(mut self, owner: &str) -> Self {
        self.owner = owner.to_string();
        self
    }

    /// Record a verified prediction
    pub fn record_prediction(&mut self, is_correct: bool) {
        self.predictions_in_epoch += 1;
        self.total_predictions += 1;
        
        if is_correct {
            self.correct_in_epoch += 1;
            self.total_correct += 1;
        }

        // Check if epoch is complete
        if self.predictions_in_epoch >= self.config.predictions_per_epoch {
            self.complete_epoch();
        }
    }

    /// Complete current epoch and move to next
    fn complete_epoch(&mut self) {
        let accuracy = if self.predictions_in_epoch > 0 {
            self.correct_in_epoch as f64 / self.predictions_in_epoch as f64
        } else {
            0.0
        };

        self.epoch_accuracies.push(accuracy);
        self.epochs_completed += 1;
        self.current_epoch += 1;
        self.predictions_in_epoch = 0;
        self.correct_in_epoch = 0;

        println!(
            "📊 [Epoch {}] Model {} completed with {:.1}% accuracy",
            self.epochs_completed,
            self.model_id,
            accuracy * 100.0
        );
    }

    /// Check if conditions are met to mint an NFT
    pub fn should_mint(&self) -> bool {
        if self.nft_minted {
            return false; // Already minted
        }

        if let Some(mint_epoch) = self.config.mint_at_epoch {
            if self.epochs_completed >= mint_epoch {
                let overall_accuracy = self.overall_accuracy();
                return overall_accuracy >= self.config.min_accuracy_to_mint;
            }
        }

        false
    }

    /// Calculate overall accuracy
    pub fn overall_accuracy(&self) -> f64 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        self.total_correct as f64 / self.total_predictions as f64
    }

    /// Get current epoch progress (0.0 to 1.0)
    pub fn epoch_progress(&self) -> f64 {
        self.predictions_in_epoch as f64 / self.config.predictions_per_epoch as f64
    }
}
