use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompassConfig {
    pub node: NodeConfig,
    pub consensus: ConsensusConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodeConfig {
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub db_path: String,
    pub log_level: String,
    #[serde(default = "default_identity_file")]
    pub identity_file: String,
    #[serde(default = "default_bootnodes")]
    pub bootnodes: Vec<String>,
}

fn default_identity_file() -> String {
    "identity.json".to_string()
}

fn default_bootnodes() -> Vec<String> {
    vec![]
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConsensusConfig {
    pub slot_duration_ms: u64,
}

impl Default for CompassConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                p2p_port: 19000,
                rpc_port: 9000,
                db_path: "./data/primary".to_string(),
                log_level: "info".to_string(),
                identity_file: "identity.json".to_string(),
                bootnodes: vec![],
            },
            consensus: ConsensusConfig {
                slot_duration_ms: 1000,
            },
        }
    }
}

impl CompassConfig {
    pub fn load_or_default(path: &str) -> Self {
        if std::path::Path::new(path).exists() {
             match std::fs::read_to_string(path) {
                 Ok(s) => match toml::from_str(&s) {
                     Ok(c) => {
                         println!("Config loaded from {}", path);
                         c
                     },
                     Err(e) => {
                         eprintln!("Error parsing config: {}. Using Defaults.", e);
                         Self::default()
                     }
                 },
                 Err(e) => {
                     eprintln!("Error reading config: {}. Using Defaults.", e);
                     Self::default()
                 }
             }
        } else {
            println!("Config file not found at '{}'. Creating default.", path);
            let config = Self::default();
            if let Ok(s) = toml::to_string_pretty(&config) {
                let _ = std::fs::write(path, s);
            }
            config
        }
    }
}
