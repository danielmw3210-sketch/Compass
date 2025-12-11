use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRequest {
    pub chain: String,           // "BTC", "LTC", "SOL"
    pub tx_hash: String,          // Transaction hash
    pub vault_address: String,    // Expected destination
    pub expected_amount: u64,     // Expected amount in smallest unit
    pub requester: String,        // User requesting mint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositProof {
    pub verified: bool,
    pub tx_hash: String,
    pub amount: u64,
    pub confirmations: u32,
    pub vault_address: String,
    pub timestamp: u64,
    pub oracle_signature: String,
    pub oracle_pubkey: String,
}

#[derive(Debug, Clone)]
pub struct OracleConfig {
    pub blockcypher_api_key: Option<String>,
    pub min_confirmations_btc: u32,
    pub min_confirmations_ltc: u32,
    pub min_confirmations_sol: u32,
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            blockcypher_api_key: None, // Optional, rate limits apply without key
            min_confirmations_btc: 6,
            min_confirmations_ltc: 12,
            min_confirmations_sol: 32,
        }
    }
}
