// RPC types for JSON-RPC 2.0 protocol
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Serialize, Debug)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

// Method-specific parameter types
#[derive(Deserialize, Debug)]
pub struct GetBalanceParams {
    pub wallet_id: String,
    pub asset: String,
}

#[derive(Deserialize, Debug)]
pub struct SubmitTransferParams {
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount: u64,
    pub signature: String,
}

#[derive(Deserialize, Debug)]
pub struct GetBlockParams {
    pub height: u64,
}

#[derive(Deserialize, Debug)]
pub struct GetLatestBlocksParams {
    pub count: u32,
}

#[derive(Deserialize, Debug)]
pub struct GetTxStatusParams {
    pub tx_hash: String,
}

#[derive(Serialize, Debug)]
pub struct NodeInfo {
    pub height: u64,
    pub head_hash: Option<String>,
    pub version: String,
    pub peer_count: u32, // placeholder
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitMintParams {
    pub vault_id: String,
    pub collateral_asset: String,
    pub collateral_amount: u64,
    pub compass_asset: String,
    pub mint_amount: u64,
    pub owner: String,
    pub tx_proof: String,
    pub oracle_signature: String,
    #[serde(default)]
    pub fee: u64,
    pub signature: String, // header signature
    pub prev_hash: Option<String>,
    pub timestamp: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitBurnParams {
    pub vault_id: String,
    pub compass_asset: String,
    pub burn_amount: u64,
    pub redeemer: String,
    pub destination_address: String,
    #[serde(default)]
    pub fee: u64,
    pub signature: String, // header signature
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPeersResponse {
    pub peers: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ValidatorStats {
    pub blocks_produced: u64,
    pub compute_earned: u64, // smaller unit
    pub uptime_hours: u64,
    pub avg_block_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetValidatorStatsParams {
    pub validator: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitComputeParams {
    pub job_id: String,
    pub model_id: String,
    pub inputs: Vec<u8>,
    pub max_compute_units: u64,
    pub signature: String,
    pub owner_id: String, // Added owner for billing
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPendingComputeJobsParams {
    // Optional filter by model_id if worker only supports one model
    pub model_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingJob {
    pub job_id: String,
    pub model_id: String,
    pub inputs: Vec<u8>,
    pub max_compute_units: u64,
    pub tx_hash: String,
    pub owner_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitResultParams {
    pub job_id: String,
    pub worker_id: String,
    pub result_data: Vec<u8>,
    pub signature: String,
}
