// RPC client for making JSON-RPC requests
use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct RpcClient {
    url: String,
    client: Client,
    request_id: AtomicU64,
}

impl RpcClient {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: Client::new(),
            request_id: AtomicU64::new(1),
        }
    }

    pub async fn get_balance(&self, wallet_id: &str, asset: &str) -> Result<u64, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "getBalance",
            "params": {
                "wallet_id": wallet_id,
                "asset": asset,
            },
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        Ok(json["result"]["balance"].as_u64().unwrap_or(0))
    }

    pub async fn get_nonce(&self, wallet_id: &str) -> Result<u64, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "getNonce",
            "params": wallet_id,
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        Ok(json["result"]["nonce"].as_u64().unwrap_or(0))
    }

    pub async fn get_chain_height(&self) -> Result<u64, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "getChainHeight",
            "params": null,
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        Ok(json["result"]["height"].as_u64().unwrap_or(0))
    }

    pub async fn get_account_info(&self, wallet_id: &str) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "getAccountInfo",
            "params": wallet_id,
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        Ok(json["result"].clone())
    }

    pub async fn submit_transaction(
        &self, 
        from: &str, 
        to: &str, 
        asset: &str, 
        amount: u64, 
        nonce: u64, 
        signature: &str
    ) -> Result<String, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "submitTransaction",
            "params": {
                "from": from,
                "to": to,
                "asset": asset,
                "amount": amount,
                "nonce": nonce,
                "signature": signature
            },
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
             // Try to extract nested message if available, else standard message
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        Ok(json["result"]["tx_hash"].as_str().unwrap_or("").to_string())
    }

    // Helper for sending requests
    async fn send_request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        });

        let response = self.client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(error["message"].as_str().unwrap_or("Unknown error").to_string());
        }
        
        Ok(json["result"].clone())
    }

    pub async fn get_node_info(&self) -> Result<serde_json::Value, String> {
        self.send_request("getNodeInfo", json!(null)).await
    }

    pub async fn get_version(&self) -> Result<String, String> {
        let res = self.send_request("getVersion", json!(null)).await?;
        Ok(res["version"].as_str().unwrap_or("unknown").to_string())
    }

    pub async fn get_peers(&self) -> Result<Vec<String>, String> {
        let res = self.send_request("getPeers", json!(null)).await?;
        // Extract peers array
        let peers_val = res.get("peers").ok_or("No 'peers' field in response")?;
        let peers: Vec<String> = serde_json::from_value(peers_val.clone())
            .map_err(|e| format!("Failed to parse peers: {}", e))?;
        Ok(peers)
    }

    pub async fn submit_mint(&self, params: crate::rpc::types::SubmitMintParams) -> Result<String, String> {
        let res = self.send_request("submitMint", serde_json::to_value(params).unwrap()).await?;
        Ok(res["tx_hash"].as_str().unwrap_or("").to_string())
    }

    pub async fn submit_burn(&self, params: crate::rpc::types::SubmitBurnParams) -> Result<String, String> {
        let res = self.send_request("submitBurn", serde_json::to_value(params).unwrap()).await?;
        Ok(res["tx_hash"].as_str().unwrap_or("").to_string())
    }
}
