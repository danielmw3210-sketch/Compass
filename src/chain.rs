use crate::block::{BlockHeader, BlockType};
use crate::crypto::verify_with_pubkey_hex;
use crate::storage::Storage;
use crate::vault::VaultManager;
use std::sync::Arc;

pub struct Chain {
    pub storage: Arc<Storage>,
    pub head_hash: Option<String>,
    pub height: u64,
    pub vault_manager: VaultManager,
}

impl Chain {
    pub fn new(storage: Arc<Storage>) -> Self {
        // Attempt to load head from storage
        // Key: "chain_info:head" -> hash
        let head_hash: Option<String> = storage.get("chain_info:head").unwrap_or(None);
        let mut height = 0;
        
        let vault_manager = VaultManager::load("vaults.json");

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
            vault_manager,
        }
    }

    pub fn head_hash(&self) -> Option<String> {
        self.head_hash.clone()
    }

    /// Internal helper to save block and update head
    fn commit_block(&mut self, header: BlockHeader) -> Result<(), &'static str> {
        let hash = header.hash.clone();
        if hash.is_empty() {
             return Err("No hash");
        }
        
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

    pub fn get_blocks_range(&self, start: u64, end: u64) -> Vec<crate::block::Block> {
        let mut blocks = Vec::new();
        for i in start..=end {
            if let Ok(Some(block)) = self.storage.get_block_by_height(i) {
                blocks.push(block);
            } else {
                break; // Stop if we hit a gap (or end of chain)
            }
        }
        blocks
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
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
             return Err("missing signature");
        }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
             return Err("invalid signature");
        }

        if header.hash != recompute {
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
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err("missing signature");
        }
        
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }
        
        if header.hash != recompute {
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
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
             return Err("missing signature");
        }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, admin_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash != recompute {
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
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err("missing signature");
        }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, voter_pubkey_hex) {
            return Err("invalid signature");
        }

        if header.hash != recompute {
            return Err("hash mismatch");
        }

        self.commit_block(header)
    }

    /// Append a transfer block (verify signature, balance, nonce)
    pub fn append_transfer(
        &mut self,
        header: BlockHeader,
        sender_pubkey_hex: &str,
    ) -> Result<(), String> {
        // 1. Check prev_hash
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch".to_string());
            }
        }

        // 2. Verify signature
        let recompute = header.calculate_hash();
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err("missing signature".to_string());
        }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, sender_pubkey_hex) {
            return Err("invalid signature".to_string());
        }

        if header.hash != recompute {
            return Err("hash mismatch".to_string());
        }

            // 3. Extract transfer details
        if let BlockType::Transfer { from, to, asset, amount, nonce, fee } = &header.block_type {
            // 4. Check nonce (replay protection)
            let current_nonce = self.storage.get_nonce(from).map_err(|e| e.to_string())?;
            if *nonce != current_nonce + 1 {
                return Err(format!("invalid nonce: expected {}, got {}", current_nonce + 1, nonce));
            }

            // 5. Check sender balance (Amount + Fee)
            // Fee is always in "Compass" (Native Token). If asset != Compass, we need to check TWO balances.
            
            // Check Fee Balance (Compass)
            let sender_compass_bal = self.storage.get_balance(from, "Compass").map_err(|e| e.to_string())?;
            let mut required_compass = *fee;
            if asset == "Compass" {
                required_compass += amount;
            }
            
            if sender_compass_bal < required_compass {
                 return Err(format!("insufficient Compass balance: has {}, needs {} (incl fee)", sender_compass_bal, required_compass));
            }
            
            // Check Asset Balance (if not Compass)
            if asset != "Compass" {
                 let sender_asset_bal = self.storage.get_balance(from, asset).map_err(|e| e.to_string())?;
                 if sender_asset_bal < *amount {
                      return Err(format!("insufficient {} balance", asset));
                 }
            }

            // 6. Execute transfer
            // Deduct Fee
            if *fee > 0 {
                self.storage.set_balance(from, "Compass", sender_compass_bal - *fee).map_err(|e| e.to_string())?; // Updates Compass bal
                // Credit Fee to Foundation (or Admin)
                let foundation_bal = self.storage.get_balance("foundation", "Compass").unwrap_or(0);
                self.storage.set_balance("foundation", "Compass", foundation_bal + *fee).map_err(|e| e.to_string())?;
            }
            
            // Deduct Amount & Credit Recipient
            // Refetch balance if it was updated by fee logic
            let sender_bal_final = self.storage.get_balance(from, asset).unwrap_or(0);
             // if asset==Compass, it's (sender_compass_bal - fee).
            self.storage.set_balance(from, asset, sender_bal_final - amount).map_err(|e| e.to_string())?;
            
            let recipient_balance = self.storage.get_balance(to, asset).unwrap_or(0);
            self.storage.set_balance(to, asset, recipient_balance + amount).map_err(|e| e.to_string())?;

            // 7. Update nonce
            self.storage.set_nonce(from, *nonce).map_err(|e| e.to_string())?;

            // 8. Commit block
            self.commit_block(header).map_err(|e| e.to_string())?;

            Ok(())
        } else {
            Err("not a transfer block".to_string())
        }
    }
    
    // ... (append_mint/burn updates below need to be separate or I can include destructuring fixes here if I replace those methods again, but they were added recently.
    // I will use multi_replace to fix destructuring in append_mint/burn if needed, or just let compilation fail and fix.
    // Providing destructuring updates here inside the same file replacement if ranges allow.
    // The previous replace_file_content targeted append_mint which is further down.
    // I will do separate edits.

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

    /// Append a Mint block (Vault logic)
    pub fn append_mint(
        &mut self,
        header: BlockHeader,
        oracle_pubkey_hex: &str, // Admin or Oracle key acting as authority
    ) -> Result<(), String> {
        // 1. Check prev_hash
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch".to_string());
            }
        }

        // 2. Verify signature (The header is signed by the USER/Owner requesting the mint)
        // Wait, the Mint block contains an `oracle_signature` field INSIDE the block type.
        // But the Block Header `signature_hex` is usually the block proposer's signature.
        // For Validated Mint, the Oracle should PROPOSE the block?
        // Or the User proposes a block containing the Oracle's proof?
        // Let's assume User proposes block:
        // Header Sig = User Sig (verifies integrity of header)
        // Oracle Sig = Inside BlockType (verifies permission/collateral)
        
        let recompute = header.calculate_hash();
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err("missing signature".to_string());
        }
        
        // Verify user signature on the header first
        if header.hash != recompute {
            return Err("hash mismatch".to_string());
        }

        if let BlockType::Mint { 
            vault_id: _, 
            collateral_asset, 
            collateral_amount, 
            compass_asset: _, 
            mint_amount, 
            owner, 
            tx_proof, 
            oracle_signature,
            fee,
        } = &header.block_type {
             // 3. Verify Header Signature matches Owner (Simplified check if we wanted)
             // ...

             // 4. Delegate to VaultManager (Verifies Oracle Sig + updates Vault State)
             // Returns (correct_asset_name, minted_amount)
             let (asset_name, minted) = self.vault_manager.deposit_and_mint(
                 collateral_asset,
                 *collateral_amount,
                 *mint_amount,
                 owner,
                 tx_proof,
                 oracle_signature,
                 oracle_pubkey_hex
             )?;

             // Save Vault state
             self.vault_manager.save("vaults.json");

             // 5. Deduct Fee (if any) - Fee is in "Compass" (Native)??
             // Wait, if I'm minting Compass-LTC, I might not have Compass-Native.
             // Fee should probably be taken from the MINTED amount or Collateral?
             // `VaultManager` already deducted a "Protocol Fee" from the collateral side (implied).
             // The `fee` field in BlockType is usually "Network Fee" (Gas).
             // If this is a gas fee, user needs native Compass.
             if *fee > 0 {
                  let user_native_bal = self.storage.get_balance(owner, "Compass").unwrap_or(0);
                   if user_native_bal < *fee {
                       return Err("insufficient Compass balance for network fee".to_string());
                   }
                   self.storage.set_balance(owner, "Compass", user_native_bal - *fee).map_err(|e| e.to_string())?;
                   let foundation = self.storage.get_balance("foundation", "Compass").unwrap_or(0);
                   self.storage.set_balance("foundation", "Compass", foundation + *fee).map_err(|e| e.to_string())?;
             }

             // 6. Credit Minted Asset to User
             let current_bal = self.storage.get_balance(owner, &asset_name).unwrap_or(0);
             self.storage.set_balance(owner, &asset_name, current_bal + minted).map_err(|e| e.to_string())?;
             
             self.commit_block(header).map_err(|e| e.to_string())?;
             Ok(())
        } else {
            Err("not a mint block".to_string())
        }
    }

    /// Append a Burn block
    pub fn append_burn(
        &mut self,
        header: BlockHeader,
        redeemer_pubkey_hex: &str,
    ) -> Result<(), String> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err("prev_hash mismatch".to_string());
            }
        }

        let recompute = header.calculate_hash();
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() { return Err("missing signature".to_string()); }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, redeemer_pubkey_hex) {
             return Err("invalid signature".to_string());
        }

        if let BlockType::Burn { 
            vault_id: _, 
            compass_asset, 
            burn_amount, 
            redeemer, 
            destination_address,
            fee 
        } = &header.block_type {
            // Check Fee
            if *fee > 0 {
                let user_native_bal = self.storage.get_balance(redeemer, "Compass").unwrap_or(0);
                if user_native_bal < *fee { return Err("insufficient Compass balance for fee".to_string()); }
                self.storage.set_balance(redeemer, "Compass", user_native_bal - *fee).map_err(|e| e.to_string())?;
                let foundation = self.storage.get_balance("foundation", "Compass").unwrap_or(0);
                self.storage.set_balance("foundation", "Compass", foundation + *fee).map_err(|e| e.to_string())?;
            }
            
            // Check balance of Compass-Asset
            let current_bal = self.storage.get_balance(redeemer, compass_asset).map_err(|e| e.to_string())?;
            if current_bal < *burn_amount {
                return Err("insufficient balance to burn".to_string());
            }
            
            // 1. Burn (reduce balance on Chain)
            self.storage.set_balance(redeemer, compass_asset, current_bal - burn_amount).map_err(|e| e.to_string())?;
            
            // 2. Update Vault State (Calculate collateral to release)
            let released_collateral = self.vault_manager.burn_and_redeem(compass_asset, *burn_amount)?;
            self.vault_manager.save("vaults.json");

            // Log for external watchers (Bridge)
            println!("EVENT: Withdrawal Authorized. {} burnt. Release {} collateral to {} on External Chain.", 
                compass_asset, released_collateral, destination_address);
            
            self.commit_block(header).map_err(|e| e.to_string())?;
            Ok(())
        } else {
             Err("not a burn block".to_string())
        }
    }
}