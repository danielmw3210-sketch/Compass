use std::error::Error;
use reqwest::Client;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use smartcore::ensemble::random_forest_regressor::RandomForestRegressor;
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::numbers::basenum::Number;
use smartcore::ensemble::random_forest_regressor::RandomForestRegressorParameters;

#[derive(Deserialize, Debug)]
struct Kline {
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    close_time: u64,
    qav: String,
    num_trades: u64,
    taker_buy_base: String,
    taker_buy_quote: String,
    ignore: String,
}

/// Generic training function for any asset
pub async fn train_asset_model(ticker: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    println!("🧠 [Rust AI] Fetching Binance Data for {}...", ticker);
    
    let client = Client::new();
    let url = "https://api.binance.com/api/v3/klines";
    let params = [
        ("symbol", ticker),
        ("interval", "1h"),
        ("limit", "1000") // 1000 hours
    ];

    // 1. Fetch Data
    let resp = client.get(url).query(&params).send().await?;
    let json_data: Vec<serde_json::Value> = resp.json().await?;
    
    let mut close_prices: Vec<f64> = Vec::new();
    
    for k in json_data {
        if let Some(c_str) = k[4].as_str() {
            if let Ok(p) = c_str.parse::<f64>() {
                close_prices.push(p);
            }
        }
    }

    if close_prices.len() < 100 {
        return Err(format!("Not enough data fetched for {}", ticker).into());
    }

    println!("🧠 [Rust AI] Training Random Forest on {} candles for {}...", close_prices.len(), ticker);

    // 2. Prepare Features (Lag 1-5)
    let mut x_data: Vec<f64> = Vec::new(); // Flattened matrix
    let mut y_data: Vec<f64> = Vec::new();
    
    let lags = 5;
    let n_samples = close_prices.len() - lags - 1;

    for i in 0..n_samples {
        let target = close_prices[i + lags];
        
        for k in 0..lags {
             x_data.push(close_prices[i + k]);
        }
        y_data.push(target);
    }
    
    // 3. Train
    let x = DenseMatrix::new(n_samples, lags, x_data, false);
    let y = y_data;

    let params = RandomForestRegressorParameters::default()
        .with_n_trees(50)
        .with_max_depth(5)
        .with_min_samples_split(2);
        
    let rf = RandomForestRegressor::fit(&x, &y, params)?;
    
    // 4. Save to CANDIDATE path
    let model_bytes = bincode::serialize(&rf)?;
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Map ticker to simpler name, e.g. BTCUSDT -> btc
    let simple_name = ticker.replace("USDT", "").to_lowercase();
    
    let candidate_path = format!("models/{}_v1_candidate_{}.bin", simple_name, timestamp);
    let latest_candidate = format!("models/{}_v1_candidate.bin", simple_name);
    
    std::fs::create_dir_all("models")?;
    
    let mut file = File::create(&candidate_path)?;
    file.write_all(&model_bytes)?;
    
    let mut latest_file = File::create(&latest_candidate)?;
    latest_file.write_all(&model_bytes)?;
    
    println!("🧠 [Rust AI] CANDIDATE Model Saved: {}", candidate_path);
    println!("   ⏳ Model will be promoted to production after epoch verification passes.");
    
    Ok(candidate_path)
}

/// Promote a candidate model to production after epoch verification
pub fn promote_model_to_production(ticker: &str) -> Result<String, Box<dyn std::error::Error>> {
    let simple_name = ticker.replace("USDT", "").to_lowercase();
    
    let candidate_path = format!("models/{}_v1_candidate.bin", simple_name);
    let production_path = format!("models/{}_v1.bin", simple_name);
    
    if !std::path::Path::new(&candidate_path).exists() {
        return Err(format!("No candidate model found at {}", candidate_path).into());
    }
    
    // Copy candidate to production
    std::fs::copy(&candidate_path, &production_path)?;
    
    println!("🎉 [Rust AI] Model PROMOTED to production: {}", production_path);
    
    Ok(production_path)
}
