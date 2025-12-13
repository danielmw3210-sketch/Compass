use crate::gulf_stream::stats::{GulfStreamStats, QueueSizes};
use crate::gulf_stream::transactions::CompassGulfStreamTransaction;
use crate::gulf_stream::utils::now_ms;
use crate::gulf_stream::validator::ValidatorSlot;
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
    pub fn add_transaction(
        &mut self,
        tx_hash: Vec<u8>,
        raw_tx: Vec<u8>,
        priority_fee: u64,
    ) -> bool {
        if self.pending_transactions.contains_key(&tx_hash) {
            println!("Transaction already exists");
            return false;
        }

        // 1. Pre-Validate Signature (Defense against DoS)
        if let Ok(payload) = bincode::deserialize::<crate::network::TransactionPayload>(&raw_tx) {
            if !payload.verify() {
                 println!("GulfStream: REJECTED invalid signature for tx {:?}", hex::encode(&tx_hash));
                 self.transactions_rejected += 1;
                 return false;
            }
        } else {
             println!("GulfStream: REJECTED malformed transaction");
             self.transactions_rejected += 1;
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
        self.pending_transactions
            .retain(|_, tx| tx.timestamp_ms >= cutoff);
        self.processing_transactions
            .retain(|_, tx| tx.timestamp_ms >= cutoff);
    }

    /// Update current slot based on validator schedule
    pub fn update_current_slot(&mut self) {
        let now = now_ms();
        self.current_slot = self
            .validator_schedule
            .iter()
            .find(|slot| slot.start_time_ms <= now && now <= slot.end_time_ms)
            .cloned();
    }

    /// Update next leader based on current slot
    pub fn update_next_leader(&mut self) {
        if let Some(current) = &self.current_slot {
            let next_slot_number = current.slot_number + 1;
            self.next_leader = self
                .validator_schedule
                .iter()
                .find(|slot| slot.slot_number == next_slot_number)
                .map(|slot| slot.validator_id.clone());
        }
    }

    /// Retrieve a batch of transactions to forward/process, prioritized by fee
    pub fn pop_ready_transactions(&mut self, limit: usize) -> Vec<CompassGulfStreamTransaction> {
        let mut result = Vec::with_capacity(limit);
        let mut count = 0;

        // Helper to process a queue
        // We can't easily capture 'self' in a closure that modifies 'self', so we do it iteratively.
        // 1. High Priority
        while count < limit && !self.high_priority_queue.is_empty() {
             // Removing from Vec (swap_remove is O(1) but changes order, remove(0) is O(N).
             // Since it's a priority queue (sorted implied? No, just > 1000 fee).
             // Actually, the current implementation uses `push` for high prio, so it's a stack or we should treat it as queue?
             // `high_priority_queue` is `Vec<HighPrioItem>`. 
             // Ideally we want the highest fees. For now, let's just take from the "front" if we treat it as queue,
             // or "back" if we treat it as stack.
             // Given it is `Vec` and others are `VecDeque` (back/front), let's assume we want FIFO or simply high fee.
             // If we just `pop()` we get the last added. To be a queue we need `remove(0)` which is slow.
             // BUT, `high_priority_queue` is a `Vec`, likely small.
             // Let's sort it by priority before popping? That's expensive every time.
             // For this step, let's just pop from back (LIFO) or remove(0).
             // Let's use `remove(0)` for FIFO behavior on the Vec, accepting O(N) for now since high prio queue shouldn't be massive.
             // Or better, change `high_priority_queue` to `VecDeque` for O(1) pop_front?
             // The struct definition has `high_priority_queue: Vec<HighPrioItem>`.
             // Changing struct diffs is annoying. I will use `remove(0)`.
             
             let item = self.high_priority_queue.remove(0);
             if let Some(mut tx) = self.pending_transactions.remove(&item.tx_hash) {
                 tx.status = crate::gulf_stream::transactions::TransactionStatus::Processing;
                 self.processing_transactions.insert(item.tx_hash.clone(), tx.clone());
                 result.push(tx);
                 count += 1;
             }
        }

        // 2. Normal Priority (VecDeque)
        while count < limit {
            if let Some(tx_hash) = self.normal_priority_queue.pop_front() {
                if let Some(mut tx) = self.pending_transactions.remove(&tx_hash) {
                    tx.status = crate::gulf_stream::transactions::TransactionStatus::Processing;
                    self.processing_transactions.insert(tx_hash.clone(), tx.clone());
                    result.push(tx);
                    count += 1;
                }
            } else {
                break;
            }
        }

        // 3. Low Priority (VecDeque)
        while count < limit {
            if let Some(tx_hash) = self.low_priority_queue.pop_front() {
                 if let Some(mut tx) = self.pending_transactions.remove(&tx_hash) {
                    tx.status = crate::gulf_stream::transactions::TransactionStatus::Processing;
                    self.processing_transactions.insert(tx_hash.clone(), tx.clone());
                    result.push(tx);
                    count += 1;
                 }
            } else {
                break;
            }
        }

        result
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
