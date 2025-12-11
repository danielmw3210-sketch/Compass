use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Storage {
    db: Arc<DB>,
}

impl Storage {
    pub fn new(path: &str) -> Self {
        let path = Path::new(path);
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path).unwrap();
        Storage { db: Arc::new(db) }
    }

    // Generic Helper: Put
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        let serialized = bincode::serialize(value).map_err(|e| e.to_string())?;
        self.db
            .put(key.as_bytes(), serialized)
            .map_err(|e| e.to_string())
    }

    // Generic Helper: Get
    pub fn get<T: for<'a> Deserialize<'a>>(&self, key: &str) -> Result<Option<T>, String> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => {
                let deserialized = bincode::deserialize(&data).map_err(|e| e.to_string())?;
                Ok(Some(deserialized))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // --- Specific Accessors ---

    // 1. Blocks
    pub fn save_block(&self, block: &crate::block::Block) -> Result<(), String> {
        // Save by Hash
        // Save by Hash
        // Save by Hash
        let hash = &block.header.hash;
        if hash.is_empty() {
            return Err("Block has no hash".to_string());
        }
        // Convert hex hash to key "block:<hash>"
        self.put(&format!("block:{}", hash), block)?;

        // Save by Height "height:<index>" -> hash
        self.put(&format!("height:{}", block.header.index), hash)?;

        // Update Head? Handled by Chain logic, but storage could track it too.
        Ok(())
    }

    pub fn get_block(&self, hash: &str) -> Result<Option<crate::block::Block>, String> {
        self.get(&format!("block:{}", hash))
    }

    pub fn get_block_by_height(&self, height: u64) -> Result<Option<crate::block::Block>, String> {
        if let Ok(Some(hash)) = self.get::<String>(&format!("height:{}", height)) {
            self.get_block(&hash)
        } else {
            Ok(None)
        }
    }

    // 2. Account State (Balances and Nonces)
    pub fn get_balance(&self, wallet_id: &str, asset: &str) -> Result<u64, String> {
        let key = format!("balance:{}:{}", wallet_id, asset);
        Ok(self.get::<u64>(&key)?.unwrap_or(0))
    }

    pub fn set_balance(&self, wallet_id: &str, asset: &str, amount: u64) -> Result<(), String> {
        let key = format!("balance:{}:{}", wallet_id, asset);
        self.put(&key, &amount)
    }

    pub fn get_nonce(&self, wallet_id: &str) -> Result<u64, String> {
        let key = format!("nonce:{}", wallet_id);
        Ok(self.get::<u64>(&key)?.unwrap_or(0))
    }

    pub fn set_nonce(&self, wallet_id: &str, nonce: u64) -> Result<(), String> {
        let key = format!("nonce:{}", wallet_id);
        self.put(&key, &nonce)
    }

    // 3. Validator Stats
    pub fn get_validator_stats(&self, validator: &str) -> std::result::Result<crate::rpc::types::ValidatorStats, String> {
         let key = format!("stats:{}", validator);
         Ok(self.get(&key)?.unwrap_or_default())
    }

    pub fn set_validator_stats(&self, validator: &str, stats: &crate::rpc::types::ValidatorStats) -> Result<(), String> {
        let key = format!("stats:{}", validator);
        self.put(&key, stats)
    }
}
