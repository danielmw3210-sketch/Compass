use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::layer3::model_nft::ModelNFT;
use crate::storage::Storage;

#[derive(Clone, Serialize, Deserialize)]
pub struct AssetManager {
    // Map: Token ID -> ModelNFT
    pub registry: HashMap<String, ModelNFT>,
    // Map: Owner Address -> List of Token IDs
    pub ownership: HashMap<String, Vec<String>>,
    
    // Persistence (Skipped in serialization to avoid cyclic/invalid states)
    #[serde(skip)]
    pub storage: Option<Arc<Storage>>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            ownership: HashMap::new(),
            storage: None,
        }
    }
    
    pub fn set_storage(&mut self, storage: Arc<Storage>) {
        self.storage = Some(storage);
    }

    /// Register a mint event (Called after checks pass)
    pub fn register_mint(&mut self, nft: ModelNFT, owner: String) {
        let id = nft.token_id.clone();
        
        // Add to registry
        self.registry.insert(id.clone(), nft.clone());

        // Add to ownership
        self.ownership.entry(owner).or_default().push(id);
        
        // Persist individual NFT to Sled (in addition to AssetManager blob)
        if let Some(db) = &self.storage {
            if let Err(e) = db.save_model_nft(&nft) {
                tracing::error!("Failed to save Model NFT to Sled: {}", e);
            } else {
                tracing::info!("✅ Persisted NFT to Sled: {}", nft.token_id);
            }
        }
        
        // Persist entire AssetManager state
        self.persist();
    }
    
    fn persist(&self) {
        if let Some(db) = &self.storage {
            // Save entire manager as blob "l2:assets" for simple v1.3 persistence
            // This matches Layer2State::load_from_db logic
            let _ = db.put("l2:assets", self);
        }
    }

    pub fn get_asset(&self, token_id: &str) -> Option<&ModelNFT> {
        self.registry.get(token_id)
    }

    pub fn transfer(&mut self, token_id: &str, _from: &str, to: &str) -> Result<(), String> {
        if !self.registry.contains_key(token_id) {
            return Err("Asset does not exist".to_string());
        }
        
        // 1. Remove from old owner
        if let Some(assets) = self.ownership.get_mut(_from) {
            if let Some(pos) = assets.iter().position(|x| x == token_id) {
                assets.remove(pos);
            } else {
                return Err("Sender does not own this asset".to_string());
            }
        } else {
             return Err("Sender has no assets".to_string());
        }

        // 2. Add to new owner
        self.ownership.entry(to.to_string()).or_default().push(token_id.to_string());
        
        // Persist State
        self.persist();

        Ok(())
    }
}

