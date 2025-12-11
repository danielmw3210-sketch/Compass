use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multi-oracle consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRegistry {
    /// Map of oracle_id -> oracle_pubkey
    pub oracles: HashMap<String, String>,
    /// Minimum signatures required (e.g., 3 for 3-of-5)
    pub threshold: usize,
}

impl OracleRegistry {
    pub fn new(threshold: usize) -> Self {
        Self {
            oracles: HashMap::new(),
            threshold,
        }
    }

    /// Register a new oracle
    pub fn register_oracle(&mut self, oracle_id: String, pubkey: String) {
        self.oracles.insert(oracle_id, pubkey);
    }

    /// Check if we have enough oracles
    pub fn has_quorum(&self) -> bool {
        self.oracles.len() >= self.threshold
    }

    /// Get oracle public key by ID
    pub fn get_oracle_pubkey(&self, oracle_id: &str) -> Option<&String> {
        self.oracles.get(oracle_id)
    }
}

/// Multi-oracle proof with multiple signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiOracleProof {
    pub tx_hash: String,
    pub amount: u64,
    pub confirmations: u32,
    pub vault_address: String,
    pub timestamp: u128,
    /// Map of oracle_id -> signature
    pub oracle_signatures: HashMap<String, String>,
}

impl MultiOracleProof {
    pub fn new(tx_hash: String, amount: u64, confirmations: u32, vault_address: String) -> Self {
        Self {
            tx_hash,
            amount,
            confirmations,
            vault_address,
            timestamp: crate::block::current_unix_timestamp_ms() as u128,
            oracle_signatures: HashMap::new(),
        }
    }

    /// Add an oracle signature
    pub fn add_signature(&mut self, oracle_id: String, signature: String) {
        self.oracle_signatures.insert(oracle_id, signature);
    }

    /// Check if we have enough signatures
    pub fn has_threshold(&self, threshold: usize) -> bool {
        self.oracle_signatures.len() >= threshold
    }

    /// Verify all signatures against registry
    pub fn verify_signatures(
        &self,
        registry: &OracleRegistry,
        message: &str,
    ) -> Result<bool, String> {
        if !self.has_threshold(registry.threshold) {
            return Err(format!(
                "Insufficient signatures: {} (need {})",
                self.oracle_signatures.len(),
                registry.threshold
            ));
        }

        let mut valid_count = 0;

        for (oracle_id, signature) in &self.oracle_signatures {
            // Get oracle's public key from registry
            let pubkey = registry
                .get_oracle_pubkey(oracle_id)
                .ok_or(format!("Unknown oracle: {}", oracle_id))?;

            // Verify signature using the crypto module's helper
            if crate::crypto::verify_with_pubkey_hex(message.as_bytes(), signature, pubkey) {
                valid_count += 1;
            } else {
                return Err(format!("Invalid signature from oracle: {}", oracle_id));
            }
        }

        if valid_count >= registry.threshold {
            Ok(true)
        } else {
            Err(format!(
                "Not enough valid signatures: {} (need {})",
                valid_count, registry.threshold
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_registry() {
        let mut registry = OracleRegistry::new(3);
        assert!(!registry.has_quorum());

        registry.register_oracle("oracle1".to_string(), "pubkey1".to_string());
        registry.register_oracle("oracle2".to_string(), "pubkey2".to_string());
        registry.register_oracle("oracle3".to_string(), "pubkey3".to_string());

        assert!(registry.has_quorum());
    }

    #[test]
    fn test_multi_oracle_proof() {
        let mut proof = MultiOracleProof::new(
            "tx123".to_string(),
            100000,
            10,
            "addr123".to_string(),
        );

        assert!(!proof.has_threshold(3));

        proof.add_signature("oracle1".to_string(), "sig1".to_string());
        proof.add_signature("oracle2".to_string(), "sig2".to_string());
        proof.add_signature("oracle3".to_string(), "sig3".to_string());

        assert!(proof.has_threshold(3));
    }
}
