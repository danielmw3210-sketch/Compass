use serde::{Serialize, Deserialize};
use std::fs;
use crate::block::{BlockHeader, BlockType};
use crate::crypto::verify_with_pubkey_hex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chain {
    pub blocks: Vec<BlockHeader>,
}

impl Chain {
    pub fn new() -> Self {
        Chain { blocks: Vec::new() }
    }

    pub fn head_hash(&self) -> Option<String> {
        self.blocks.last().and_then(|b| b.hash.clone())
    }

    pub fn save_to_json(&self, path: &str) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json)
    }

    pub fn load_from_json(path: &str) -> Self {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(chain) = serde_json::from_str(&contents) {
                return chain;
            }
        }
        Chain::new()
    }

    /// Append a reward block (verify signature)
    pub fn append_reward(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if header.prev_hash != self.head_hash() {
            return Err("prev_hash mismatch");
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.blocks.push(header);
        Ok(())
    }

    /// Append a proposal block (verify signature)
    pub fn append_proposal(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if header.prev_hash != self.head_hash() {
            return Err("prev_hash mismatch");
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.blocks.push(header);
        Ok(())
    }

    /// Append a vote block (verify signature)
    pub fn append_vote(
        &mut self,
        header: BlockHeader,
        voter_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if header.prev_hash != self.head_hash() {
            return Err("prev_hash mismatch");
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, voter_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.blocks.push(header);
        Ok(())
    }

    /// Tally votes for a proposal
    pub fn tally_votes(&self, proposal_id: u64) -> (u64, u64) {
        let mut yes = 0;
        let mut no = 0;
        for b in &self.blocks {
            if let BlockType::Vote { proposal_id: pid, choice, .. } = &b.block_type {
                if *pid == proposal_id {
                    if *choice { yes += 1; } else { no += 1; }
                }
            }
        }
        (yes, no)
    }

    /// Check if a proposal ID already exists
    pub fn proposal_id_exists(&self, id: u64) -> bool {
        self.blocks.iter().any(|b| {
            if let BlockType::Proposal { id: pid, .. } = &b.block_type {
                *pid == id
            } else {
                false
            }
        })
    }
}