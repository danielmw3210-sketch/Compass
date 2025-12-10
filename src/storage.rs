use rocksdb::{DB, Options};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::Path;

pub struct Storage {
    db: Arc<DB>,
}

impl Storage {
    pub fn new(path: &str) -> Self {
        let path = Path::new(path);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path).unwrap();
        Storage {
            db: Arc::new(db),
        }
    }

    // Generic Helper: Put
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        let serialized = bincode::serialize(value).map_err(|e| e.to_string())?;
        self.db.put(key.as_bytes(), serialized).map_err(|e| e.to_string())
    }

    // Generic Helper: Get
    pub fn get<T: for<'a> Deserialize<'a>>(&self, key: &str) -> Result<Option<T>, String> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => {
                let deserialized = bincode::deserialize(&data).map_err(|e| e.to_string())?;
                Ok(Some(deserialized))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // --- Specific Accessors ---

    // 1. Blocks
    pub fn save_block(&self, block: &crate::block::Block) -> Result<(), String> {
        // Save by Hash
        let hash = &block.header.hash; // String?
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
}
