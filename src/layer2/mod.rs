pub mod economics;
pub mod assets;
pub mod collateral;

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::storage::Storage;

/// Global State for Layer 2
/// Note: We keep Serialize/Deserialize for components, but the State itself is now DB-managed.
#[derive(Clone)] // Removed Ser/De from top level as it holds Arc<Storage>
pub struct Layer2State {
    pub economics: economics::TokenomicsEngine,
    pub assets: assets::AssetManager,
    pub collateral: collateral::CollateralManager,
    
    // DB Access (Skipped during Component serialization)
    storage: Option<Arc<Storage>>,
}

impl Layer2State {
    pub fn new(storage: Option<Arc<Storage>>) -> Self {
        let mut state = Self {
            economics: economics::TokenomicsEngine::new(),
            assets: assets::AssetManager::new(),
            collateral: collateral::CollateralManager::new(),
            storage: storage.clone(),
        };
        
        // If storage exists, try to load state
        let db_opt = state.storage.clone();
        if let Some(db) = db_opt {
             state.load_from_db(&db);
        }
        
        state
    }

    fn load_from_db(&mut self, db: &Storage) {
        println!("Persistence: Loading Layer 2 State from DB...");
        
        // 1. Load Custom "Chunks" if we saved them that way
        // Economics
        if let Ok(Some(eco)) = db.get::<economics::TokenomicsEngine>("l2:economics") {
            self.economics = eco;
        }

        // Collateral
        if let Ok(Some(col)) = db.get::<collateral::CollateralManager>("l2:collateral") {
            self.collateral = col;
        }

        // Assets (Special Handling: Load Registry)
        // For now, simpler to load entire AssetManager Blob if we save it as blob
        // But plan was Write-Through.
        // Let's assume for v1.3 Phase 2 Step 1 we save AssetManager as blob "l2:assets"
        // Granular updates come later or via internal Storage usage.
        if let Ok(Some(assets)) = db.get::<assets::AssetManager>("l2:assets") {
            self.assets = assets;
        }
        
        // Inject Storage for Write-Through components
        if let Some(db_arc) = &self.storage {
             self.assets.set_storage(db_arc.clone());
        }
    }

    pub fn save(&self, _path: &str) -> Result<(), std::io::Error> {
        // Deprecated JSON path usage.
        // We trigger DB save.
        if let Some(db) = &self.storage {
            // Save Components
            let _ = db.put("l2:economics", &self.economics);
            let _ = db.put("l2:collateral", &self.collateral);
            let _ = db.put("l2:assets", &self.assets); // TODO: Make granular
            
            let _ = db.flush();
        }
        Ok(())
    }
}
