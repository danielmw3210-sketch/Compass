//! Signal Model Training - Per-Asset BUY/SELL/HOLD Classification
//! 
//! This module trains classification models for each supported asset (BTC, SOL, LTC, ETH)
//! that predict whether to BUY, SELL, or HOLD based on price patterns.

use std::error::Error;
use reqwest::Client;
use std::fs::File;
use std::io::Write;
use smartcore::ensemble::random_forest_classifier::RandomForestClassifier;
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::ensemble::random_forest_classifier::RandomForestClassifierParameters;

/// Trading signal classification labels
/// Trading signal classification labels (Used in training thresholds)
pub const LABEL_SELL: f64 = 0.0;
pub const LABEL_HOLD: f64 = 1.0;
pub const LABEL_BUY: f64 = 2.0;

/// Supported tickers for signal models
pub const SIGNAL_TICKERS: [&str; 4] = ["BTCUSDT", "SOLUSDT", "LTCUSDT", "ETHUSDT"];

// --- Technical Indicator Helpers ---

fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
    let mut sma = vec![0.0; prices.len()];
    for i in period..prices.len() {
        let sum: f64 = prices[i - period..i].iter().sum();
        sma[i] = sum / period as f64;
    }
    sma
}

fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
    let k = 2.0 / (period as f64 + 1.0);
    let mut ema = vec![0.0; prices.len()];
    
    // Initial SMA for first EMA point
    if prices.len() > period {
        let sma_start = prices[0..period].iter().sum::<f64>() / period as f64;
        ema[period - 1] = sma_start;
        
        for i in period..prices.len() {
            ema[i] = (prices[i] - ema[i - 1]) * k + ema[i - 1];
        }
    }
    ema
}

fn calculate_macd(prices: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    // Standard MACD (12, 26, 9)
    let ema12 = calculate_ema(prices, 12);
    let ema26 = calculate_ema(prices, 26);
    
    let mut macd_line = vec![0.0; prices.len()];
    for i in 0..prices.len() {
        if ema26[i] != 0.0 {
            macd_line[i] = ema12[i] - ema26[i];
        }
    }
    
    let signal_line = calculate_ema(&macd_line, 9);
    
    let mut histogram = vec![0.0; prices.len()];
    for i in 0..prices.len() {
        if signal_line[i] != 0.0 {
            histogram[i] = macd_line[i] - signal_line[i];
        }
    }
    
    (macd_line, signal_line, histogram)
}

fn calculate_bollinger_bands(prices: &[f64], period: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let sma = calculate_sma(prices, period);
    let mut upper = vec![0.0; prices.len()];
    let mut lower = vec![0.0; prices.len()];
    
    for i in period..prices.len() {
        let avg = sma[i];
        let variance: f64 = prices[i - period..i].iter().map(|p| (p - avg).powi(2)).sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        
        upper[i] = avg + 2.0 * std_dev;
        lower[i] = avg - 2.0 * std_dev;
    }
    
    (sma, upper, lower)
}

fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
    let mut rsi = vec![50.0; prices.len()];
    if prices.len() <= period { return rsi; }
    
    let mut gains = 0.0;
    let mut losses = 0.0;
    
    // Initial accumulation
    for i in 1..=period {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 { gains += change; } else { losses -= change; }
    }
    
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    
    for i in period + 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        let (gain, loss) = if change > 0.0 { (change, 0.0) } else { (0.0, -change) };
        
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        
        if avg_loss == 0.0 {
            rsi[i] = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            rsi[i] = 100.0 - (100.0 / (1.0 + rs));
        }
    }
    rsi
}

/// Calculate Average True Range (ATR) - Volatility Indicator
fn calculate_atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Vec<f64> {
    let mut tr = vec![0.0; highs.len()];
    
    // True Range = max(high-low, |high-prev_close|, |low-prev_close|)
    for i in 1..highs.len() {
        let h_l = highs[i] - lows[i];
        let h_c = (highs[i] - closes[i-1]).abs();
        let l_c = (lows[i] - closes[i-1]).abs();
        tr[i] = h_l.max(h_c).max(l_c);
    }
    
    // ATR is EMA of True Range
    calculate_ema(&tr, period)
}

/// Calculate Stochastic Oscillator - Momentum Indicator
fn calculate_stochastic(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Vec<f64> {
    let mut k = vec![50.0; closes.len()];
    
    for i in period..closes.len() {
        let lowest = lows[i-period..i].iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let highest = highs[i-period..i].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        if highest != lowest {
            k[i] = ((closes[i] - lowest) / (highest - lowest)) * 100.0;
        }
    }
    k
}

/// Calculate On-Balance Volume (OBV) - Volume-based Trend Indicator
fn calculate_obv(closes: &[f64], volumes: &[f64]) -> Vec<f64> {
    let mut obv = vec![0.0; closes.len()];
    if closes.is_empty() { return obv; }
    
    obv[0] = volumes[0];
    for i in 1..closes.len() {
        obv[i] = obv[i-1] + if closes[i] > closes[i-1] { 
            volumes[i] 
        } else if closes[i] < closes[i-1] { 
            -volumes[i] 
        } else { 
            0.0 
        };
    }
    obv
}

// --- Main Training Logic ---

/// Train a signal model for the specified ticker
pub async fn train_signal_model(ticker: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    println!("🧠 [Signal Model] Fetching Data for {} (5m candles)...", ticker);
    
    let client = Client::new();
    
    // 1. Fetch Data (Try Kraken first, then Binance.US)
    let kraken_pair = match ticker {
        "BTCUSDT" => "XXBTZUSD",
        "ETHUSDT" => "XETHZUSD",
        "SOLUSDT" => "SOLUSD",
        "LTCUSDT" => "XLTCZUSD",
        _ => ticker, // Fallback to ticker
    };

    let mut high_prices: Vec<f64> = Vec::new();
    let mut low_prices: Vec<f64> = Vec::new();
    let mut close_prices: Vec<f64> = Vec::new();
    let mut volumes: Vec<f64> = Vec::new();

    let mut success = false;

    // Try Kraken
    println!("   📈 Attempting [Kraken] API for {}...", kraken_pair);
    let kraken_url = "https://api.kraken.com/0/public/OHLC";
    let kraken_params = [
        ("pair", kraken_pair),
        ("interval", "5"), // 5 minutes
    ];

    if let Ok(resp) = client.get(kraken_url).query(&kraken_params).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(result) = json.get("result") {
                    if let Some(ohlc_data) = result.as_object().and_then(|m| m.values().next()).and_then(|v| v.as_array()) {
                        for k in ohlc_data {
                            let arr = k.as_array().ok_or("Invalid Kraken candle")?;
                            // [time, open, high, low, close, vwap, volume, count]
                            if let (Some(h), Some(l), Some(c), Some(v)) = (
                                arr.get(2).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                                arr.get(3).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                                arr.get(4).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                                arr.get(6).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                            ) {
                                high_prices.push(h);
                                low_prices.push(l);
                                close_prices.push(c);
                                volumes.push(v);
                            }
                        }
                        if close_prices.len() >= 200 {
                            println!("   ✅ [Kraken] Fetched {} candles", close_prices.len());
                            success = true;
                        }
                    }
                }
            }
        }
    }

    // Try Binance.US if Kraken failed
    if !success {
        println!("   📈 Attempting [Binance.US] API for {}...", ticker);
        let b_us_url = "https://api.binance.us/api/v3/klines";
        let b_us_params = [
            ("symbol", ticker),
            ("interval", "5m"),
            ("limit", "1000")
        ];

        if let Ok(resp) = client.get(b_us_url).query(&b_us_params).send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<Vec<serde_json::Value>>().await {
                    high_prices.clear();
                    low_prices.clear();
                    close_prices.clear();
                    volumes.clear();

                    for k in &json_data {
                        if let (Some(h), Some(l), Some(c), Some(v)) = (
                            k[2].as_str().and_then(|s| s.parse::<f64>().ok()),
                            k[3].as_str().and_then(|s| s.parse::<f64>().ok()),
                            k[4].as_str().and_then(|s| s.parse::<f64>().ok()),
                            k[5].as_str().and_then(|s| s.parse::<f64>().ok()),
                        ) {
                            high_prices.push(h);
                            low_prices.push(l);
                            close_prices.push(c);
                            volumes.push(v);
                        }
                    }
                    if close_prices.len() >= 200 {
                        println!("   ✅ [Binance.US] Fetched {} candles", close_prices.len());
                        success = true;
                    }
                }
            }
        }
    }

    if !success {
        return Err(format!("All data sources failed for {}. Check network/geo-restrictions.", ticker).into());
    }

    println!("🧠 [Signal Model] Computing indicators for {} candles...", close_prices.len());

    // 2. Compute Indicators (Batch) - ENHANCED with ATR, Stochastic, OBV
    let sma_20 = calculate_sma(&close_prices, 20);
    let sma_50 = calculate_sma(&close_prices, 50);
    let sma_200 = calculate_sma(&close_prices, 200);
    let (_, _, macd_hist) = calculate_macd(&close_prices);
    let (bb_mid, bb_upper, bb_lower) = calculate_bollinger_bands(&close_prices, 20);
    let rsi = calculate_rsi(&close_prices, 14);
    
    // NEW INDICATORS (Week 1 Enhancement)
    let atr = calculate_atr(&high_prices, &low_prices, &close_prices, 14);
    let stochastic = calculate_stochastic(&high_prices, &low_prices, &close_prices, 14);
    let obv = calculate_obv(&close_prices, &volumes);

    // 3. Prepare Features and Labels
    let look_ahead = 6; // Predict 30 mins ahead (6 * 5m)
    let start_idx = 200; // Need 200 for SMA200
    
    if close_prices.len() <= start_idx + look_ahead {
         return Err("Not enough data after calculating indicators".into());
    }
    
    let n_samples = close_prices.len() - start_idx - look_ahead;
    // ENHANCED Features (11 total - was 8):
    // 0: RSI
    // 1: MACD Histogram
    // 2: BB Width % ((Upper-Lower)/Mid)
    // 3: BB Position % ((Price-Lower)/(Upper-Lower))
    // 4: SMA 20/50 Divergence %
    // 5: SMA 50/200 Divergence %
    // 6: Price Change % (Last 5 candles)
    // 7: Volume Ratio (Current / Avg last 20)
    // 8: ATR (Volatility) - NEW
    // 9: Stochastic Oscillator - NEW
    // 10: OBV Momentum - NEW
    let n_features = 11;
    
    let mut x_data: Vec<f64> = Vec::with_capacity(n_samples * n_features);
    let mut y_data: Vec<u32> = Vec::with_capacity(n_samples);
    
    for i in start_idx..(close_prices.len() - look_ahead) {
        let price = close_prices[i];
        
        // Feature 0: RSI
        x_data.push(rsi[i]);
        
        // Feature 1: MACD Hist
        x_data.push(macd_hist[i]);
        
        // Feature 2: BB Width
        let bb_w = if bb_mid[i] != 0.0 { (bb_upper[i] - bb_lower[i]) / bb_mid[i] } else { 0.0 };
        x_data.push(bb_w);
        
        // Feature 3: BB Position
        let bb_pos = if (bb_upper[i] - bb_lower[i]) != 0.0 { (price - bb_lower[i]) / (bb_upper[i] - bb_lower[i]) } else { 0.5 };
        x_data.push(bb_pos);
        
        // Feature 4: SMA Div 20/50
        let div_20_50 = if sma_50[i] != 0.0 { (sma_20[i] - sma_50[i]) / sma_50[i] } else { 0.0 };
        x_data.push(div_20_50);
        
        // Feature 5: SMA Div 50/200
        let div_50_200 = if sma_200[i] != 0.0 { (sma_50[i] - sma_200[i]) / sma_200[i] } else { 0.0 };
        x_data.push(div_50_200);
        
        // Feature 6: Price Momentum (5 candles)
        let mom_price = if close_prices[i-5] != 0.0 { (price - close_prices[i-5]) / close_prices[i-5] } else { 0.0 };
        x_data.push(mom_price);
        
        // Feature 7: Volume Ratio (Current / Avg last 20)
        let vol_avg: f64 = volumes[i-20..i].iter().sum::<f64>() / 20.0;
        let vol_ratio = if vol_avg != 0.0 { volumes[i] / vol_avg } else { 1.0 };
        x_data.push(vol_ratio);
        
        // Feature 8: ATR (Normalized by price - volatility %)
        let atr_pct = if price != 0.0 { atr[i] / price } else { 0.0 };
        x_data.push(atr_pct);
        
        // Feature 9: Stochastic Oscillator (already 0-100 scale)
        x_data.push(stochastic[i]);
        
        // Feature 10: OBV Momentum (change in OBV over last 5 periods)
        let obv_momentum = if i >= 5 && obv[i-5] != 0.0 {
            (obv[i] - obv[i-5]) / obv[i-5].abs()
        } else { 0.0 };
        x_data.push(obv_momentum);
        
        // Label: Look Ahead
        let future_price = close_prices[i + look_ahead];
        let future_return = (future_price - price) / price * 100.0;
        
        // Thresholds for 30m prediction
        // REFACTOR: Lowered from 0.5% to 0.15% to increase sensitivity and avoid "Always HOLD" (1.0)
        let label: u32 = if future_return > 0.15 {
            2 // BUY
        } else if future_return < -0.15 {
            0 // SELL
        } else {
            1 // HOLD
        };
        y_data.push(label);
    }
    
    // 3. Train Random Forest Classifier
    let x = DenseMatrix::new(n_samples, n_features, x_data, false);
    
    let params = RandomForestClassifierParameters::default()
        .with_n_trees(100)
        .with_max_depth(12) // Increased depth for better fitting
        .with_min_samples_split(3); // Allow finer splits
    
    let rf = RandomForestClassifier::fit(&x, &y_data, params)?;
    
    // 4. Save to candidate path
    let model_bytes = bincode::serialize(&rf)?;
    
    let ticker_short = ticker.replace("USDT", "").to_lowercase();
    let candidate_path = format!("models/{}_signal_candidate.bin", ticker_short);
    let production_path = format!("models/{}_signal.bin", ticker_short);
    
    std::fs::create_dir_all("models")?;
    
    // Save candidate
    let mut file = File::create(&candidate_path)?;
    file.write_all(&model_bytes)?;
    
    // Also save as production for now (initial bootstrap)
    let mut prod_file = File::create(&production_path)?;
    prod_file.write_all(&model_bytes)?;
    
    println!("🧠 [Signal Model] {} v2 (Enhanced) Saved: {}", ticker, candidate_path);
    
    Ok(candidate_path)
}

/// Train all signal models (BTC, SOL, LTC, ETH)
pub async fn train_all_signal_models() -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();
    
    for ticker in SIGNAL_TICKERS.iter() {
        match train_signal_model(ticker).await {
            Ok(path) => {
                println!("✅ {} signal model trained", ticker);
                results.push(path);
            }
            Err(e) => {
                println!("❌ {} signal model failed: {}", ticker, e);
            }
        }
    }
    
    Ok(results)
}

/// Promote a signal model from candidate to production
pub fn promote_signal_model(ticker: &str) -> Result<String, Box<dyn Error>> {
    let ticker_short = ticker.replace("USDT", "").to_lowercase();
    let candidate_path = format!("models/{}_signal_candidate.bin", ticker_short);
    let production_path = format!("models/{}_signal.bin", ticker_short);
    
    if !std::path::Path::new(&candidate_path).exists() {
        return Err(format!("No candidate model found at {}", candidate_path).into());
    }
    
    std::fs::copy(&candidate_path, &production_path)?;
    
    println!("🎉 [Signal Model] {} promoted to production: {}", ticker, production_path);
    
    Ok(production_path)
}

/// Load and run inference on a signal model
pub fn predict_signal(ticker: &str, features: &[f64]) -> Result<u32, Box<dyn Error>> {
    let ticker_short = ticker.replace("USDT", "").to_lowercase();
    let model_path = format!("models/{}_signal.bin", ticker_short);
    
    if !std::path::Path::new(&model_path).exists() {
        return Err(format!("No signal model found at {}", model_path).into());
    }
    
    let model_bytes = std::fs::read(&model_path)?;
    let rf: RandomForestClassifier<f64, u32, DenseMatrix<f64>, Vec<u32>> = bincode::deserialize(&model_bytes)?;
    
    // Create input matrix [1, n_features]
    // NOTE: This assumes 'features' passed in matches the training set (11 features - ENHANCED)
    // The calling code (oracle_scheduler) needs to prepare these features.
    
    if features.len() != 11 {
        return Err(format!("Model expects 11 features, got {}", features.len()).into());
    }
    
    println!("🔍 [Model Inference] {} | Features: {:?}", ticker, features);

    let x = DenseMatrix::new(1, features.len(), features.to_vec(), false);
    
    let prediction = rf.predict(&x)?;
    
    println!("   -> Predicted Class: {}", prediction[0]);

    Ok(prediction[0])
}

/// Compute features for the *last* candle in the series, for inference.
/// Returns the 11-feature vector expected by the enhanced model.
pub fn compute_inference_features(
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
    volumes: &[f64]
) -> Result<Vec<f64>, String> {
    if closes.len() < 200 {
        return Err(format!("Insufficient data: {}/200 candles needed for SMA200", closes.len()));
    }
    
    let i = closes.len() - 1; // Last index
    let price = closes[i];
    
    // 1. Indicators
    // Note: This computes indicators for the whole series, which is inefficient for just 1 point
    // but correct and robust. Optimization: incremental calculation.
    
    let sma_20 = calculate_sma(closes, 20);
    let sma_50 = calculate_sma(closes, 50);
    let sma_200 = calculate_sma(closes, 200);
    let (_, _, macd_hist) = calculate_macd(closes);
    let (bb_mid, bb_upper, bb_lower) = calculate_bollinger_bands(closes, 20);
    let rsi = calculate_rsi(closes, 14);
    
    // NEW INDICATORS (Week 1 Enhancement)
    let atr = calculate_atr(highs, lows, closes, 14);
    let stochastic = calculate_stochastic(highs, lows, closes, 14);
    let obv = calculate_obv(closes, volumes);
    
    let mut features = Vec::with_capacity(11);
    
    // Feature 0: RSI
    features.push(rsi[i]);
    
    // Feature 1: MACD Hist
    features.push(macd_hist[i]);
    
    // Feature 2: BB Width
    let bb_w = if bb_mid[i] != 0.0 { (bb_upper[i] - bb_lower[i]) / bb_mid[i] } else { 0.0 };
    features.push(bb_w);
    
    // Feature 3: BB Position
    let bb_pos = if (bb_upper[i] - bb_lower[i]) != 0.0 { (price - bb_lower[i]) / (bb_upper[i] - bb_lower[i]) } else { 0.5 };
    features.push(bb_pos);
    
    // Feature 4: SMA Div 20/50
    let div_20_50 = if sma_50[i] != 0.0 { (sma_20[i] - sma_50[i]) / sma_50[i] } else { 0.0 };
    features.push(div_20_50);
    
    // Feature 5: SMA Div 50/200
    let div_50_200 = if sma_200[i] != 0.0 { (sma_50[i] - sma_200[i]) / sma_200[i] } else { 0.0 };
    features.push(div_50_200);
    
    // Feature 6: Price Momentum (5 candles)
    let mom_price = if i >= 5 && closes[i-5] != 0.0 { (price - closes[i-5]) / closes[i-5] } else { 0.0 };
    features.push(mom_price);
    
    // Feature 7: Volume Ratio
    let vol_avg: f64 = if i >= 20 { volumes[i-20..i].iter().sum::<f64>() / 20.0 } else { volumes[i] };
    let vol_ratio = if vol_avg != 0.0 { volumes[i] / vol_avg } else { 1.0 };
    features.push(vol_ratio);
    
    // Feature 8: ATR (Normalized by price - volatility %)
    let atr_pct = if price != 0.0 { atr[i] / price } else { 0.0 };
    features.push(atr_pct);
    
    // Feature 9: Stochastic Oscillator (already 0-100 scale)
    features.push(stochastic[i]);
    
    // Feature 10: OBV Momentum (change in OBV over last 5 periods)
    let obv_momentum = if i >= 5 && obv[i-5] != 0.0 {
        (obv[i] - obv[i-5]) / obv[i-5].abs()
    } else { 0.0 };
    features.push(obv_momentum);
    
    Ok(features)
}

/// Convert prediction label to TradingSignal string
pub fn label_to_signal(label: u32) -> &'static str {
    match label {
        0 => "SELL",
        1 => "HOLD",
        2 => "BUY",
        _ => "UNKNOWN"
    }
}
