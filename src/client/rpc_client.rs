// RPC client for making JSON-RPC requests
use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct RpcClient {
    pub(super) url: String,
    pub(super) client: Client,
    pub(super) request_id: AtomicU64,
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

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
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

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
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

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
        }

        Ok(json["result"]["height"].as_u64().unwrap_or(0))
    }


    pub async fn call_method<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let response = self.client.post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
            
        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
            
        if let Some(err) = json.get("error") {
            return Err(format!("RPC Error: {}", err));
        }
        
        if let Some(res) = json.get("result") {
            serde_json::from_value(res.clone())
                .map_err(|e| format!("Result type mismatch: {}", e))
        } else {
             Err("No result in response".to_string())
        }
    }

    pub async fn get_account_info(&self, wallet_id: &str) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = json!({
            "jsonrpc": "2.0",
            "method": "getAccountInfo",
            "params": { "wallet_id": wallet_id },
            "id": id,
        });

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
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
        signature: &str,
        prev_hash: Option<String>,
        timestamp: Option<u64>,
        public_key: &str,
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
                "signature": signature,
                "prev_hash": prev_hash,
                "timestamp": timestamp,
                "public_key": public_key
            },
            "id": id,
        });

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            // Try to extract nested message if available, else standard message
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
        }

        Ok(json["result"]["tx_hash"].as_str().unwrap_or("").to_string())
    }

    // Helper for sending requests
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        });

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = json.get("error") {
            return Err(error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string());
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

    pub async fn submit_mint(
        &self,
        params: crate::rpc::types::SubmitMintParams,
    ) -> Result<String, String> {
        let res = self
            .send_request("submitMint", serde_json::to_value(params).unwrap())
            .await?;
        Ok(res["tx_hash"].as_str().unwrap_or("").to_string())
    }

    pub async fn submit_burn(
        &self,
        params: crate::rpc::types::SubmitBurnParams,
    ) -> Result<String, String> {
        let res = self
            .send_request("submitBurn", serde_json::to_value(params).unwrap())
            .await?;
        Ok(res["tx_hash"].as_str().unwrap_or("").to_string())
    }

    pub async fn submit_compute(
        &self,
        job_id: String,
        model_id: String,
        inputs: Vec<u8>,
        max_compute_units: u64,
        bid_amount: u64,
        bid_asset: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
         let params = crate::rpc::types::SubmitComputeParams {
            job_id,
            model_id,
            inputs,
            max_compute_units,
            bid_amount,
            bid_asset,
            signature: "stub_sig".to_string(), // TODO: sign
            owner_id: "client_user".to_string(), // TODO: Pass actual user
        };
        let resp: serde_json::Value = self.send_request("submitCompute", serde_json::to_value(params).unwrap()).await.map_err(|e| format!("RPC error: {}", e))?;
        let result = resp.get("tx_hash").ok_or("No tx_hash field")?.as_str().ok_or("tx_hash not a string")?;
        Ok(result.to_string())
    }
    
    pub async fn get_pending_compute_jobs(
        &self,
        model_id: Option<String>,
    ) -> Result<Vec<crate::rpc::types::PendingJob>, Box<dyn std::error::Error>> {
        let params = crate::rpc::types::GetPendingComputeJobsParams { model_id };
        let resp: serde_json::Value = self.send_request("getPendingComputeJobs", serde_json::to_value(params).unwrap()).await.map_err(|e| format!("RPC error: {}", e))?;
        let jobs: Vec<crate::rpc::types::PendingJob> = serde_json::from_value(resp)?;
        Ok(jobs)
    }

    pub async fn submit_result(
        &self,
        job_id: String,
        worker_id: String,
        result_data: Vec<u8>,
        pow_hash: Option<String>,
        pow_nonce: Option<u64>,
        compute_rate: u64, // NEW param
    ) -> Result<String, Box<dyn std::error::Error>> {
         let params = crate::rpc::types::SubmitResultParams {
            job_id,
            worker_id,
            result_data,
            signature: "stub_worker_sig".to_string(),
            pow_hash,
            pow_nonce,
            compute_rate,
        };
        let resp: serde_json::Value = self.send_request("submitResult", serde_json::to_value(params).unwrap()).await.map_err(|e| format!("RPC error: {}", e))?;
        let result = resp.get("tx_hash").ok_or("No tx_hash field")?.as_str().ok_or("tx_hash not a string")?;
        Ok(result.to_string())
    }

    pub async fn get_all_nfts(&self) -> Result<Vec<crate::layer3::model_nft::ModelNFT>, String> {
        let res = self.send_request("getAllNFTs", serde_json::json!(null)).await?;
        serde_json::from_value(res).map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn get_block_range(&self, start: Option<u64>, count: Option<u64>) -> Result<Vec<crate::block::Block>, String> {
        let params = serde_json::json!({
            "start": start,
            "count": count
        });
        let res = self.send_request("getBlockRange", params).await?;
        serde_json::from_value(res).map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn get_oracle_prices(&self) -> Result<std::collections::HashMap<String, f64>, String> {
        let res = self.send_request("getOraclePrices", serde_json::json!(null)).await?;
        serde_json::from_value(res).map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn get_latest_signal(&self, ticker: &str) -> Result<serde_json::Value, String> {
        let params = serde_json::json!({
            "ticker": ticker,
            "subscriber": "admin"
        });
        self.send_request("getLatestSignal", params).await
    }
    
    pub async fn get_paper_trading_stats(&self) -> Result<crate::layer3::paper_trading::TradingPortfolio, String> {
        let res = self.send_request("getPaperTradingStats", serde_json::json!(null)).await?;
        serde_json::from_value(res).map_err(|e| format!("Parse error: {}", e))
    }
}
