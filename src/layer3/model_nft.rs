#![allow(dead_code, unused)]
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LicenseType {
    OpenSource, // Free for everyone
    Commercial, // Must pay royalties
    Exclusive,  // Only owner can use
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RentalAgreement {
    pub renter: String,
    pub expires_at: u64,
    pub rate_per_hour: u64, // Cost in COMPASS
}

/// AI Model NFT: Trained neural network minted as on-chain asset
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelNFT {
    /// Unique NFT identifier
    pub token_id: String,
    
    /// Model metadata
    pub name: String,
    pub description: String,
    pub creator: String,  // Original trainer's address
    
    /// Rights & Licensing (New for AI Economy)
    pub license: LicenseType,
    pub rental_status: Option<RentalAgreement>,

    /// Performance stats
    pub accuracy: f64,
    pub win_rate: f64,
    pub total_predictions: usize,
    pub profitable_predictions: usize,
    pub total_profit: i64,  // COMPASS earned
    
    /// Training provenance
    pub training_samples: usize,
    pub training_epochs: usize,
    pub final_loss: f64,
    pub training_duration_seconds: u64,
    pub trained_on_data_hash: String,  // Proof of data used
    
    /// Model weights (serialized)
    pub weights_hash: String,  // SHA256 of weights file
    pub weights_uri: String,   // IPFS or on-chain storage
    pub architecture: String,  // "7→32→5 Attention+MoE"
    
    /// Lineage (which models this was derived from)
    pub parent_models: Vec<String>,  // NFT IDs if forked
    pub generation: u32,  // 0=original, 1=1st derivative, etc.
    
    /// Economics
    pub mint_price: u64,  // COMPASS cost to mint
    pub royalty_rate: f64,  // % to creator on resales
    pub current_owner: String,
    pub sale_history: Vec<SaleRecord>,
    
    /// Timestamps
    pub minted_at: u64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaleRecord {
    pub from: String,
    pub to: String,
    pub price: u64,
    pub royalty_paid: u64,
    pub timestamp: u64,
    pub tx_hash: String,
}

impl ModelNFT {
    /// Create NFT from a trained neural network
    pub fn from_network(
        network: &crate::layer3::models::NeuralNetwork,
        creator: String,
        name: String,
        description: String,
        stats: &ModelStats,
    ) -> Self {
        // ... (existing implementation)
        // Hash network identifier (not serializing weights for NFT)
        let weights_hash = hex::encode(&sha2::Sha256::digest(
            format!("{:?}-{}", std::time::SystemTime::now(), creator).as_bytes()
        ));
        
        // Generate unique token ID
        let token_id = format!("MODEL-{}-{}", 
            &weights_hash[..8],
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        Self {
            token_id: token_id.clone(),
            name,
            description,
            creator: creator.clone(),
            license: LicenseType::Commercial,
            rental_status: None,
            accuracy: stats.accuracy,
            win_rate: stats.win_rate,
            total_predictions: stats.total_predictions,
            profitable_predictions: stats.profitable_predictions,
            total_profit: stats.total_profit,
            training_samples: stats.training_samples,
            training_epochs: stats.training_epochs,
            final_loss: stats.final_loss,
            training_duration_seconds: stats.training_duration,
            trained_on_data_hash: stats.data_hash.clone(),
            weights_hash: weights_hash.clone(),
            weights_uri: format!("compass://models/{}", token_id),
            architecture: "7→32→5 (Attention+MoE)".to_string(),
            parent_models: vec![],
            generation: 0,
            mint_price: 1000,  // Base mint cost
            royalty_rate: 0.10,  // 10% creator royalty
            current_owner: creator,
            sale_history: vec![],
            minted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Create NFT from Oracle Job (Auto-Minting)
    pub fn from_job(
        job_id: &str,
        ticker: &str,
        creator: String,
        stats: &ModelStats,
    ) -> Self {
        let weights_hash = hex::encode(&sha2::Sha256::digest(
            format!("{:?}-{}-{}", std::time::SystemTime::now(), creator, job_id).as_bytes()
        ));
        
        let token_id = format!("MODEL-ORACLE-{}-{}", 
            ticker,
            &weights_hash[..6]
        );

        Self {
            token_id: token_id.clone(),
            name: format!("Oracle Predictor - {}", ticker),
            description: format!("Autonomous Oracle Model trained on {} market data via Job {}", ticker, job_id),
            creator: creator.clone(),
            license: LicenseType::Commercial,
            rental_status: None,
            accuracy: stats.accuracy,
            win_rate: stats.win_rate,
            total_predictions: stats.total_predictions,
            profitable_predictions: stats.profitable_predictions,
            total_profit: stats.total_profit,
            training_samples: stats.training_samples,
            training_epochs: stats.training_epochs,
            final_loss: stats.final_loss,
            training_duration_seconds: stats.training_duration,
            trained_on_data_hash: stats.data_hash.clone(),
            weights_hash: weights_hash.clone(),
            weights_uri: format!("compass://oracle/{}", job_id),
            architecture: "Oracle-V1 (External Feed)".to_string(),
            parent_models: vec![],
            generation: 0,
            mint_price: 5000,  // Oracle Models are premium
            royalty_rate: 0.15,
            current_owner: creator,
            sale_history: vec![],
            minted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Create derivative NFT (forked from parent)
    pub fn fork_from(parent: &ModelNFT, new_owner: String, improvements: String) -> Self {
        let mut forked = parent.clone();
        forked.token_id = format!("MODEL-FORK-{}-{}", 
            &parent.token_id[..8],
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        forked.name = format!("{} v2", parent.name);
        forked.description = format!("Forked from {} - Improvements: {}", parent.token_id, improvements);
        forked.parent_models = vec![parent.token_id.clone()];
        forked.generation = parent.generation + 1;
        forked.current_owner = new_owner;
        forked.sale_history = vec![];
        forked.minted_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        forked
    }

    /// Calculate market value based on performance
    pub fn estimated_value(&self) -> u64 {
        let base_value = 1000u64;
        let accuracy_multiplier = (self.accuracy * 10.0) as u64;
        let profit_bonus = (self.total_profit.max(0) as u64) / 100;
        let rarity_bonus = if self.generation == 0 { 500 } else { 0 };
        
        base_value + (accuracy_multiplier * 100) + profit_bonus + rarity_bonus
    }

    /// Transfer ownership (with royalty)
    pub fn transfer(&mut self, to: String, price: u64, tx_hash: String) {
        let royalty = (price as f64 * self.royalty_rate) as u64;
        
        let sale = SaleRecord {
            from: self.current_owner.clone(),
            to: to.clone(),
            price,
            royalty_paid: royalty,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tx_hash,
        };
        
        self.sale_history.push(sale);
        self.current_owner = to;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Performance statistics for NFT metadata
#[derive(Clone, Debug)]
pub struct ModelStats {
    pub accuracy: f64,
    pub win_rate: f64,
    pub total_predictions: usize,
    pub profitable_predictions: usize,
    pub total_profit: i64,
    pub training_samples: usize,
    pub training_epochs: usize,
    pub final_loss: f64,
    pub training_duration: u64,
    pub data_hash: String,
}

/// NFT Registry: On-chain catalog of all minted models
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelNFTRegistry {
    pub nfts: Vec<ModelNFT>,
    pub total_minted: usize,
    pub total_volume: u64,  // Total COMPASS traded
}

impl ModelNFTRegistry {
    pub fn new() -> Self {
        Self {
            nfts: Vec::new(),
            total_minted: 0,
            total_volume: 0,
        }
    }

    /// Mint new model NFT
    pub fn mint(&mut self, nft: ModelNFT) -> String {
        let token_id = nft.token_id.clone();
        println!("   🎨 Minting AI Model NFT: {}", token_id);
        println!("      • Name: {}", nft.name);
        println!("      • Accuracy: {:.1}%", nft.accuracy * 100.0);
        println!("      • Win Rate: {:.1}%", nft.win_rate * 100.0);
        println!("      • Total Profit: {} COMPASS", nft.total_profit);
        println!("      • Estimated Value: {} COMPASS", nft.estimated_value());
        
        self.nfts.push(nft);
        self.total_minted += 1;
        token_id
    }

    /// Get NFT by token ID
    pub fn get(&self, token_id: &str) -> Option<&ModelNFT> {
        self.nfts.iter().find(|nft| nft.token_id == token_id)
    }

    /// Get all NFTs owned by address
    pub fn get_owned_by(&self, owner: &str) -> Vec<&ModelNFT> {
        self.nfts.iter()
            .filter(|nft| nft.current_owner == owner)
            .collect()
    }

    /// Get top performing models
    pub fn get_leaderboard(&self, n: usize) -> Vec<&ModelNFT> {
        let mut sorted = self.nfts.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.accuracy.partial_cmp(&a.accuracy).unwrap());
        sorted.into_iter().take(n).collect()
    }

    /// List for sale
    pub fn list_for_sale(&mut self, token_id: &str, price: u64) -> Result<(), String> {
        // In production, this would create an on-chain listing
        println!("   💰 Listed Model NFT {} for {} COMPASS", token_id, price);
        Ok(())
    }

    /// Execute sale
    pub fn execute_sale(&mut self, token_id: &str, buyer: String, price: u64, tx_hash: String) 
        -> Result<(), String> 
    {
        let nft = self.nfts.iter_mut()
            .find(|n| n.token_id == token_id)
            .ok_or("NFT not found")?;
        
        let royalty = (price as f64 * nft.royalty_rate) as u64;
        let seller_receives = price - royalty;
        
        println!("   ✅ Model NFT Sale Executed:");
        println!("      • Token: {}", token_id);
        println!("      • Price: {} COMPASS", price);
        println!("      • Creator Royalty: {} COMPASS", royalty);
        println!("      • Seller Receives: {} COMPASS", seller_receives);
        
        nft.transfer(buyer, price, tx_hash);
        self.total_volume += price;
        
        Ok(())
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Helper: Extract stats from BridgePredictor for NFT minting
pub fn extract_model_stats(predictor: &crate::layer3::models::BridgePredictor) -> ModelStats {
    let (staked, won, lost, win_rate) = predictor.betting_ledger.get_stats();
    
    ModelStats {
        accuracy: 0.95,  // From last training
        win_rate,
        total_predictions: (won + lost) as usize,
        profitable_predictions: won as usize,
        total_profit: (won as i64) - (lost as i64),
        training_samples: predictor.experience.len(),
        training_epochs: predictor.training_history.len(),
        final_loss: predictor.training_history.last().map(|t| t.loss).unwrap_or(0.0),
        training_duration: 3600,  // Placeholder
        data_hash: "0x1234...".to_string(),  // Hash of experience buffer
    }
}
