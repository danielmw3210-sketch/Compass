use rust_compass::layer3::paper_trading::{TradingPortfolio, PaperTrade, TradingSignal};
use rust_compass::layer3::onnx_inference::{ModelRegistry, price_to_signal};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use serde::Deserialize;

// --- Binance Data Structures ---
#[derive(Debug, Deserialize)]
struct BinanceKline(
    u64,    // Open time
    String, // Open
    String, // High
    String, // Low
    String, // Close
    String, // Volume
    u64,    // Close time
    String, // Quote asset volume
    i64,    // Number of trades
    String, // Taker buy base asset volume
    String, // Taker buy quote asset volume
    String  // Ignore
);

async fn fetch_candles(ticker: &str, limit: usize) -> Result<Vec<Vec<f64>>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
        ticker, limit
    );
    let resp = reqwest::get(&url).await?.json::<Vec<BinanceKline>>().await?;
    
    let mut sequence = Vec::new();
    for k in resp {
        // Parse OHLCV
        let open = k.1.parse::<f64>()?;
        let high = k.2.parse::<f64>()?;
        let low = k.3.parse::<f64>()?;
        let close = k.4.parse::<f64>()?;
        let vol = k.5.parse::<f64>()?;
        
        // For V1 models, we often just use [close] or [open, high, low, close, vol]
        // Let's assume the model trained on [close] for now, or check ScalerParams.
        // But to be safe and generic, let's provide [close] as single feature if unsure,
        // OR standard OHLCV if the registry handles it.
        // Looking at onnx_inference.rs, heuristic uses k[0].
        // Let's pass [close] as a single feature for now as it's the most common for simple LSTMs.
        sequence.push(vec![close]); 
    }
    Ok(sequence)
}

async fn get_current_price(ticker: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", ticker);
    let resp = reqwest::get(&url).await?.json::<serde_json::Value>().await?;
    let price = resp["price"].as_str().unwrap().parse::<f64>()?;
    Ok(price)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Initializing AI Trading Bot...");
    
    // 1. Initialize Portfolio
    let mut portfolio = TradingPortfolio::new(10_000.0); // $10k paper money
    println!("{}", portfolio.get_summary());

    // 2. Initialize Models
    let mut registry = ModelRegistry::new();
    let tickers = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];
    
    for ticker in &tickers {
        // Try to load models (assuming they exist in models/ or dist/models/)
        if let Err(e) = registry.load_model(ticker) {
            println!("⚠️ Failed to load model for {}: {}", ticker, e);
        }
    }

    // 3. Trading Loop
    println!("🚀 Starting Trading Loop (Interval: 60s)...");
    
    loop {
        for ticker in &tickers {
            // A. Fetch Data
            let price_res = get_current_price(ticker).await;
            let candles_res = fetch_candles(ticker, 50).await;
            
            if let (Ok(current_price), Ok(sequence)) = (price_res, candles_res) {
                
                // B. Inference
                // Note: predict might fail if model not loaded, but has heuristic fallback
                match registry.predict(ticker, &sequence) {
                    Ok(predicted_price) => {
                        let signal_raw = price_to_signal(current_price, predicted_price, 0.001); // 0.1% threshold
                        let signal = match signal_raw {
                            2 => TradingSignal::Buy,
                            0 => TradingSignal::Sell,
                            _ => TradingSignal::Hold,
                        };

                        println!("📈 {} | Price: ${:.2} | Pred: ${:.2} | Signal: {:?}", 
                                 ticker, current_price, predicted_price, signal);
                        
                        // C. Execution Logic
                        // Check if we have an open position
                        let has_position = portfolio.open_trades.contains_key(*ticker);
                        
                        if signal == TradingSignal::Buy && !has_position {
                            // BUY Logic
                            let position_size = portfolio.current_balance * 0.10; // 10% of portfolio
                            if position_size > 10.0 {
                                let trade = PaperTrade::new(
                                    ticker,
                                    "model_v1", // placeholder ID
                                    "1m",
                                    TradingSignal::Buy,
                                    current_price,
                                    position_size,
                                    "pred_id"
                                );
                                
                                if let Ok(_) = portfolio.open_trade(trade) {
                                    println!("   🟢 OPEN LONG: {} @ ${:.2}", ticker, current_price);
                                }
                            }
                        } else if signal == TradingSignal::Sell && has_position {
                            // CLOSE LONG Logic (Assuming we only do Long-Only strategies for now)
                            // Or if we supported shorting, we'd check position direction.
                            // For simplicity, let's assume 'Sell' signal closes 'Buy' position.
                            
                            // Check if the open position direction matches (Buy)
                            let trade = portfolio.open_trades.get(*ticker).unwrap();
                            if trade.signal == TradingSignal::Buy {
                                if let Ok(_) = portfolio.close_trade(ticker, current_price) {
                                    println!("   🔴 CLOSE LONG: {} @ ${:.2}", ticker, current_price);
                                }
                            }
                        }
                    },
                    Err(e) => println!("   ❌ Prediction failed: {}", e),
                }
            } else {
                println!("   ⚠️ Failed to fetch data for {}", ticker);
            }
        }

        // Save Portfolio State
        let json = serde_json::to_string_pretty(&portfolio)?;
        std::fs::write("trading_bot_portfolio.json", json)?;
        
        println!("⏳ Waiting 60s...");
        sleep(Duration::from_secs(60)).await;
    }
}
