//! Compass v2.0 Oracle Node
//! 
//! Standalone oracle service that submits price feeds and external chain
//! data to the Compass blockchain via RPC.
//! 
//! Staking Requirement: 100,000 COMPASS

use clap::Parser;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn, error};

/// Oracle node command-line arguments
#[derive(Parser, Debug)]
#[clap(name = "oracle_node", version = "2.0.0")]
struct Args {
    /// RPC endpoint of the Compass node
    #[clap(long, default_value = "http://localhost:3030")]
    rpc_url: String,
    
    /// Oracle account name
    #[clap(long)]
    account: String,
    
    /// Oracle account password
    #[clap(long)]
    password: String,
    
    /// Price update interval in seconds
    #[clap(long, default_value = "30")]
    interval: u64,
    
    /// Supported tickers (comma-separated)
    #[clap(long, default_value = "BTCUSD,ETHUSD,SOLUSD,LTCUSD")]
    tickers: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let args = Args::parse();
    
    info!("🔮 Compass Oracle Node v2.0 Starting...");
    info!("Account: {}", args.account);
    info!("RPC URL: {}", args.rpc_url);
    info!("Update Interval: {}s", args.interval);
    
    // Parse tickers
    let tickers: Vec<String> = args.tickers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    info!("Monitoring tickers: {:?}", tickers);
    
    // Oracle main loop
    let mut interval_timer = time::interval(Duration::from_secs(args.interval));
    
    loop {
        interval_timer.tick().await;
        
        info!("⏰ Fetching price updates...");
        
        for ticker in &tickers {
            match fetch_price(ticker).await {
                Ok(price) => {
                    info!("  {} = ${:.2}", ticker, price);
                    
                    // Submit to chain via RPC
                    if let Err(e) = submit_price_feed(
                        &args.rpc_url,
                        &args.account,
                        &args.password,
                        ticker,
                        price,
                    ).await {
                        error!("Failed to submit {} price: {}", ticker, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch {} price: {}", ticker, e);
                }
            }
        }
    }
}

/// Fetch current price from external API (Binance)
async fn fetch_price(ticker: &str) -> Result<f64, Box<dyn std::error::Error>> {
    // Convert ticker format: BTCUSD -> BTCUSDT
    let symbol = ticker.replace("USD", "USDT");
    
    let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", symbol);
    
    let response = reqwest::get(&url).await?;
    let data: serde_json::Value = response.json().await?;
    
    let price_str = data["price"]
        .as_str()
        .ok_or("Missing price field")?;
    
    let price: f64 = price_str.parse()?;
    
    Ok(price)
}

/// Submit price feed to Compass chain via RPC
async fn submit_price_feed(
    rpc_url: &str,
    account: &str,
    password: &str,
    ticker: &str,
    price: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Prepare RPC request
    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "submitOraclePrice",
        "params": {
            "account": account,
            "password": password,
            "ticker": ticker,
            "price": price,
            "timestamp": current_timestamp_ms(),
        }
    });
    
    let response = client
        .post(rpc_url)
        .json(&req_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("RPC error: {}", response.status()).into());
    }
    
    let result: serde_json::Value = response.json().await?;
    
    if let Some(error) = result.get("error") {
        return Err(format!("RPC error: {}", error).into());
    }
    
    Ok(())
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}
