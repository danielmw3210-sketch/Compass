use reqwest;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    pub source: String,
    pub ticker: String,
    pub price_usd: Decimal,
    pub timestamp: u64,
}

pub struct PriceFetcher {
    client: reqwest::Client,
}

impl PriceFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Fetch price from CoinGecko API
    pub async fn fetch_coingecko(&self, ticker: &str) -> Result<PriceQuote, String> {
        let coin_id = match ticker {
            "BTC" => "bitcoin",
            "ETH" => "ethereum",
            "LTC" => "litecoin",
            "SOL" => "solana",
            _ => return Err(format!("Unsupported ticker: {}", ticker)),
        };

        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            coin_id
        );

        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("CoinGecko API error: {}", e))?;

        #[derive(Deserialize)]
        struct CoinGeckoResponse {
            #[serde(flatten)]
            prices: std::collections::HashMap<String, CoinPrice>,
        }

        #[derive(Deserialize)]
        struct CoinPrice {
            usd: f64,
        }

        let data: CoinGeckoResponse = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        let price = data.prices.get(coin_id)
            .ok_or_else(|| format!("Price not found for {}", coin_id))?;

        Ok(PriceQuote {
            source: "CoinGecko".to_string(),
            ticker: ticker.to_string(),
            price_usd: Decimal::from_str(&price.usd.to_string()).unwrap_or(Decimal::ZERO),
            timestamp: crate::block::current_unix_timestamp_ms(),
        })
    }

    /// Fetch price from Binance API (US compatible)
    pub async fn fetch_binance(&self, ticker: &str) -> Result<PriceQuote, String> {
        let symbol = format!("{}USDT", ticker);
        let url = format!("https://api.binance.us/api/v3/ticker/price?symbol={}", symbol);

        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Binance API error: {}", e))?;

        #[derive(Deserialize)]
        struct BinanceResponse {
            price: String,
        }

        let data: BinanceResponse = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        let price = Decimal::from_str(&data.price).map_err(|_| "Invalid price format")?;

        Ok(PriceQuote {
            source: "Binance".to_string(),
            ticker: ticker.to_string(),
            price_usd: price,
            timestamp: crate::block::current_unix_timestamp_ms(),
        })
    }

    /// Fetch price from Kraken API
    pub async fn fetch_kraken(&self, ticker: &str) -> Result<PriceQuote, String> {
        let pair = match ticker {
            "BTC" => "XXBTZUSD",
            "ETH" => "XETHZUSD",
            "LTC" => "XLTCZUSD",
            _ => return Err(format!("Unsupported ticker for Kraken: {}", ticker)),
        };

        let url = format!("https://api.kraken.com/0/public/Ticker?pair={}", pair);

        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Kraken API error: {}", e))?;

        #[derive(Deserialize)]
        struct KrakenResponse {
            result: std::collections::HashMap<String, KrakenTicker>,
        }

        #[derive(Deserialize)]
        struct KrakenTicker {
            c: Vec<String>, // Last trade closed array [price, lot volume]
        }

        let data: KrakenResponse = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        let ticker_data = data.result.get(pair)
            .ok_or_else(|| format!("Ticker {} not found", pair))?;
        
        let price_str = ticker_data.c.get(0).ok_or("No price data")?;
        let price = Decimal::from_str(price_str).map_err(|_| "Invalid price format")?;

        Ok(PriceQuote {
            source: "Kraken".to_string(),
            ticker: ticker.to_string(),
            price_usd: price,
            timestamp: crate::block::current_unix_timestamp_ms(),
        })
    }

    /// Fetch from all sources and return average
    pub async fn fetch_all(&self, ticker: &str) -> Result<Vec<PriceQuote>, String> {
        let mut quotes = Vec::new();

        // Try CoinGecko
        if let Ok(quote) = self.fetch_coingecko(ticker).await {
            quotes.push(quote);
        }

        // Try Binance
        if let Ok(quote) = self.fetch_binance(ticker).await {
            quotes.push(quote);
        }

        // Try Kraken (not all tickers supported)
        if let Ok(quote) = self.fetch_kraken(ticker).await {
            quotes.push(quote);
        }

        if quotes.is_empty() {
            return Err(format!("Failed to fetch price from any source for {}", ticker));
        }

        Ok(quotes)
    }

    /// Calculate average price from multiple quotes
    pub fn calculate_average(quotes: &[PriceQuote]) -> Decimal {
        if quotes.is_empty() {
            return Decimal::ZERO;
        }

        let sum: Decimal = quotes.iter().map(|q| q.price_usd).sum();
        sum / Decimal::from(quotes.len())
    }

    /// Calculate deviation percentage between two prices
    pub fn calculate_deviation(price1: Decimal, price2: Decimal) -> Decimal {
        if price1.is_zero() {
            return Decimal::from(100);
        }
        ((price2 - price1).abs() / price1) * Decimal::from(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_coingecko() {
        let fetcher = PriceFetcher::new();
        let quote = fetcher.fetch_coingecko("BTC").await.unwrap();
        assert_eq!(quote.ticker, "BTC");
        assert!(quote.price_usd > Decimal::ZERO);
    }

    #[test]
    fn test_calculate_deviation() {
        let price1 = Decimal::from(100);
        let price2 = Decimal::from(105);
        let deviation = PriceFetcher::calculate_deviation(price1, price2);
        assert_eq!(deviation, Decimal::from(5)); // 5% deviation
    }
}

use std::str::FromStr;
