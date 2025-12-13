use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub timestamp: u64,
    pub initial_balances: HashMap<String, u64>,
    #[serde(default)]
    pub initial_validators: Vec<GenesisValidator>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenesisValidator {
    pub id: String,
    pub public_key: String,
    pub stake: u64,
}

impl GenesisConfig {
    pub fn load(path: &str) -> Result<Self, String> {
        if !Path::new(path).exists() {
            return Err(format!("Genesis file not found: {}", path));
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }
}

/// Generates a default genesis configuration
pub fn generate_admin_config() {
    println!("Generating default genesis.json...");
    
    let mut initial_balances = HashMap::new();
    initial_balances.insert("admin".to_string(), 1_000_000_000_000);
    initial_balances.insert("foundation".to_string(), 1_000_000_000_000);

    let config = GenesisConfig {
        chain_id: "compass-mainnet".to_string(),
        timestamp: 0, // In real world use Utc::now()
        initial_balances,
        initial_validators: vec![],
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    if let Err(e) = fs::write("genesis.json", json) {
        println!("Error writing genesis.json: {}", e);
    } else {
        println!("Created 'genesis.json'.");
    }
}
