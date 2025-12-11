use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GulfStreamStats {
    pub transactions_received: u64,
    pub transactions_forwarded: u64,
    pub transactions_confirmed: u64,
    pub transactions_rejected: u64,
    pub pending_transactions: u64,
    pub processing_transactions: u64,
    pub confirmed_transactions: u64,
    pub rejected_transactions: u64,
    pub avg_confirmation_time_ms: f64,
    pub current_slot: Option<String>,
    pub next_leader: Option<String>,
    pub queue_sizes: QueueSizes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSizes {
    pub high_priority: u64,
    pub normal_priority: u64,
    pub low_priority: u64,
}
