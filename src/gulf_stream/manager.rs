impl CompassGulfStreamManager {
    /// Constructor
    pub fn new(node_id: String, capacity: usize) -> Self {
        CompassGulfStreamManager {
            node_id,
            capacity,
            pending_transactions: std::collections::HashMap::new(),
            processing_transactions: std::collections::HashMap::new(),
            high_priority_queue: Vec::new(),
            normal_priority_queue: std::collections::VecDeque::new(),
            low_priority_queue: std::collections::VecDeque::new(),
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
            received: self.transactions_received,
            confirmed: self.transactions_confirmed,
            rejected: self.transactions_rejected,
            queue_sizes: QueueSizes {
                high: self.high_priority_queue.len(),
                normal: self.normal_priority_queue.len(),
                low: self.low_priority_queue.len(),
            },
        }
    }
}