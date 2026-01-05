//! Account type definitions for Compass v2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Account identifier - human-readable name
pub type AccountId = String;

/// Asset identifier (COMPASS, cBTC, COMPUTE, etc.)
pub type Asset = String;

/// Main account structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    // Identity
    pub name: AccountId,
    pub account_type: AccountType,
    
    // Authentication
    pub password_hash: String,     // Argon2id hash
    pub salt: Vec<u8>,             // Random salt (32 bytes)
    
    // Recovery
    pub backup_seed_encrypted: Vec<u8>,  // BIP39 seed encrypted with password
    pub recovery_pubkey: String,         // Public key derived from seed
    
    // Authorization (for transaction signing)
    pub signing_pubkey: String,
    pub signing_privkey_encrypted: Vec<u8>,  // Private key encrypted with password
    
    // State
    pub nonce: u64,
    pub created_at: u64,
    pub metadata: HashMap<String, String>,
}

/// Different types of accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AccountType {
    /// Standard user account
    User(UserAccountData),
    
    /// Oracle node account (requires 100K COMPASS stake)
    Oracle(OracleAccountData),
    
    /// Validator account (future use)
    Validator(ValidatorAccountData),
    
    /// Admin account (genesis account with special privileges)
    Admin(AdminAccountData),
}

/// User account data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserAccountData {
    pub display_name: Option<String>,
    pub encryption_pubkey: Option<String>,
    pub preferences: HashMap<String, String>,
}

/// Oracle account data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OracleAccountData {
    pub oracle_type: OracleType,
    pub stake_amount: u64,         // Must be >= 100,000 COMPASS
    pub stake_locked_until: u64,   // Timestamp
    pub reputation_score: f64,     // 0.0 - 1.0
    pub total_submissions: u64,
    pub correct_submissions: u64,
    pub supported_feeds: Vec<String>,
    pub endpoint: Option<String>,  // Oracle API endpoint
}

/// Types of oracle data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OracleType {
    PriceFeed,           // Crypto price data
    ExternalChain,       // BTC/LTC/SOL deposit verification
    Custom(String),      // Future: weather, sports, etc.
}

/// Validator account data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ValidatorAccountData {
    pub stake_amount: u64,
    pub commission_rate: f64,  // 0.0 - 1.0
    pub total_blocks: u64,
    pub is_active: bool,
}

/// Admin account data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AdminAccountData {
    pub permissions: Vec<String>,
}

impl Account {
    /// Check if account has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self.account_type, AccountType::Admin(_))
    }
    
    /// Check if account is an oracle
    pub fn is_oracle(&self) -> bool {
        matches!(self.account_type, AccountType::Oracle(_))
    }
    
    /// Check if account can sign transactions
    pub fn can_sign(&self) -> bool {
        !self.signing_privkey_encrypted.is_empty()
    }
    
    /// Get oracle data if this is an oracle account
    pub fn oracle_data(&self) -> Option<&OracleAccountData> {
        match &self.account_type {
            AccountType::Oracle(data) => Some(data),
            _ => None,
        }
    }
    
    /// Get mutable oracle data if this is an oracle account
    pub fn oracle_data_mut(&mut self) -> Option<&mut OracleAccountData> {
        match &mut self.account_type {
            AccountType::Oracle(data) => Some(data),
            _ => None,
        }
    }
}

impl Default for UserAccountData {
    fn default() -> Self {
        Self {
            display_name: None,
            encryption_pubkey: None,
            preferences: HashMap::new(),
        }
    }
}

impl Default for AdminAccountData {
    fn default() -> Self {
        Self {
            permissions: vec!["*".to_string()], // Full permissions
        }
    }
}
