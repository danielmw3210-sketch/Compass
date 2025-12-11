use reqwest::Client;
use serde::Deserialize;

/// Litecoin client using BlockCypher public API
#[derive(Debug, Clone)]
pub struct LitecoinClient {
    client: Client,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockCypherTx {
    pub hash: String,
    pub confirmations: u32,
    pub outputs: Vec<TxOutput>,
}

#[derive(Debug, Deserialize)]
pub struct TxOutput {
    pub value: u64,  // Litoshis (smallest unit)
    pub addresses: Vec<String>,
}

impl LitecoinClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn get_transaction(&self, txid: &str) -> Result<BlockCypherTx, String> {
        let mut url = format!("https://api.blockcypher.com/v1/ltc/main/txs/{}", txid);
        
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

    pub async fn verify_deposit(
        &self,
        txid: &str,
        address: &str,
        min_amount_litoshis: u64,
    ) -> Result<(bool, u32), String> {
        let tx = self.get_transaction(txid).await?;

        for output in &tx.outputs {
            if output.addresses.contains(&address.to_string()) && output.value >= min_amount_litoshis {
                return Ok((true, tx.confirmations));
            }
        }

        Ok((false, tx.confirmations))
    }
}
