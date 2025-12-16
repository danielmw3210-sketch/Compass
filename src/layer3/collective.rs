#![allow(dead_code)]
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::layer3::data::MarketContext;

/// Collective Intelligence: Shared Memory Pool
/// All oracle nodes contribute and learn from this pool
#[derive(Serialize, Deserialize, Clone)]
pub struct SharedMemoryPool {
    /// Experiences contributed by all nodes
    pub experiences: Vec<CollectiveExperience>,
    /// Maximum capacity (500K across all nodes)
    pub max_capacity: usize,
    /// Quality score threshold for inclusion
    pub min_quality_score: f64,
}

/// Single experience contributed by a node
#[derive(Serialize, Deserialize, Clone)]
pub struct CollectiveExperience {
    pub context: MarketContext,
    pub labels: Vec<f64>,  // 5 labels (L1/L2, BTC, ETH, SOL, meta)
    pub contributor: String,  // Worker ID
    pub timestamp: u64,
    pub quality_score: f64,  // Based on contributor's accuracy
    pub votes: HashMap<String, bool>,  // Consensus voting
}

impl SharedMemoryPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            experiences: Vec::new(),
            max_capacity: capacity,
            min_quality_score: 0.5,
        }
    }

    /// Contribute experiences to the collective pool
    pub fn contribute(&mut self, contexts: Vec<MarketContext>, labels: Vec<Vec<f64>>, 
                      contributor: String, quality_score: f64) {
        for (ctx, label_vec) in contexts.iter().zip(labels.iter()) {
            if quality_score >= self.min_quality_score {
                let exp = CollectiveExperience {
                    context: ctx.clone(),
                    labels: label_vec.clone(),
                    contributor: contributor.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    quality_score,
                    votes: HashMap::new(),
                };
                
                self.experiences.push(exp);
            }
        }

        // Maintain capacity by removing lowest quality samples
        if self.experiences.len() > self.max_capacity {
            self.experiences.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());
            self.experiences.truncate(self.max_capacity);
        }

        println!("   🌐 Contributed to collective pool (Total: {} experiences)", 
                 self.experiences.len());
    }

    /// Get top N experiences for training
    pub fn get_top_experiences(&self, n: usize) -> Vec<(MarketContext, Vec<f64>)> {
        self.experiences.iter()
            .take(n.min(self.experiences.len()))
            .map(|exp| (exp.context.clone(), exp.labels.clone()))
            .collect()
    }

    /// Get experiences contributed by specific node
    pub fn get_by_contributor(&self, contributor: &str) -> Vec<&CollectiveExperience> {
        self.experiences.iter()
            .filter(|exp| exp.contributor == contributor)
            .collect()
    }

    /// Save pool to disk
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load pool from disk
    pub fn load(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Federated Learning: Gradient Sharing
#[derive(Serialize, Deserialize, Clone)]
pub struct GradientUpdate {
    pub worker_id: String,
    pub timestamp: u64,
    pub w1_delta: Vec<Vec<f64>>,  // 7x32
    pub w2_delta: Vec<Vec<f64>>,  // 32x5
    pub contribution_score: f64,  // Based on accuracy
    pub samples_trained: usize,
}

impl GradientUpdate {
    pub fn new(worker_id: String, w1: Vec<Vec<f64>>, w2: Vec<Vec<f64>>, 
               score: f64, samples: usize) -> Self {
        Self {
            worker_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            w1_delta: w1,
            w2_delta: w2,
            contribution_score: score,
            samples_trained: samples,
        }
    }
}

/// Federated Learning Server
#[derive(Serialize, Deserialize, Clone)]
pub struct FederatedServer {
    pub gradient_updates: Vec<GradientUpdate>,
    pub round: usize,
    pub min_contributors: usize,
}

impl FederatedServer {
    pub fn new() -> Self {
        Self {
            gradient_updates: Vec::new(),
            round: 0,
            min_contributors: 3,  // Need at least 3 nodes for averaging
        }
    }

    /// Submit gradients to federated server
    pub fn submit_gradients(&mut self, update: GradientUpdate) {
        self.gradient_updates.push(update);
        println!("   📡 Gradient submitted to federated server (Round {})", self.round);
    }

    /// Aggregate gradients using weighted averaging
    pub fn aggregate_gradients(&mut self) -> Option<(Vec<Vec<f64>>, Vec<Vec<f64>>)> {
        if self.gradient_updates.len() < self.min_contributors {
            println!("   ⏳ Waiting for more gradient contributions ({}/{})", 
                     self.gradient_updates.len(), self.min_contributors);
            return None;
        }

        let total_weight: f64 = self.gradient_updates.iter()
            .map(|u| u.contribution_score)
            .sum();

        // Weighted average for W1
        let mut avg_w1 = vec![vec![0.0; 32]; 7];
        for update in &self.gradient_updates {
            let weight = update.contribution_score / total_weight;
            for i in 0..7 {
                for j in 0..32 {
                    avg_w1[i][j] += update.w1_delta[i][j] * weight;
                }
            }
        }

        // Weighted average for W2
        let mut avg_w2 = vec![vec![0.0; 5]; 32];
        for update in &self.gradient_updates {
            let weight = update.contribution_score / total_weight;
            for i in 0..32 {
                for j in 0..5 {
                    avg_w2[i][j] += update.w2_delta[i][j] * weight;
                }
            }
        }

        // Clear for next round
        self.gradient_updates.clear();
        self.round += 1;

        println!("   ✅ Gradients aggregated! Federated round {} complete.", self.round);

        Some((avg_w1, avg_w2))
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

/// Model Marketplace: Buy/Sell Trained Models
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelListing {
    pub model_id: String,
    pub owner: String,
    pub accuracy: f64,
    pub win_rate: f64,
    pub total_predictions: usize,
    pub price: u64,  // COMPASS tokens
    pub model_hash: String,  // Hash of weights
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelMarketplace {
    pub listings: Vec<ModelListing>,
    pub sales: Vec<ModelSale>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelSale {
    pub buyer: String,
    pub seller: String,
    pub model_id: String,
    pub price: u64,
    pub timestamp: u64,
}

impl ModelMarketplace {
    pub fn new() -> Self {
        Self {
            listings: Vec::new(),
            sales: Vec::new(),
        }
    }

    /// List a model for sale
    pub fn list_model(&mut self, listing: ModelListing) {
        println!("   💰 Model listed on marketplace: {} COMPASS (Accuracy: {:.1}%)", 
                 listing.price, listing.accuracy * 100.0);
        self.listings.push(listing);
    }

    /// Get top performing models
    pub fn get_top_models(&self, n: usize) -> Vec<&ModelListing> {
        let mut sorted = self.listings.clone();
        sorted.sort_by(|a, b| b.accuracy.partial_cmp(&a.accuracy).unwrap());
        sorted.iter().take(n).map(|_| &self.listings[0]).collect()
    }

    /// Buy a model (returns model_id to download weights)
    pub fn buy_model(&mut self, model_id: &str, buyer: String) -> Option<String> {
        let listing = self.listings.iter().find(|l| l.model_id == model_id)?;
        
        let sale = ModelSale {
            buyer: buyer.clone(),
            seller: listing.owner.clone(),
            model_id: model_id.to_string(),
            price: listing.price,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.sales.push(sale);
        println!("   ✅ Model purchased! {} paid {} COMPASS to {}", 
                 buyer, listing.price, listing.owner);

        Some(model_id.to_string())
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

/// A Shared Model Pool allows users to pool COMPASS tokens to co-own a high-performance model.
/// Royalties from the model are distributed to contributors based on their stake share.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPool {
    /// Unique ID of the pool (e.g., "pool-btc-alpha")
    pub pool_id: String,
    /// Display name
    pub name: String,
    /// Type of model (e.g., "signal-classifier", "gpt-4-finetune")
    pub model_type: String, 
    /// Total COMPASS staked in this pool
    pub total_staked: u64,
    /// Map of Wallet Address -> Staked Amount
    pub contributors: HashMap<String, u64>,
    /// Accumulated rewards (royalties) ready for distribution
    pub vault_balance: u64, 
    /// Epoch count (for tracking performance)
    pub epoch: u64,
}

impl ModelPool {
    /// Create a new empty pool
    pub fn new(id: String, name: String, model_type: String) -> Self {
        ModelPool {
            pool_id: id,
            name,
            model_type,
            total_staked: 0,
            contributors: HashMap::new(),
            vault_balance: 0,
            epoch: 0,
        }
    }

    /// Add stake from a contributor
    pub fn add_stake(&mut self, contributor: String, amount: u64) {
        let current = self.contributors.entry(contributor.clone()).or_insert(0);
        *current += amount;
        self.total_staked += amount;
    }

    /// Remove stake (if allowed, currently locked)
    pub fn remove_stake(&mut self, contributor: &str, amount: u64) -> Result<(), String> {
        if let Some(balance) = self.contributors.get_mut(contributor) {
            if *balance >= amount {
                *balance -= amount;
                self.total_staked -= amount;
                if *balance == 0 {
                    self.contributors.remove(contributor);
                }
                Ok(())
            } else {
                Err("Insufficient stake".to_string())
            }
        } else {
            Err("Not a contributor".to_string())
        }
    }

    /// Calculate share percentage for a user (0.0 to 1.0)
    pub fn get_share(&self, contributor: &str) -> f64 {
        if self.total_staked == 0 { return 0.0; }
        let stake = self.contributors.get(contributor).copied().unwrap_or(0);
        stake as f64 / self.total_staked as f64
    }

    /// Add revenue to the pool's vault
    pub fn add_revenue(&mut self, amount: u64) {
        self.vault_balance += amount;
    }
}
