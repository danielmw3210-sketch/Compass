#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::error::Error;
use rand::Rng;

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

// API Response Schemas
#[derive(Debug, Deserialize)]
struct CoinGeckoPriceResponse {
    bitcoin: Option<CoinGeckoPrice>,
    ethereum: Option<CoinGeckoPrice>,
    solana: Option<CoinGeckoPrice>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: f64,
}

#[derive(Debug, Deserialize)]
struct EtherscanGasResponse {
    status: String,
    result: EtherscanGasResult,
}

#[derive(Debug, Deserialize)]
struct EtherscanGasResult {
    #[serde(rename = "ProposeGasPrice")]
    propose_gas_price: String,
}

#[derive(Debug, Deserialize)]
struct KrakenTradesResponse {
    result: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DeFiLlamaTVLResponse {
    #[serde(flatten)]
    chains: serde_json::Value,
}

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct FinanceDataFetcher {
    client: reqwest::Client,
    // Cache: Ticker -> (Price, Timestamp)
    price_cache: HashMap<String, (f64, Instant)>,
    // Keeping context cache for other fields (Gas/TVL)
    last_fetch_context: Instant,
    cached_context: Option<MarketContext>,
}

impl FinanceDataFetcher {
    // Production API URLs ... (Already defined) ...
    pub const COINGECKO_URL: &'static str = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana&vs_currencies=usd";
    pub const ETHERSCAN_GAS_URL: &'static str = "https://api.etherscan.io/v2/api?chainid=1&module=gastracker&action=gasoracle&apikey=YourApiKeyToken"; 
    pub const KRAKEN_TRADES_URL: &'static str = "https://api.kraken.com/0/public/Trades?pair=XBTUSD";
    pub const DEFILLAMA_TVL_URL: &'static str = "https://api.llama.fi/v2/chains";
    pub const BINANCE_URL: &'static str = "https://api.binance.com/api/v3/ticker/price";

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .build()
                .unwrap(),
            price_cache: HashMap::new(),
            last_fetch_context: Instant::now() - Duration::from_secs(60),
            cached_context: None,
        }
    }

    // Helper: Binance Fetch
    async fn fetch_binance_single(&self, symbol: &str) -> Result<f64, Box<dyn Error>> {
        let url = format!("{}?symbol={}", Self::BINANCE_URL, symbol);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() { return Err("Binance Error".into()); }
        let json: serde_json::Value = resp.json().await?;
        let price_str = json.get("price").and_then(|v| v.as_str()).ok_or("No price field")?;
        Ok(price_str.parse()?)
    }

    // Helper: CoinGecko Fetch (Bulk) - returns (BTC, ETH, SOL)
    async fn fetch_coingecko_prices(&self) -> Result<(f64, f64, f64), Box<dyn Error>> {
        let response = self.client.get(Self::COINGECKO_URL).send().await?;
        if !response.status().is_success() { return Err(format!("CoinGecko: {}", response.status()).into()); }
        let data: CoinGeckoPriceResponse = response.json().await?;
        Ok((
            data.bitcoin.ok_or("No BTC")?.usd,
            data.ethereum.ok_or("No ETH")?.usd,
            data.solana.ok_or("No SOL")?.usd
        ))
    }

    // Smart Cached Fetch
    pub async fn fetch_crypto_prices(&mut self) -> Result<(f64, f64, f64), Box<dyn Error>> {
        // Check Cache Validity (1 Minute TTL)
        let now = Instant::now();
        let ttl = Duration::from_secs(60);

        let btc_valid = self.price_cache.get("BTC").map(|(_, ts)| now.duration_since(*ts) < ttl).unwrap_or(false);
        let eth_valid = self.price_cache.get("ETH").map(|(_, ts)| now.duration_since(*ts) < ttl).unwrap_or(false);
        let sol_valid = self.price_cache.get("SOL").map(|(_, ts)| now.duration_since(*ts) < ttl).unwrap_or(false);

        if btc_valid && eth_valid && sol_valid {
            // tracing::info!("Using Cached Prices (Valid < 60s)");
            return Ok((
                self.price_cache["BTC"].0,
                self.price_cache["ETH"].0,
                self.price_cache["SOL"].0
            ));
        }

        // Cache expired or missing -> Fetch Fresh
        // 1. Try CoinGecko (Bulk)
        match self.fetch_coingecko_prices().await {
            Ok((b, e, s)) => {
                self.price_cache.insert("BTC".to_string(), (b, now));
                self.price_cache.insert("ETH".to_string(), (e, now));
                self.price_cache.insert("SOL".to_string(), (s, now));
                return Ok((b, e, s));
            },
            Err(err) => {
                tracing::warn!("⚠️ CoinGecko Failed: {}. Trying Binance...", err);
            }
        }

        // 2. Fallback: Binance (Individual)
        let btc = self.fetch_binance_single("BTCUSDT").await.unwrap_or(0.0);
        let eth = self.fetch_binance_single("ETHUSDT").await.unwrap_or(0.0);
        let sol = self.fetch_binance_single("SOLUSDT").await.unwrap_or(0.0);

        if btc > 0.0 { self.price_cache.insert("BTC".to_string(), (btc, now)); }
        if eth > 0.0 { self.price_cache.insert("ETH".to_string(), (eth, now)); }
        if sol > 0.0 { self.price_cache.insert("SOL".to_string(), (sol, now)); }

        if btc > 0.0 && eth > 0.0 {
            Ok((btc, eth, sol))
        } else {
             // 3. Stale Cache Fallback (If API totally down, use OLD cache even if expired)
             if let (Some(&(b_old, _)), Some(&(e_old, _)), Some(&(s_old, _))) = 
                (self.price_cache.get("BTC"), self.price_cache.get("ETH"), self.price_cache.get("SOL")) {
                 tracing::warn!("⚠️ All APIs Down. Using STALE cache.");
                 Ok((b_old, e_old, s_old))
             } else {
                 Err("All Price APIs Failed & No Cache".into())
             }
        }
    }

    // ... (keep fetch_gas_price, fetch_kraken_data, fetch_l2_tvl as is) ...
    pub async fn fetch_gas_price(&self) -> Result<f64, Box<dyn Error>> {
        let response = self.client.get(Self::ETHERSCAN_GAS_URL).send().await?;
        if !response.status().is_success() { return Err(format!("Etherscan: {}", response.status()).into()); }
        let json: serde_json::Value = response.json().await?;
        if let Some(result) = json.get("result") {
            if let Some(obj) = result.as_object() {
                if let Some(gas_str) = obj.get("ProposeGasPrice").and_then(|v| v.as_str()) {
                    return Ok(gas_str.parse()?);
                }
            }
        }
        Err("Etherscan invalid".into())
    }

    pub async fn fetch_kraken_data(&self) -> Result<(u32, f64), Box<dyn Error>> {
        let response = self.client.get(Self::KRAKEN_TRADES_URL).send().await?;
        if !response.status().is_success() { return Err(format!("Kraken: {}", response.status()).into()); }
        let data: KrakenTradesResponse = response.json().await?;
        if let Some(trades_array) = data.result.get("XXBTZUSD").and_then(|v| v.as_array()) {
            let count = trades_array.len() as u32;
            let vol: f64 = trades_array.iter().take(50).filter_map(|t| t[1].as_str().unwrap_or("0").parse::<f64>().ok()).sum();
            Ok((count, vol))
        } else {
            Err("Kraken format error".into())
        }
    }

    pub async fn fetch_l2_tvl(&self) -> Result<f64, Box<dyn Error>> {
        let response = self.client.get(Self::DEFILLAMA_TVL_URL).send().await?;
        if !response.status().is_success() { return Err(format!("DeFiLlama: {}", response.status()).into()); }
        let chains: Vec<serde_json::Value> = response.json().await?;
        let l2s = ["Arbitrum", "Optimism", "Base", "zkSync Era"];
        let tvl: f64 = chains.iter().filter_map(|c| {
            if l2s.contains(&c.get("name")?.as_str()?) { c.get("tvl")?.as_f64() } else { None }
        }).sum();
        Ok(tvl)
    }

    pub async fn fetch_context(&mut self) -> Result<MarketContext, Box<dyn Error>> {
        // General Context Rate Limit (15s) - still useful to coordinate the "Bundle"
        if self.last_fetch_context.elapsed() < Duration::from_secs(15) {
            if let Some(ctx) = &self.cached_context {
                return Ok(ctx.clone());
            }
        }

        // 1. Crypto Prices (Now handles its own 60s cache internally)
        // Note: fetch_crypto_prices is now &mut self to update cache
        let (btc, eth, sol) = match self.fetch_crypto_prices().await {
            Ok(v) => { tracing::info!("✅ Prices: BTC=${} ETH=${}", v.0, v.1); v },
            Err(e) => { 
                tracing::warn!("⚠️ Price Fetch Failed: {}", e);
                (65000.0, 3500.0, 145.0) 
            }
        };


        // 2. Gas
        let gas = match self.fetch_gas_price().await {
            Ok(v) => { tracing::info!("✅ Etherscan: Gas={}", v); v },
            Err(e) => { tracing::warn!("⚠️ Etherscan: {}", e); 25.0 }
        };

        // 3. Kraken
        let (k_tx, k_vol) = match self.fetch_kraken_data().await {
            Ok(v) => { tracing::info!("✅ Kraken: {} txs", v.0); v },
            Err(e) => { tracing::warn!("⚠️ Kraken: {}", e); (100, 10.0) }
        };

        // 4. TVL
        let tvl = match self.fetch_l2_tvl().await {
            Ok(v) => { tracing::info!("✅ DeFiLlama: TVL=${:.2}B", v/1e9); v },
            Err(e) => { tracing::warn!("⚠️ DeFiLlama: {}", e); 45_000_000_000.0 }
        };

        // Simulated (Sentiment)
        let mut rng = rand::thread_rng();
        let sent = rng.gen_range(0.3..0.8);
        let btc_sent = rng.gen_range(0.4..0.9);
        let eth_users = rng.gen_range(300_000..450_000);
        let sol_users = rng.gen_range(800_000..1_200_000);
        let vol = 2_500_000_000.0;

        let ctx = MarketContext {
            btc_price: btc, btc_sentiment: btc_sent,
            eth_price: eth, eth_active_users: eth_users,
            sol_price: sol, sol_active_users: sol_users,
            gas_price_gwei: gas, l2_tvl_usd: tvl,
            dex_volume_24h: vol, market_sentiment: sent,
            kraken_recent_txs: k_tx, kraken_scan_vol: k_vol
        };

        // Update Context Cache
        self.cached_context = Some(ctx.clone());
        self.last_fetch_context = Instant::now();

        Ok(ctx)
    }
}
