use crate::gulf_stream::transactions::CompassGulfStreamTransaction;
use crate::gulf_stream::validator::ValidatorSlot;
use crate::gulf_stream::stats::{GulfStreamStats, QueueSizes};
use crate::gulf_stream::utils::now_ms;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct HighPrioItem {
    pub priority_fee: u64,
    pub timestamp_ms: u128,
    pub tx_hash: Vec<u8>,
}

pub struct CompassGulfStreamManager {
    pub node_id: String,
    pub capacity: usize,
    pub pending_transactions: HashMap<Vec<u8>, CompassGulfStreamTransaction>,
    pub processing_transactions: HashMap<Vec<u8>, CompassGulfStreamTransaction>,
    pub high_priority_queue: Vec<HighPrioItem>,
    pub normal_priority_queue: VecDeque<Vec<u8>>,
    pub low_priority_queue: VecDeque<Vec<u8>>,
    pub validator_schedule: Vec<ValidatorSlot>,
    pub current_slot: Option<ValidatorSlot>,
    pub next_leader: Option<String>,
    pub transactions_received: u64,
    pub transactions_confirmed: u64,
    pub transactions_rejected: u64,
}

impl CompassGulfStreamManager {
    /// Constructor
    pub fn new(node_id: String, capacity: usize) -> Self {
        CompassGulfStreamManager {
            node_id,
            capacity,
            pending_transactions: HashMap::new(),
            processing_transactions: HashMap::new(),
            high_priority_queue: Vec::new(),
            normal_priority_queue: VecDeque::new(),
            low_priority_queue: VecDeque::new(),
            validator_schedule: Vec::new(),
            current_slot: None,
            next_leader: None,
            transactions_received: 0,
            transactions_confirmed: 0,
            transactions_rejected: 0,
        }
    }

    /// Add a transaction into the Gulf Stream queues
    pub fn add_transaction(&mut self, tx_hash: Vec<u8>, raw_tx: Vec<u8>, priority_fee: u64) -> bool {
        if self.pending_transactions.contains_key(&tx_hash) {
            println!("Transaction already exists");
            return false;
        }

        let gs_tx = CompassGulfStreamTransaction::new(tx_hash.clone(), raw_tx, priority_fee);

        if priority_fee > 1000 {
            self.high_priority_queue.push(HighPrioItem {
                priority_fee,
                timestamp_ms: gs_tx.timestamp_ms,
                tx_hash: tx_hash.clone(),
            });
        } else if priority_fee > 100 {
            self.normal_priority_queue.push_back(tx_hash.clone());
        } else {
            self.low_priority_queue.push_back(tx_hash.clone());
        }

        self.pending_transactions.insert(tx_hash.clone(), gs_tx);
        self.transactions_received += 1;
        true
    }

    /// Confirm a transaction (move from pending → confirmed)
    pub fn confirm_transaction(&mut self, tx_hash: &Vec<u8>) -> bool {
        if let Some(tx) = self.pending_transactions.remove(tx_hash) {
            self.processing_transactions.insert(tx_hash.clone(), tx);
            self.transactions_confirmed += 1;
            println!("Transaction {:?} confirmed", tx_hash);
            true
        } else {
            println!("Transaction {:?} not found in pending", tx_hash);
            false
        }
    }

    /// Reject a transaction (remove from pending)
    pub fn reject_transaction(&mut self, tx_hash: &Vec<u8>) -> bool {
        if self.pending_transactions.remove(tx_hash).is_some() {
            self.transactions_rejected += 1;
            println!("Transaction {:?} rejected", tx_hash);
            true
        } else {
            println!("Transaction {:?} not found in pending", tx_hash);
            false
        }
    }

    /// Cleanup expired transactions
    pub fn cleanup_expired_transactions(&mut self, max_age_seconds: u64) {
        let cutoff = now_ms() - (max_age_seconds as u128 * 1000);
        self.pending_transactions.retain(|_, tx| tx.timestamp_ms >= cutoff);
        self.processing_transactions.retain(|_, tx| tx.timestamp_ms >= cutoff);
    }

    /// Update current slot based on validator schedule
    pub fn update_current_slot(&mut self) {
        let now = now_ms();
        self.current_slot = self.validator_schedule
            .iter()
            .find(|slot| slot.start_time_ms <= now && now <= slot.end_time_ms)
            .cloned();
    }

    /// Update next leader based on current slot
    pub fn update_next_leader(&mut self) {
        if let Some(current) = &self.current_slot {
            let next_slot_number = current.slot_number + 1;
            self.next_leader = self.validator_schedule
                .iter()
                .find(|slot| slot.slot_number == next_slot_number)
                .map(|slot| slot.validator_id.clone());
        }
    }

    /// Get Gulf Stream stats
    pub fn get_stats(&self) -> GulfStreamStats {
        GulfStreamStats {
            transactions_received: self.transactions_received,
            transactions_forwarded: 0,
            transactions_confirmed: self.transactions_confirmed,
            transactions_rejected: self.transactions_rejected,
            pending_transactions: self.pending_transactions.len() as u64,
            processing_transactions: self.processing_transactions.len() as u64,
            confirmed_transactions: self.transactions_confirmed,
            rejected_transactions: self.transactions_rejected,
            avg_confirmation_time_ms: 0.0,
            current_slot: self.current_slot.as_ref().map(|s| s.validator_id.clone()),
            next_leader: self.next_leader.clone(),
            queue_sizes: QueueSizes {
                high_priority: self.high_priority_queue.len() as u64,
                normal_priority: self.normal_priority_queue.len() as u64,
                low_priority: self.low_priority_queue.len() as u64,
            },
        }
    }
}