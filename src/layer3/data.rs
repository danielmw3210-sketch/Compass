#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::error::Error;
use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use async_trait::async_trait;

// ==============================================================================
//  Data Structures (Preserved for Compatibility)
// ==============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct GasInfo {
    pub safe_low: f64,
    pub standard: f64,
    pub fast: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    pub btc_price: f64,
    pub btc_sentiment: f64,
    pub eth_price: f64,
    pub eth_active_users: u64,
    pub sol_price: f64,
    pub sol_active_users: u64,
    pub gas_price_gwei: f64,
    pub l2_tvl_usd: f64,
    pub dex_volume_24h: f64,
    pub market_sentiment: f64,
    pub kraken_recent_txs: u32,
    pub kraken_scan_vol: f64,
}

// ==============================================================================
//  Price Provider Trait & Implementations
// ==============================================================================

#[async_trait]
pub trait PriceProvider: Send + Sync {
    async fn get_price(&self, start_ticker: &str) -> Result<f64, String>; // ticker e.g. "BTC"
    fn name(&self) -> &str;
}

// --- Binance Provider ---
pub struct BinanceProvider {
    client: reqwest::Client,
}

impl BinanceProvider {
    pub fn new(client: reqwest::Client) -> Self { Self { client } }
}

#[async_trait]
impl PriceProvider for BinanceProvider {
    fn name(&self) -> &str { "Binance" }
    
    async fn get_price(&self, ticker: &str) -> Result<f64, String> {
        let symbol = format!("{}USDT", ticker); // BTC -> BTCUSDT
        let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", symbol);
        
        let resp = self.client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;
            
        if !resp.status().is_success() {
            return Err(format!("Status {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
            
        let price_str = json.get("price")
            .and_then(|v| v.as_str())
            .ok_or("No price field")?;
            
        price_str.parse::<f64>().map_err(|_| "Invalid float".to_string())
    }
}

// --- CoinGecko Provider ---
pub struct CoinGeckoProvider {
    client: reqwest::Client,
}

impl CoinGeckoProvider {
    pub fn new(client: reqwest::Client) -> Self { Self { client } }
}

#[async_trait]
impl PriceProvider for CoinGeckoProvider {
    fn name(&self) -> &str { "CoinGecko" }
    
    async fn get_price(&self, ticker: &str) -> Result<f64, String> {
        let id = match ticker {
            "BTC" => "bitcoin",
            "ETH" => "ethereum",
            "SOL" => "solana",
            _ => return Err("Unsupported ticker".to_string()),
        };
        
        // Note: CoinGecko bulk is efficient, but this trait is single-ticker. 
        // For v1.4 Architecture we strictly follow trait.
        // Optimization: In real prod, we'd have `get_prices_bulk`.
        let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd", id);
        
        let resp = self.client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
             return Err(format!("Status {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
            
        let price = json.get(id)
            .and_then(|v| v.get("usd"))
            .and_then(|v| v.as_f64())
            .ok_or("No price field")?;
            
        Ok(price)
    }
}

// ==============================================================================
//  Main Data Fetcher
// ==============================================================================

pub struct FinanceDataFetcher {
    client: reqwest::Client,
    providers: Vec<Box<dyn PriceProvider>>,
    // Cache: Ticker -> (Price, Timestamp)
    price_cache: HashMap<String, (f64, Instant)>,
    // Context Cache
    last_fetch_context: Instant,
    cached_context: Option<MarketContext>,
}

impl FinanceDataFetcher {
    // API Constants (for specialized calls)
    pub const ETHERSCAN_GAS_URL: &'static str = "https://api.etherscan.io/v2/api?chainid=1&module=gastracker&action=gasoracle&apikey=YourApiKeyToken"; 
    pub const KRAKEN_TRADES_URL: &'static str = "https://api.kraken.com/0/public/Trades?pair=XBTUSD";
    pub const DEFILLAMA_TVL_URL: &'static str = "https://api.llama.fi/v2/chains";

    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) CompassNode/1.0")
            .build()
            .unwrap();
            
        // Provider Chain: Binance (Primary) -> CoinGecko (Backup)
        let providers: Vec<Box<dyn PriceProvider>> = vec![
            Box::new(BinanceProvider::new(client.clone())),
            Box::new(CoinGeckoProvider::new(client.clone())),
        ];

        Self {
            client,
            providers,
            price_cache: HashMap::new(),
            last_fetch_context: Instant::now() - Duration::from_secs(300), // Start stale
            cached_context: None,
        }
    }

    /// Primary Aggregation Logic
    /// Tries all providers in order until one succeeds. 
    /// Returns 0.0 if all fail (graceful degradation).
    async fn get_price_robust(&self, ticker: &str) -> f64 {
        for provider in &self.providers {
            match provider.get_price(ticker).await {
                Ok(price) => {
                    // tracing::info!("✅ Fetched {} from {} (${})", ticker, provider.name(), price);
                    return price;
                },
                Err(e) => {
                    tracing::warn!("⚠️ {} failed for {}: {}", provider.name(), ticker, e);
                    // Continue to next provider
                }
            }
        }
        tracing::error!("❌ All providers failed for {}", ticker);
        0.0
    }

    /// Fetches prices for BTC, ETH, SOL with Caching
    pub async fn fetch_crypto_prices(&mut self) -> Result<(f64, f64, f64), Box<dyn Error>> {
        let now = Instant::now();
        let ttl = Duration::from_secs(60); // 1 Minute Cache

        let mut results = Vec::new();

        for ticker in ["BTC", "ETH", "SOL"] {
            // 1. Check Cache
            if let Some((price, ts)) = self.price_cache.get(ticker) {
                if now.duration_since(*ts) < ttl {
                    results.push(*price);
                    continue;
                }
            }

            // 2. Fetch Fresh (Robust)
            let fresh_price = self.get_price_robust(ticker).await;
            
            if fresh_price > 0.0 {
                // Update Cache
                self.price_cache.insert(ticker.to_string(), (fresh_price, now));
                results.push(fresh_price);
            } else {
                // 3. Stale Fallback
                 if let Some((old_price, _)) = self.price_cache.get(ticker) {
                     tracing::warn!("⚠️ Using STALE cache for {}", ticker);
                     results.push(*old_price);
                 } else {
                     results.push(0.0); // Extreme failure
                 }
            }
        }

        Ok((results[0], results[1], results[2]))
    }

    // --- Auxiliary Fetchers (Gas, Kraken, TVL) ---
    // Kept as direct calls for now, could be modularized in Phase 1.5

    pub async fn fetch_gas_price(&self) -> Result<f64, Box<dyn Error>> {
        // Etherscan V2 Logic here (simulating upgrade by robust parsing)
        let response = self.client.get(Self::ETHERSCAN_GAS_URL).send().await?;
        if !response.status().is_success() { return Err("HTTP Error".into()); }
        
        let json: serde_json::Value = response.json().await?;
        // Try strict V2 structure, fallback to V1-ish
        if let Some(res) = json.get("result") {
            if let Some(obj) = res.as_object() {
                if let Some(s) = obj.get("ProposeGasPrice").and_then(|v| v.as_str()) {
                    return Ok(s.parse()?);
                }
            }
        }
        // Fallback or Error
        Ok(25.0) // Default safe gas
    }

    pub async fn fetch_kraken_data(&self) -> Result<(u32, f64), Box<dyn Error>> {
        let response = self.client.get(Self::KRAKEN_TRADES_URL).send().await?;
        if !response.status().is_success() { return Err("HTTP Error".into()); }
        let data: serde_json::Value = response.json().await?;
        
        if let Some(res) = data.get("result") {
            if let Some(arr) = res.get("XXBTZUSD").and_then(|v| v.as_array()) {
                let count = arr.len() as u32;
                let vol: f64 = arr.iter().take(50)
                    .filter_map(|t| t.get(1).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()))
                    .sum();
                return Ok((count, vol));
            }
        }
        Ok((100, 10.0)) // Fallback
    }

    pub async fn fetch_l2_tvl(&self) -> Result<f64, Box<dyn Error>> {
        let response = self.client.get(Self::DEFILLAMA_TVL_URL).send().await?;
        if !response.status().is_success() { return Err("HTTP Error".into()); }
        let chains: Vec<serde_json::Value> = response.json().await?;
        
        let target_chains = ["Arbitrum", "Optimism", "Base", "zkSync Era"];
        let tvl: f64 = chains.iter().filter_map(|c| {
            let name = c.get("name")?.as_str()?;
            if target_chains.contains(&name) {
                c.get("tvl")?.as_f64()
            } else { None }
        }).sum();
        Ok(tvl)
    }

    // --- Main Context Builder ---

    pub async fn fetch_context(&mut self) -> Result<MarketContext, Box<dyn Error>> {
        // Global Context Rate Limit (15s)
        if self.last_fetch_context.elapsed() < Duration::from_secs(15) {
             if let Some(ctx) = &self.cached_context {
                 return Ok(ctx.clone());
             }
        }

        // 1. Robust Prices
        let (btc, eth, sol) = self.fetch_crypto_prices().await.unwrap_or((65000.0, 3500.0, 145.0));

        // 2. Auxiliary Data
        let gas = self.fetch_gas_price().await.unwrap_or(25.0);
        let (k_tx, k_vol) = self.fetch_kraken_data().await.unwrap_or((100, 10.0));
        let tvl = self.fetch_l2_tvl().await.unwrap_or(45_000_000_000.0);

        // 3. Simulated Sentiment (Placeholder for Phase 2 AI Sentiment)
        let mut rng = rand::thread_rng();
        let ctx = MarketContext {
            btc_price: btc, btc_sentiment: rng.gen_range(0.4..0.9),
            eth_price: eth, eth_active_users: rng.gen_range(300000..450000),
            sol_price: sol, sol_active_users: rng.gen_range(800000..1200000),
            gas_price_gwei: gas, l2_tvl_usd: tvl,
            dex_volume_24h: 2_500_000_000.0, 
            market_sentiment: rng.gen_range(0.3..0.8),
            kraken_recent_txs: k_tx, kraken_scan_vol: k_vol
        };

        // Cache It
        self.cached_context = Some(ctx.clone());
        self.last_fetch_context = Instant::now();

        Ok(ctx)
    }
}
