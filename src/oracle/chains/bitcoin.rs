use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Bitcoin client using BlockCypher public API
#[derive(Debug, Clone)]
pub struct BitcoinClient {
    client: Client,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockCypherTx {
    pub hash: String,
    pub confirmations: u32,
    pub outputs: Vec<TxOutput>,
    pub block_height: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TxOutput {
    pub value: u64,  // Satoshis
    pub addresses: Vec<String>,
}

impl BitcoinClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Get transaction from BlockCypher API
    pub async fn get_transaction(&self, txid: &str) -> Result<BlockCypherTx, String> {
        let mut url = format!("https://api.blockcypher.com/v1/btc/main/txs/{}", txid);
        
        if let Some(key) = &self.api_key {
            url.push_str(&format!("?token={}", key));
        }

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error: {}", response.status()));
        }

        let tx: BlockCypherTx = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(tx)
    }

    /// Verify deposit to specific address with minimum amount
    pub async fn verify_deposit(
        &self,
        txid: &str,
        address: &str,
        min_amount_sats: u64,
    ) -> Result<(bool, u32), String> {
        let tx = self.get_transaction(txid).await?;

        // Check if any output matches the address and amount
        for output in &tx.outputs {
            if output.addresses.contains(&address.to_string()) && output.value >= min_amount_sats {
                return Ok((true, tx.confirmations));
            }
        }

        Ok((false, tx.confirmations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_transaction() {
        let client = BitcoinClient::new(None);
        
        // Known Bitcoin transaction
        let txid = "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16";
        
        let result = client.get_transaction(txid).await;
        assert!(result.is_ok());
    }
}
