#![allow(dead_code)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub balances: std::collections::HashMap<String, u64>,
    pub nonce: u64,
}


// Method-specific parameter types
#[derive(Deserialize, Debug)]
pub struct GetBalanceParams {
    pub wallet_id: String,
    pub asset: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SubmitTransferParams {
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount: u64,
    pub signature: String,
    // Validation
    pub public_key: String,
    pub timestamp: u64,
    pub prev_hash: String,
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
    pub public_key: String,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingJob {
    pub job_id: String,
    pub model_id: String,
    pub inputs: Vec<u8>,
    pub max_compute_units: u64,
    pub tx_hash: String,
    pub owner_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitResultParams {
    pub job_id: String,
    pub worker_id: String,
    pub result_data: Vec<u8>,
    pub signature: String,
    pub pow_hash: Option<String>,   // Proof-of-work hash
    pub pow_nonce: Option<u64>,     // PoW nonce
    #[serde(default)]
    pub compute_rate: u64,          // NEW: Ops/sec or Score
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterValidatorParams {
    pub validator_id: String,
    pub pubkey: String,
    pub stake_amount: u64, // Must lock Compass tokens
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitOracleVerificationJobParams {
    pub ticker: String,  // e.g., "BTC", "ETH"
    pub max_compute_units: u64,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OracleVerificationJob {
    pub job_id: String,
    pub ticker: String,
    pub oracle_price: Option<String>,  // Current oracle price if available
    pub max_compute_units: u64,
    pub submission_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecurringOracleJob {
    pub job_id: String,
    pub ticker: String,
    pub start_time: u64,
    pub end_time: u64,              // 6 hours from start
    pub interval_seconds: u64,       // 60 (every minute)
    pub total_updates_required: u32, // 360
    pub completed_updates: u32,
    pub last_update_time: u64,
    pub worker_reward_per_update: u64, // COMPASS tokens
    pub assigned_worker: Option<String>,
    pub status: String,  // "Active", "Paused", "Completed", "Cancelled"
    pub owner: String,   // NEW: Who owns the model/job
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitOracleVerificationResultParams {
    pub job_id: String,
    pub ticker: String,
    pub oracle_price: String,
    pub external_prices: Vec<(String, String)>,  // (source, price)
    pub avg_external_price: String,
    pub deviation_pct: String,
    pub passed: bool,
    pub worker_id: String,
    pub signature: String,
    pub update_number: Option<u32>,  // NEW: Track which update this is
    #[serde(default)]
    pub compute_units_used: u64,     // Track computing power
    #[serde(default)]
    pub duration_ms: u64,            // Execution time
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitRecurringJobParams {
    pub ticker: String,
    pub duration_hours: u32,
    pub interval_minutes: u32,
    pub reward_per_update: u64,
    pub submitter: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetJobProgressParams {
    pub job_id: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PurchaseNeuralNetParams {
    pub owner: String,
    pub ticker: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MintModelNFTParams {
    pub creator: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StakeParams {
    pub entity: String, // Validator or Worker address
    pub amount: u64,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnstakeParams {
    pub entity: String,
    pub amount: u64,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListModelNFTParams {
    pub token_id: String,
    pub price: u64,
    pub currency: String,
    pub seller: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuyModelNFTParams {
    pub token_id: String,
    pub buyer: String,
    pub signature: String,
}
