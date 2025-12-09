use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Confirmed,
    Rejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct CompassGulfStreamTransaction {
    pub tx_hash: Vec<u8>,
    pub raw_tx: Vec<u8>,
    pub priority_fee: u64,
    pub timestamp_ms: u128,
    pub status: TransactionStatus,
    pub processing_node: Option<String>,
    pub confirmation_time_ms: Option<u128>,
    pub rejection_reason: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl CompassGulfStreamTransaction {
    pub fn new(tx_hash: Vec<u8>, raw_tx: Vec<u8>, priority_fee: u64) -> Self {
        let timestamp_ms = crate::gulf_stream::now_ms();
        Self {
            tx_hash,
            raw_tx,
            priority_fee,
            timestamp_ms,
            status: TransactionStatus::Pending,
            processing_node: None,
            confirmation_time_ms: None,
            rejection_reason: None,
            retry_count: 0,
            max_retries: 3,
        }
    }
}

/// TransactionObject — mirrors the Python transient object.
#[derive(Debug, Clone)]
pub struct TransactionObject {
    pub transaction_id: Option<String>,
    pub tx_hash: Vec<u8>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub status: Option<String>,
    pub transaction_type: Option<String>,
    pub validator: Option<String>,
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub timestamp: u64,
    pub data: Option<serde_json::Value>,
    pub raw_tx: Vec<u8>,
}