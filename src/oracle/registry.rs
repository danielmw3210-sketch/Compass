//! Oracle Registry for Layer 1 State
//! 
//! Manages registered oracles and their staking requirements

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::account::AccountId;

/// Minimum COMPASS stake required to become an oracle
pub const ORACLE_MIN_STAKE: u64 = 100_000 * 1_000_000; // 100,000 COMPASS (assuming 6 decimals)

/// Oracle status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OracleStatus {
    Active,
    Inactive,
    Slashed { reason: String },
}

/// Registered oracle
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisteredOracle {
    pub account_id: AccountId,
    pub stake_amount: u64,
    pub registration_block: u64,
    pub status: OracleStatus,
    pub reputation_score: f64,        // 0.0 - 1.0
    pub total_submissions: u64,
    pub correct_submissions: u64,
    pub incorrect_submissions: u64,
    pub last_submission_block: u64,
    pub supported_feeds: Vec<String>,  // BTCUSD, ETHUSD, etc.
}

impl RegisteredOracle {
    /// Create a new oracle registration
    pub fn new(account_id: AccountId, stake_amount: u64, registration_block: u64) -> Self {
        Self {
            account_id,
            stake_amount,
            registration_block,
            status: OracleStatus::Active,
            reputation_score: 1.0,
            total_submissions: 0,
            correct_submissions: 0,
            incorrect_submissions: 0,
            last_submission_block: registration_block,
            supported_feeds: vec![],
        }
    }
    
    /// Check if oracle is active
    pub fn is_active(&self) -> bool {
        self.status == OracleStatus::Active && self.stake_amount >= ORACLE_MIN_STAKE
    }
    
    /// Record a submission
    pub fn record_submission(&mut self, block_height: u64, is_correct: bool) {
        self.total_submissions += 1;
        if is_correct {
            self.correct_submissions += 1;
        } else {
            self.incorrect_submissions += 1;
        }
        self.last_submission_block = block_height;
        
        // Update reputation score
        self.update_reputation();
    }
    
    /// Update reputation based on correctness
    fn update_reputation(&mut self) {
        if self.total_submissions == 0 {
            return;
        }
        
        self.reputation_score = self.correct_submissions as f64 / self.total_submissions as f64;
    }
    
    /// Slash oracle for misbehavior
    pub fn slash(&mut self, reason: String, slash_amount: u64) {
        self.status = OracleStatus::Slashed { reason };
        self.stake_amount = self.stake_amount.saturating_sub(slash_amount);
    }
}

/// Oracle registry managing all oracles
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OracleRegistry {
    oracles: HashMap<AccountId, RegisteredOracle>,
}

impl OracleRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            oracles: HashMap::new(),
        }
    }
    
    /// Register a new oracle
    pub fn register_oracle(
        &mut self,
        account_id: AccountId,
        stake_amount: u64,
        block_height: u64,
    ) -> Result<(), String> {
        if self.oracles.contains_key(&account_id) {
            return Err("Oracle already registered".to_string());
        }
        
        if stake_amount < ORACLE_MIN_STAKE {
            return Err(format!(
                "Insufficient stake: {} < {} required",
                stake_amount,
                ORACLE_MIN_STAKE
            ));
        }
        
        let oracle = RegisteredOracle::new(account_id.clone(), stake_amount, block_height);
        self.oracles.insert(account_id, oracle);
        
        Ok(())
    }
    
    /// Get oracle by account ID
    pub fn get_oracle(&self, account_id: &AccountId) -> Option<&RegisteredOracle> {
        self.oracles.get(account_id)
    }
    
    /// Get mutable oracle by account ID
    pub fn get_oracle_mut(&mut self, account_id: &AccountId) -> Option<&mut RegisteredOracle> {
        self.oracles.get_mut(account_id)
    }
    
    /// Get all active oracles
    pub fn active_oracles(&self) -> Vec<&RegisteredOracle> {
        self.oracles
            .values()
            .filter(|o| o.is_active())
            .collect()
    }
    
    /// Get count of active oracles
    pub fn active_count(&self) -> usize {
        self.active_oracles().len()
    }
    
    /// Check if account is a registered oracle
    pub fn is_oracle(&self, account_id: &AccountId) -> bool {
        self.oracles
            .get(account_id)
            .map(|o| o.is_active())
            .unwrap_or(false)
    }
}

impl Default for OracleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Price feed submission from oracle
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OraclePriceSubmission {
    pub oracle_account: AccountId,
    pub ticker: String,
    pub price: f64,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_oracle_registration() {
        let mut registry = OracleRegistry::new();
        
        // Should succeed with sufficient stake
        assert!(registry.register_oracle(
            "alice".to_string(),
            ORACLE_MIN_STAKE,
            0
        ).is_ok());
        
        // Should fail with insufficient stake
        assert!(registry.register_oracle(
            "bob".to_string(),
            ORACLE_MIN_STAKE - 1,
            0
        ).is_err());
        
        // Should fail if already registered
        assert!(registry.register_oracle(
            "alice".to_string(),
            ORACLE_MIN_STAKE,
            0
        ).is_err());
    }
    
    #[test]
    fn test_reputation_tracking() {
        let mut oracle = RegisteredOracle::new("oracle1".to_string(), ORACLE_MIN_STAKE, 0);
        
        // Perfect record
        oracle.record_submission(1, true);
        oracle.record_submission(2, true);
        assert_eq!(oracle.reputation_score, 1.0);
        
        // One incorrect
        oracle.record_submission(3, false);
        assert!((oracle.reputation_score - 0.666).abs() < 0.01);
    }
}
