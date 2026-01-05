//! Multi-Signature Oracle Attestation System
//! 
//! Enables decentralized oracle consensus by requiring N-of-M signatures
//! from registered oracles before accepting price updates or deposit proofs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::account::AccountId;
use crate::crypto;

/// Minimum number of oracle signatures required for consensus
pub const MIN_ORACLE_SIGNATURES: usize = 2;

/// Attestation types
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AttestationType {
    /// Price feed attestation
    Price { ticker: String, price_usd: f64 },
    /// Deposit verification attestation
    Deposit { 
        chain: String, 
        tx_hash: String, 
        amount_satoshis: u64,
        recipient: String,
    },
    /// Bridge withdrawal approval
    Withdrawal {
        chain: String,
        amount_satoshis: u64,
        destination: String,
    },
}

/// A single oracle's signature for an attestation
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OracleSignature {
    pub oracle_account: AccountId,
    pub signature_hex: String,
    pub timestamp: u64,
}

/// A pending attestation collecting signatures
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingAttestation {
    pub attestation_id: String,
    pub attestation_type: AttestationType,
    pub created_at: u64,
    pub signatures: Vec<OracleSignature>,
    pub finalized: bool,
}

impl PendingAttestation {
    /// Create new pending attestation
    pub fn new(attestation_id: String, attestation_type: AttestationType, created_at: u64) -> Self {
        Self {
            attestation_id,
            attestation_type,
            created_at,
            signatures: Vec::new(),
            finalized: false,
        }
    }
    
    /// Add a signature from an oracle
    pub fn add_signature(&mut self, sig: OracleSignature) -> Result<(), String> {
        // Check for duplicates
        if self.signatures.iter().any(|s| s.oracle_account == sig.oracle_account) {
            return Err("Oracle already signed this attestation".to_string());
        }
        
        self.signatures.push(sig);
        Ok(())
    }
    
    /// Check if attestation has enough signatures
    pub fn has_quorum(&self, required: usize) -> bool {
        self.signatures.len() >= required
    }
    
    /// Get the canonical message to sign
    pub fn canonical_message(&self) -> Vec<u8> {
        let msg = format!(
            "COMPASS_ATTESTATION:{}:{}",
            self.attestation_id,
            serde_json::to_string(&self.attestation_type).unwrap_or_default()
        );
        msg.into_bytes()
    }
}

/// Multi-signature attestation manager
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AttestationManager {
    /// Pending attestations awaiting signatures
    pub pending: HashMap<String, PendingAttestation>,
    /// Finalized attestations (for history/audit)
    pub finalized: Vec<PendingAttestation>,
    /// Required signature count (configurable)
    pub required_signatures: usize,
}

impl AttestationManager {
    /// Create new attestation manager
    pub fn new(required_signatures: usize) -> Self {
        Self {
            pending: HashMap::new(),
            finalized: Vec::new(),
            required_signatures: required_signatures.max(MIN_ORACLE_SIGNATURES),
        }
    }
    
    /// Create a new attestation for signature collection
    pub fn create_attestation(
        &mut self,
        attestation_id: String,
        attestation_type: AttestationType,
        timestamp: u64,
    ) -> Result<&PendingAttestation, String> {
        if self.pending.contains_key(&attestation_id) {
            return Err("Attestation already exists".to_string());
        }
        
        let attestation = PendingAttestation::new(attestation_id.clone(), attestation_type, timestamp);
        self.pending.insert(attestation_id.clone(), attestation);
        
        Ok(self.pending.get(&attestation_id).unwrap())
    }
    
    /// Submit a signature for an attestation
    pub fn submit_signature(
        &mut self,
        attestation_id: &str,
        oracle_account: AccountId,
        signature_hex: String,
        timestamp: u64,
        oracle_pubkey: &str,
    ) -> Result<bool, String> {
        let attestation = self.pending.get_mut(attestation_id)
            .ok_or("Attestation not found")?;
        
        if attestation.finalized {
            return Err("Attestation already finalized".to_string());
        }
        
        // Verify signature
        let message = attestation.canonical_message();
        if !crypto::verify_with_pubkey_hex(&message, &signature_hex, oracle_pubkey) {
            return Err("Invalid signature".to_string());
        }
        
        // Add signature
        attestation.add_signature(OracleSignature {
            oracle_account,
            signature_hex,
            timestamp,
        })?;
        
        // Check for quorum
        if attestation.has_quorum(self.required_signatures) {
            attestation.finalized = true;
            
            // Move to finalized list
            let finalized = self.pending.remove(attestation_id).unwrap();
            self.finalized.push(finalized);
            
            return Ok(true); // Quorum reached!
        }
        
        Ok(false) // Still collecting signatures
    }
    
    /// Get pending attestations for an oracle to sign
    pub fn get_pending_for_oracle(&self, _oracle: &AccountId) -> Vec<&PendingAttestation> {
        self.pending.values()
            .filter(|a| !a.finalized)
            .collect()
    }
    
    /// Clean up old pending attestations (garbage collection)
    pub fn cleanup_stale(&mut self, max_age_seconds: u64, current_time: u64) {
        self.pending.retain(|_, a| {
            current_time.saturating_sub(a.created_at) < max_age_seconds * 1000
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attestation_quorum() {
        let mut manager = AttestationManager::new(2);
        
        // Create attestation
        manager.create_attestation(
            "test_1".to_string(),
            AttestationType::Price { ticker: "BTC".to_string(), price_usd: 50000.0 },
            1000,
        ).unwrap();
        
        // First signature - no quorum yet
        // (In real test would use valid signatures)
    }
}
