/// RPC Client extension for Oracle Verification
use super::RpcClient;
use crate::rpc::types::OracleVerificationJob;
use serde_json::json;

impl RpcClient {
    /// Get pending oracle verification jobs
    pub async fn get_pending_oracle_jobs(&self) -> Result<Vec<OracleVerificationJob>, String> {
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "getPendingOracleJobs",
            "params": {},
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
        
        let result = json
            .get("result")
            .ok_or("Missing result field")?;
        
        serde_json::from_value(result.clone())
            .map_err(|e| format!("Failed to parse oracle jobs: {}", e))
    }
    
    /// Submit oracle verification result
    pub async fn submit_oracle_verification_result(
        &self,
        job_id: String,
        ticker: String,
        oracle_price: String,
        avg_external_price: String,
        deviation_pct: String,
        passed: bool,
        worker_id: String,
        signature: String,
    ) -> Result<String, String> {
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": "submitOracleVerificationResult",
            "params": {
                "job_id": job_id,
                "ticker": ticker,
                "oracle_price": oracle_price,
                "external_prices": [],
                "avg_external_price": avg_external_price,
                "deviation_pct": deviation_pct,
                "passed": passed,
                "worker_id": worker_id,
                "signature": signature,
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
        
        Ok(json
            .get("result")
            .and_then(|r| r.as_str())
            .unwrap_or("Success")
            .to_string())
    }
}
