use crate::block::{BlockHeader, BlockType};
use crate::crypto::verify_with_pubkey_hex;
use crate::storage::Storage;
use std::sync::Arc;

pub struct Chain {
    pub storage: Arc<Storage>,
    pub head_hash: Option<String>,
    pub height: u64,
}

impl Chain {
    pub fn new(storage: Arc<Storage>) -> Self {
        // Attempt to load head from storage
        // Key: "chain_info:head" -> hash
        let head_hash: Option<String> = storage.get("chain_info:head").unwrap_or(None);
        let mut height = 0;
        
        if let Some(ref h) = head_hash {
             // Load block to get index/height
             if let Ok(Some(b)) = storage.get_block(h) {
                 height = b.header.index + 1; // Height is next index
             }
        }

        Chain {
            storage,
            head_hash,
            height,
        }
    }

    pub fn head_hash(&self) -> Option<String> {
        self.head_hash.clone()
    }

    /// Internal helper to save block and update head
    fn commit_block(&mut self, header: BlockHeader) -> Result<(), &'static str> {
        let hash = header.hash.clone().ok_or("No hash")?;
        
        // Save to DB
        // For migration/simplicity, we wrap header in a Block with empty txs for now
        let full_block = crate::block::Block { header: header.clone(), transactions: vec![] };
        
        if let Err(_) = self.storage.save_block(&full_block) {
            return Err("DB Error");
        }

        // Update Head
        if let Err(_) = self.storage.put("chain_info:head", &hash) {
             return Err("Failed to update head");
        }

        self.head_hash = Some(hash);
        self.height += 1;
        Ok(())
    }

    /// Public method for P2P Sync (Trusts the block verified by peer)
    pub fn sync_block(&mut self, header: BlockHeader) -> Result<(), &'static str> {
        // In a real system, we would verify PoW/PoS/Sig here too.
        self.commit_block(header)
    }

    /// Append a reward block (verify signature)
    pub fn append_reward(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch");
            }
        }
        // If genesis (head is None), we might allow if prev_hash is correct (e.g. 0/GENESIS)

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.commit_block(header)
    }

    /// Append a PoH block (admin only)
    pub fn append_poh(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch");
            }
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }
        
        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.commit_block(header)
    }

    /// Append a proposal block (verify signature)
    pub fn append_proposal(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch");
            }
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.commit_block(header)
    }

    /// Append a vote block (verify signature)
    pub fn append_vote(
        &mut self,
        header: BlockHeader,
        voter_pubkey_hex: &str,
    ) -> Result<(), &'static str> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch");
            }
        }

        let recompute = header.calculate_hash();
        let sig_hex = header.signature_hex.as_ref().ok_or("missing signature")?;
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, voter_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash.as_deref() != Some(&recompute) {
            return Err("hash mismatch");
        }

        self.commit_block(header)
    }

    /// Tally votes for a proposal
    pub fn tally_votes(&self, proposal_id: u64) -> (u64, u64) {
        let mut yes = 0;
        let mut no = 0;
        
        // Scan blockchain
        for i in 0..self.height {
             if let Ok(Some(block)) = self.storage.get_block_by_height(i) {
                 if let BlockType::Vote { proposal_id: pid, choice, .. } = &block.header.block_type {
                     if *pid == proposal_id {
                         if *choice { yes += 1; } else { no += 1; }
                     }
                 }
             }
        }

        (yes, no)
    }

    /// Check if a proposal ID already exists
    pub fn proposal_id_exists(&self, id: u64) -> bool {
        for i in 0..self.height {
             if let Ok(Some(block)) = self.storage.get_block_by_height(i) {
                 if let BlockType::Proposal { id: pid, .. } = &block.header.block_type {
                     if *pid == id { return true; }
                 }
             }
        }
        false
    }
    
    pub fn calculate_reward_amount(&self) -> u64 {
        let base_reward = 50_000_000;
        let halvings = self.height / 10_000;
        base_reward >> halvings
    }
}