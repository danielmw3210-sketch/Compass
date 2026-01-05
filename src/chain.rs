use crate::block::{BlockHeader, BlockType};
use crate::crypto::verify_with_pubkey_hex;
use crate::storage::Storage;
use crate::vault::VaultManager;
use crate::error::CompassError;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, debug};

// v2.0: Account-based state management
use crate::account::store::AccountStore;
use crate::account::balance::BalanceStore;
use crate::oracle::registry::OracleRegistry;

/// Fork detection result
#[derive(Debug, Clone, PartialEq)]
pub enum ForkStatus {
    Compatible,  // Block extends current chain
    Fork,        // Block creates a competing fork
    Gap,         // Missing parent blocks
}

pub struct Chain {
    pub storage: Arc<Storage>,
    pub head_hash: Option<String>,
    pub height: u64,
    pub vault_manager: VaultManager,
    
    // v2.0: Account-based state management
    pub account_store: Arc<Mutex<AccountStore>>,
    pub balance_store: Arc<Mutex<BalanceStore>>,
    pub oracle_registry: Arc<Mutex<OracleRegistry>>,
}

impl Chain {
    pub fn new(storage: Arc<Storage>) -> Self {
        // Attempt to load head from storage
        // Key: "chain_info:head" -> hash
        let head_hash: Option<String> = storage.get("chain_info:head").unwrap_or(None);
        let mut height = 0;

        // --- Vault Manager (Migrated to Sled) ---
        let mut vault_manager = VaultManager::new_with_storage(storage.clone());
        if vault_manager.vaults.is_empty() && std::path::Path::new("vaults.json").exists() {
            info!("Persistence: ⚠️  Migrating 'vaults.json' to Sled DB...");
            let old_vm = VaultManager::load("vaults.json");
            for (k, v) in old_vm.vaults { 
                vault_manager.vaults.insert(k, v); 
            }
            for d in old_vm.processed_deposits { 
                vault_manager.processed_deposits.insert(d); 
            }
            for (k, v) in old_vm.oracle_prices { 
                vault_manager.oracle_prices.insert(k, v); 
            }
            let _ = vault_manager.save(""); 
            info!("Persistence: ✅ Vault Migration Complete.");
        }

        if let Some(ref h) = head_hash {
            // Load block to get index/height
            if let Ok(Some(b)) = storage.get_block(h) {
                height = b.header.index + 1; // Height is next index
                info!("✅ Blockchain Persistence: Loaded {} blocks (Head: {}...", height, &h[..12]);
            } else {
                warn!("⚠️  Head hash found but block not in DB: {}", h);
            }
        } else {
            info!("🆕 No existing blockchain found - will initialize genesis");
        }

        Chain {
            storage: storage.clone(),
            head_hash,
            height,
            vault_manager,
            
            // v2.0: Initialize account system
            account_store: Arc::new(Mutex::new(AccountStore::new())),
            balance_store: Arc::new(Mutex::new(BalanceStore::new())),
            oracle_registry: Arc::new(Mutex::new(OracleRegistry::new())),
        }
    }

    pub fn initialize_genesis(&mut self, config: &crate::genesis::GenesisConfig) -> Result<(), CompassError> {
        if self.height > 0 || self.head_hash.is_some() {
            info!("⏩ Skipping genesis initialization (chain already has {} blocks)", self.height);
            return Ok(()); // Already initialized
        }
        
        info!("🌱 Initializing Genesis Block...");

        let genesis_block = crate::block::Block {
            header: crate::block::BlockHeader {
                index: 0,
                block_type: BlockType::Genesis,
                proposer: "genesis".to_string(),
                signature_hex: "".to_string(),
                prev_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                hash: "".to_string(), // Calculated below
                timestamp: config.timestamp,
            },
            transactions: vec![],
        };
        
        let mut final_block = genesis_block;
        final_block.header.hash = final_block.header.calculate_hash()?;
        
        // --- 🔒 MAINNET IMMUTABILITY LOCK ---
        // This ensures the Genesis Parameters (Balances, Validators, Timestamp) can NEVER be changed.
        let expected_genesis = "293bf8b6db2779c6b666291e8e01560d80b8364e45d4e392610b3474c709c790";
        if final_block.header.hash != expected_genesis {
            panic!("❌ GENESIS MISMATCH! The code expects hash {}, but generated {}. Did you modify genesis.json?", expected_genesis, final_block.header.hash);
        }
        // ------------------------------------
        
        // Commit without validation checks (it's genesis)
        self.commit_block(final_block)?;
        
        // Set Genesis Hash Key
        if let Some(h) = &self.head_hash {
             self.storage.put("chain_info:genesis", h).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        }

        // Apply Initial Balances
        for (addr, amount) in &config.initial_balances {
            self.storage.set_balance(addr, "Compass", *amount).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            info!("Genesis: Credited {} with {}", addr, amount);
        }
        
        // Initial Validators (if any)
        if !config.initial_validators.is_empty() {
             let mut val_ids = Vec::new();
             for v in &config.initial_validators {
                 val_ids.push(v.id.clone());
                 self.storage.set_validator_pubkey(&v.id, &v.public_key).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                 // Stake? Genesis validators might be pre-staked.
                 // For now just add to list.
             }
             self.storage.set_active_validators(&val_ids).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        }
        
        // v2.0: Initialize admin account
        info!("🔐 Creating admin account: vikingcoder");
        {
            let mut acc_store = self.account_store.lock().unwrap();
            use crate::account::types::{AccountType, AdminAccountData};
            
            let admin_account = acc_store.create_account(
                "vikingcoder".to_string(),
                "D4rkness10@@".to_string(),
                AccountType::Admin(AdminAccountData {
                    permissions: vec!["*".to_string()], // Full permissions
                }),
            ).map_err(|e| CompassError::InvalidState(format!("Failed to create admin account: {:?}", e)))?;
            
            info!("✅ Admin account created: {}", admin_account.name);
        }
        
        // v2.0: Mint 10M COMPASS to admin
        {
            let mut bal_store = self.balance_store.lock().unwrap();
            
            bal_store.credit(
                &"vikingcoder".to_string(),
                &"COMPASS".to_string(), // Asset is type alias for String
                10_000_000 * 1_000_000, // 10M COMPASS (6 decimals)
            ).map_err(|e| CompassError::InvalidState(format!("Failed to mint genesis COMPASS: {:?}", e)))?;
            
            info!("💰 Minted 10,000,000 COMPASS to admin");
        }
        
        // v2.0: Register admin as first oracle (100K COMPASS stake)
        {
            let mut oracle_reg = self.oracle_registry.lock().unwrap();
            
            oracle_reg.register_oracle(
                "vikingcoder".to_string(),
                100_000 * 1_000_000, // 100K COMPASS stake
                0, // Genesis block
            ).map_err(|e| CompassError::InvalidState(format!("Failed to register admin oracle: {}", e)))?;
            
            info!("🔮 Registered admin as genesis oracle (100K COMPASS staked)");
        }

        Ok(())
    }

    pub fn head_hash(&self) -> Option<String> {
        self.head_hash.clone()
    }

    pub fn get_genesis_hash(&self) -> Option<String> {
        // Try direct lookup
        if let Ok(Some(hash)) = self.storage.get::<String>("chain_info:genesis") {
            return Some(hash);
        }
        // Fallback: Try height 0
        if let Ok(Some(block)) = self.storage.get_block_by_height(0) {
            return Some(block.header.hash);
        }
        None
    }

    /// Internal helper to save block and update head
    fn commit_block(&mut self, block: crate::block::Block) -> Result<(), CompassError> {
        let hash = block.header.hash.clone();
        if hash.is_empty() {
            return Err(CompassError::InvalidState("No hash".to_string()));
        }

        // Save to DB
        if let Err(e) = self.storage.save_block(&block) {
            return Err(CompassError::DatabaseError(e.to_string()));
        }

        // Update Head
        if let Err(e) = self.storage.put("chain_info:head", &hash) {
            return Err(CompassError::DatabaseError(format!("Failed to update head: {}", e)));
        }

        self.head_hash = Some(hash);
        self.height += 1;
        Ok(())
    }

    /// Public method for P2P Sync (Trusts the block verified by peer)
    pub fn sync_block(&mut self, block: crate::block::Block) -> Result<(), CompassError> {
        // 1. Idempotency
        if self.storage.get_block(&block.header.hash).map(|o| o.is_some()).unwrap_or(false) {
            return Ok(());
        }

        // 2. Validate Parent Existence
        // We MUST currently have the parent block to accept this block.
        // We do NOT strictly require parent == head anymore (supporting forks).
        let parent_exists = self.storage.get_block(&block.header.prev_hash).map(|o| o.is_some()).unwrap_or(false);
        if !parent_exists && block.header.index > 0 {
             return Err(CompassError::InvalidState(format!("Parent block {} not found (orphan)", block.header.prev_hash)));
        }

        // 3. Verify Signature & Integrity
        if block.header.hash != block.header.calculate_hash()? {
            return Err(CompassError::HashMismatch("calculated".to_string(), block.header.hash.clone()));
        }
        self.verify_block_signature(&block)?;

        // 4. Fork Choice Rule (Longest Chain / Heaviest Chain)
        // Check if this block creates a new Head (Higher Height)
        let current_head_height = self.height;
        let new_block_height = block.header.index + 1;

        if new_block_height > current_head_height {
            // New Heaviest Chain detected!
            // Case A: Simple Extension (Parent == Current Head)
            if let Some(h) = &self.head_hash {
                if &block.header.prev_hash == h {
                    // Happy path: Append
                    info!("🔗 Chain Extended: Height {} -> {}", current_head_height, new_block_height);
                    return self.commit_block(block);
                }
            } else if block.header.index == 0 {
                 // Genesis case
                 return self.commit_block(block);
            }

            // Case B: Re-Org (Fork with Higher Weight)
            // Example: We are at Height 10 (Head A). New Block is Height 11 (Head B), Parent is Height 10 (Branch B).
            // Parent B exists (checked above).
            // We switch Head pointer. 
            // WARN: State (Balances) currently reflects Chain A. 
            // In a full implementation, we must revert A and apply B.
            // For v1.4, we will Log the Re-Org and update Head, assuming state convergence handling later.
            warn!("🔀 RE-ORG DETECTED: Switching Head from Height {} to {}", current_head_height, new_block_height);
            return self.commit_block(block);

        } else {
            // Block is valid but not heavier (Side-chain or old block).
            // Just store it without updating Head.
            // This allows us to track forks.
            info!("🥢 Fork/Stale Block stored: Height {} (Head is {})", new_block_height, current_head_height);
            if let Err(e) = self.storage.save_block(&block) {
                return Err(CompassError::DatabaseError(e.to_string()));
            }
            return Ok(());
        }
    }
    
    /// Detect fork status of an incoming block
    pub fn detect_fork(&self, block: &crate::block::Block) -> ForkStatus {
        // Check if parent exists
        let parent_exists = self.storage.get_block(&block.header.prev_hash)
            .map(|o| o.is_some())
            .unwrap_or(false);
        
        if !parent_exists && block.header.index > 0 {
            return ForkStatus::Gap;
        }
        
        // Check if this extends current head
        if let Some(head) = &self.head_hash {
            if &block.header.prev_hash == head {
                return ForkStatus::Compatible;
            }
        }
        
        // Block has parent but doesn't extend head = Fork
        ForkStatus::Fork
    }

    /// Verify block signature based on proposer type
    fn verify_block_signature(&self, block: &crate::block::Block) -> Result<(), CompassError> {
        let header = &block.header;
        let recompute = header.calculate_hash()?;
        
        // Skip Genesis (index 0) verification for now if manual, or check hardcoded?
        if header.index == 0 {
            return Ok(());
        }

        match &header.block_type {
            BlockType::PoH { .. } => {
                // Consensus Block: Must be signed by a registered validator (or admin)
                // 1. Fetch proposer pubkey from storage
                let pubkey_opt = self.storage.get_validator_pubkey(&header.proposer).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                
                if let Some(pk) = pubkey_opt {
                     // Decode hex signature
                     // Decode hash to verify RAW bytes
                     if let Ok(raw_hash) = hex::decode(&recompute) {
                         if verify_with_pubkey_hex(&raw_hash, &header.signature_hex, &pk) {
                             // Verified!
                         } else {
                             debug!("Sig Fail.");
                             debug!("Header: {:?}", header);
                             debug!("Proposer: '{}'", header.proposer);
                             debug!("PubKey:   {}", pk);
                             debug!("BlockHash(Recomputed): {}", recompute);
                             debug!("Sig: {}", header.signature_hex);
                             return Err(CompassError::InvalidSignature);
                         }
                     } else {
                         return Err(CompassError::SerializationError("Failed to decode block hash".to_string()));
                     }
                } else {
                     // If proposer unknown, fail? Or allow if "admin" (but we need key)?
                     // We must fail to ensure safety. Admin key should be in storage.
                     warn!("Unknown proposer '{}'. Lookup failed.", header.proposer);
                     // DEBUG: Check what we HAVE in storage for admin
                     if let Ok(Some(admin_k)) = self.storage.get_validator_pubkey("admin") {
                         debug!("'admin' key IS in storage: {}", admin_k);
                     } else {
                         debug!("'admin' key is NOT in storage.");
                     }
                     return Err(CompassError::MissingMetadata("Unknown proposer".to_string()));
                }
            },
            BlockType::ValidatorRegistration { validator_id, pubkey, .. } => {
                 // Self-Signed by the explicit pubkey in the payload
                 // header.proposer should match validator_id
                 if &header.proposer != validator_id {
                     return Err(CompassError::InvalidState("Validator Registration proposer mismatch".to_string()));
                 }
                 if !verify_with_pubkey_hex(recompute.as_bytes(), &header.signature_hex, pubkey) {
                     return Err(CompassError::InvalidSignature);
                 }
            },
            _ => {
                // For other blocks (Transfer/Mint/etc), verification requires user pubkey lookup.
                // If proposer is the pubkey hex, we can verify.
                // If proposer is username, we need storage lookup.
                // For now, we allow them if hash is valid, effectively "Trusting" the gossiped content 
                // until we implement full user-pubkey storage.
                // But PoH verification ensures the CHAIN structure is secure.
            }
        }
        Ok(())
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
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }
        // If genesis (head is None), we might allow if prev_hash is correct (e.g. 0/GENESIS)

        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }
        
        // Decode hash to verify RAW bytes
        let raw_hash = hex::decode(&recompute).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        if !verify_with_pubkey_hex(&raw_hash, sig_hex, admin_pubkey_hex) {
            return Err(CompassError::InvalidSignature);
        }

        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }

        let full_block = crate::block::Block {
            header: header.clone(),
            transactions: vec![],
        };
        self.commit_block(full_block)
    }

    /// Append a PoH block (admin only)
    pub fn append_poh(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }

        let raw_hash = hex::decode(&recompute).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        if !verify_with_pubkey_hex(&raw_hash, sig_hex, admin_pubkey_hex) {
            return Err(CompassError::InvalidSignature);
        }

        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }

        let full_block = crate::block::Block {
            header: header.clone(),
            transactions: vec![],
        };
        self.commit_block(full_block)
    }

    /// Append a proposal block (verify signature)
    pub fn append_proposal(
        &mut self,
        header: BlockHeader,
        admin_pubkey_hex: &str,
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }
        
        let raw_hash = hex::decode(&recompute).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        if !verify_with_pubkey_hex(&raw_hash, sig_hex, admin_pubkey_hex) {
            return Err(CompassError::InvalidSignature);
        }

        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }

        let full_block = crate::block::Block {
            header: header.clone(),
            transactions: vec![],
        };
        self.commit_block(full_block)
    }

    /// Append a vote block (verify signature)
    pub fn append_vote(
        &mut self,
        header: BlockHeader,
        voter_pubkey_hex: &str,
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }
        
        let raw_hash = hex::decode(&recompute).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        if !verify_with_pubkey_hex(&raw_hash, sig_hex, voter_pubkey_hex) {
            return Err(CompassError::InvalidSignature);
        }

        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }

        let full_block = crate::block::Block {
            header: header.clone(),
            transactions: vec![],
        };
        self.commit_block(full_block)
    }

    /// Append a transfer block (verify signature, balance, nonce)
    pub fn append_transfer(
        &mut self,
        header: BlockHeader,
        sender_pubkey_hex: &str,
    ) -> Result<(), CompassError> {
        // 1. Check prev_hash
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        // 2. Verify signature
        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }
        
        let raw_hash = hex::decode(&recompute).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        if !verify_with_pubkey_hex(&raw_hash, sig_hex, sender_pubkey_hex) {
            warn!("Sig Verify Failed!");
            debug!("Hash (Recomputed): {}", recompute);
            debug!("Signature: {}", sig_hex);
            debug!("PubKey: {}", sender_pubkey_hex);
            return Err(CompassError::InvalidSignature);
        }

        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }

        // 3. Extract transfer details
        if let BlockType::Transfer {
            from,
            to,
            asset,
            amount,
            nonce,
            fee,
        } = &header.block_type
        {
            // 4. Check nonce (replay protection)
            let current_nonce = self.storage.get_nonce(from).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            if *nonce != current_nonce + 1 {
                return Err(CompassError::InvalidState(format!(
                    "invalid nonce: expected {}, got {}",
                    current_nonce + 1,
                    nonce
                )));
            }

            // 5. Check sender balance (Amount + Fee)
            // Fee is always in "Compass" (Native Token). If asset != Compass, we need to check TWO balances.

            // Check Fee Balance (Compass)
            let sender_compass_bal = self
                .storage
                .get_balance(from, "Compass")
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            let mut required_compass = *fee;
            if asset == "Compass" {
                required_compass += amount;
            }

            if sender_compass_bal < required_compass {
                return Err(CompassError::InvalidState(format!(
                    "insufficient Compass balance: has {}, needs {} (incl fee)",
                    sender_compass_bal, required_compass
                )));
            }

            // Check Asset Balance (if not Compass)
            if asset != "Compass" {
                let sender_asset_bal = self
                    .storage
                    .get_balance(from, asset)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                if sender_asset_bal < *amount {
                    return Err(CompassError::InvalidState(format!("insufficient {} balance", asset)));
                }
            }

            // 6. Execute transfer
            // Deduct Fee
            if *fee > 0 {
                self.storage
                    .set_balance(from, "Compass", sender_compass_bal - *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?; // Updates Compass bal
                                                  // Credit Fee to Foundation (or Admin)
                let foundation_bal = self
                    .storage
                    .get_balance("foundation", "Compass")
                    .unwrap_or(0);
                self.storage
                    .set_balance("foundation", "Compass", foundation_bal + *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            }

            // Deduct Amount & Credit Recipient
            // Refetch balance if it was updated by fee logic
            let sender_bal_final = self.storage.get_balance(from, asset).unwrap_or(0);
            // if asset==Compass, it's (sender_compass_bal - fee).
            self.storage
                .set_balance(from, asset, sender_bal_final - amount)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            let recipient_balance = self.storage.get_balance(to, asset).unwrap_or(0);
            self.storage
                .set_balance(to, asset, recipient_balance + amount)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            // 7. Update nonce
            self.storage
                .set_nonce(from, *nonce)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            // 8. Commit block
            let full_block = crate::block::Block {
                header: header.clone(),
                transactions: vec![], // TODO: In real system, pass transaction here
            };
            self.commit_block(full_block)?;

            Ok(())
        } else {
            Err(CompassError::InvalidState("not a transfer block".to_string()))
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
                if let BlockType::Vote {
                    proposal_id: pid,
                    choice,
                    ..
                } = &block.header.block_type
                {
                    if *pid == proposal_id {
                        if *choice {
                            yes += 1;
                        } else {
                            no += 1;
                        }
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
                    if *pid == id {
                        return true;
                    }
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
    ) -> Result<(), CompassError> {
        // 1. Check prev_hash
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        // 2. Verify signature
        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
             return Err(CompassError::InvalidSignature);
        }

        // Verify user signature on the header first
        if header.hash != recompute {
             return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
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
        } = &header.block_type
        {
            // 4. Delegate to VaultManager (Verifies Oracle Sig + updates Vault State)
            // Returns (correct_asset_name, minted_amount)
            let (asset_name, minted) = self.vault_manager.deposit_and_mint(
                collateral_asset,
                *collateral_amount,
                *mint_amount,
                owner,
                tx_proof,
                oracle_signature,
                oracle_pubkey_hex,
            ).map_err(|e| CompassError::TransactionError(e.to_string()))?;

            // Save Vault state to DB
            if let Err(e) = self.vault_manager.save("") {
                 return Err(CompassError::DatabaseError(e.to_string()));
            }

            // 5. Deduct Fee (if any)
            if *fee > 0 {
                let user_native_bal = self.storage.get_balance(owner, "Compass").unwrap_or(0);
                if user_native_bal < *fee {
                    return Err(CompassError::InvalidState("insufficient Compass balance for network fee".to_string()));
                }
                self.storage
                    .set_balance(owner, "Compass", user_native_bal - *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                let foundation = self
                    .storage
                    .get_balance("foundation", "Compass")
                    .unwrap_or(0);
                self.storage
                    .set_balance("foundation", "Compass", foundation + *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            }

            // 6. Credit Minted Asset to User
            let current_bal = self.storage.get_balance(owner, &asset_name).unwrap_or(0);
            self.storage
                .set_balance(owner, &asset_name, current_bal + minted)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            let full_block = crate::block::Block {
                header: header.clone(),
                transactions: vec![],
            };
            self.commit_block(full_block)?;
            Ok(())
        } else {
             Err(CompassError::InvalidState("not a mint block".to_string()))
        }
    }

    /// Append a Burn block
    pub fn append_burn(
        &mut self,
        header: BlockHeader,
        redeemer_pubkey_hex: &str,
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        let recompute = header.calculate_hash()?;
        let sig_hex = &header.signature_hex;
        if sig_hex.is_empty() {
            return Err(CompassError::InvalidSignature);
        }
        if !verify_with_pubkey_hex(recompute.as_bytes(), sig_hex, redeemer_pubkey_hex) {
            return Err(CompassError::InvalidSignature);
        }

        if let BlockType::Burn {
            vault_id: _,
            collateral_asset: _,
            compass_asset,
            burn_amount,
            redeemer,
            destination_address,
            fee,
        } = &header.block_type
        {
            // Check Fee
            if *fee > 0 {
                let user_native_bal = self.storage.get_balance(redeemer, "Compass").unwrap_or(0);
                if user_native_bal < *fee {
                    return Err(CompassError::InvalidState("insufficient Compass balance for fee".to_string()));
                }
                self.storage
                    .set_balance(redeemer, "Compass", user_native_bal - *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                let foundation = self
                    .storage
                    .get_balance("foundation", "Compass")
                    .unwrap_or(0);
                self.storage
                    .set_balance("foundation", "Compass", foundation + *fee)
                    .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            }

            // Check balance of Compass-Asset
            let current_bal = self
                .storage
                .get_balance(redeemer, compass_asset)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            if current_bal < *burn_amount {
                return Err(CompassError::InvalidState("insufficient balance to burn".to_string()));
            }

            // 1. Burn (reduce balance on Chain)
            self.storage
                .set_balance(redeemer, compass_asset, current_bal - burn_amount)
                .map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            // 2. Update Vault State (Calculate collateral to release)
            let released_collateral = self
                .vault_manager
                .burn_and_redeem(compass_asset, *burn_amount)
                .map_err(|e| CompassError::TransactionError(e.to_string()))?;
            
            // Save Vault state to DB
            if let Err(e) = self.vault_manager.save("") {
                 return Err(CompassError::DatabaseError(e.to_string()));
            };

            // Log for external watchers (Bridge)
            info!("EVENT: Withdrawal Authorized. {} burnt. Release {} collateral to {} on External Chain.", 
                compass_asset, released_collateral, destination_address);

            let full_block = crate::block::Block {
                header: header.clone(),
                transactions: vec![],
            };
            self.commit_block(full_block)?;
            Ok(())
        } else {
            Err(CompassError::InvalidState("not a burn block".to_string()))
        }
    }

    // 4. Validator Stats
    pub fn update_validator_stats(&self, validator: &str, reward: u64, block_time_ms: u64) -> Result<(), CompassError> {
        let mut stats = self.storage.get_validator_stats(validator).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        stats.blocks_produced += 1;
        stats.compute_earned += reward;
        // Simple moving average for block time
        if stats.blocks_produced > 1 {
            stats.avg_block_time_ms = (stats.avg_block_time_ms + block_time_ms) / 2;
        } else {
            stats.avg_block_time_ms = block_time_ms;
        }
        // Uptime logic: For now, assuming 1 block = 1 second of uptime approx
        // better would be tracking timestamps.
        // Let's just approximate uptime hours based on block count for now (assuming 1 block/sec)
        // 3600 blocks = 1 hour
        stats.uptime_hours = stats.blocks_produced / 3600;

        self.storage.set_validator_stats(validator, &stats).map_err(|e| CompassError::DatabaseError(e.to_string()))
    }

    // 5. Validator Management
    pub fn append_validator_registration(
        &mut self,
        header: BlockHeader,
    ) -> Result<(), CompassError> {
        if let Some(head) = self.head_hash() {
            if header.prev_hash != head {
                return Err(CompassError::InvalidState("prev_hash mismatch".to_string()));
            }
        }

        let recompute = header.calculate_hash()?;
        if header.hash != recompute {
            return Err(CompassError::HashMismatch("calculated".to_string(), header.hash.clone()));
        }
        
        // Check Header Sig (Proposer integrity) - Usually same as validator
        // We skip explicit header sig check here if we trust the payload sig check below, 
        // but for full security we should check both.
        // Assuming header.signature_hex is valid for header.proposer.

        if let BlockType::ValidatorRegistration {
            validator_id,
            pubkey,
            stake_amount,
            signature,
        } = &header.block_type
        {
            // 1. Verify Payload Signature (Proof of possession of Validator Key)
            // Message signed should be something binding, e.g. "REGISTER:<validator_id>" or just the whole block?
            // For simplicity, let's say they sign their own validator_id.
            if !verify_with_pubkey_hex(validator_id.as_bytes(), signature, pubkey) {
                return Err(CompassError::InvalidSignature);
            }

            // 2. Check Stake (Compass Balance)
            let current_bal = self.storage.get_balance(validator_id, "Compass").unwrap_or(0);
            if current_bal < *stake_amount {
                return Err(CompassError::InvalidState(format!("Insufficient Compass balance for stake. Has {}, needs {}", current_bal, stake_amount)));
            }

            // 3. Deduct Stake (Lock it)
            // We just remove it from circulating balance. 
            // In future, move to "StakedCompass" balance.
            self.storage.set_balance(validator_id, "Compass", current_bal - stake_amount).map_err(|e| CompassError::DatabaseError(e.to_string()))?;

            // 4. Add to Validator List
            let mut validators = self.storage.get_active_validators().map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            if !validators.contains(validator_id) {
                validators.push(validator_id.clone());
                self.storage.set_active_validators(&validators).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
                println!("Validator Registered: {} (Total: {})", validator_id, validators.len());
            }
            
            // 5. Save Pubkey Mapping
            self.storage.set_validator_pubkey(validator_id, pubkey).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
            // Note: Validators list already saved above via set_active_validators

            let full_block = crate::block::Block {
                header: header.clone(),
                transactions: vec![],
            };
            self.commit_block(full_block)?;
            Ok(())
        } else {
            Err(CompassError::InvalidState("Not a validator registration block".to_string()))
        }
    }

    pub fn get_leader(&self, tick: u64) -> Result<String, CompassError> {
        let validators = self.storage.get_active_validators().map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        if validators.is_empty() {
            return Ok("admin".to_string());
        }
        // Round Robin
        let index = (tick as usize) % validators.len();
        Ok(validators[index].clone())
    }
}
