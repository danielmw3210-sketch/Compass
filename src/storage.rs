use sled::Db;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::error::CompassError;

#[derive(Clone)]
pub struct Storage {
    db: Db,
}

impl Storage {
    pub fn new(path: &str) -> Result<Self, CompassError> {
        let db = sled::open(path).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        Ok(Storage { db })
    }

    // Generic Helper: Put
    pub fn put<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<(), CompassError> {
        let serialized = bincode::serialize(value).map_err(|e| CompassError::SerializationError(e.to_string()))?;
        self.db
            .insert(key.as_bytes(), serialized)
            .map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    // Generic Helper: Get
    pub fn get<T: for<'a> Deserialize<'a>>(&self, key: &str) -> Result<Option<T>, CompassError> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => {
                let deserialized = bincode::deserialize(&data).map_err(|e| CompassError::SerializationError(e.to_string()))?;
                Ok(Some(deserialized))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CompassError::DatabaseError(e.to_string())),
        }
    }

    // Helper: Delete
    pub fn delete(&self, key: &str) -> Result<(), CompassError> {
        self.db.remove(key.as_bytes()).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    // --- Specific Accessors ---

    pub fn get_balance(&self, wallet_id: &str, asset: &str) -> Result<u64, CompassError> {
        let key = format!("bal:{}:{}", wallet_id, asset);
        match self.db.get(key.as_bytes()) {
            Ok(Some(val)) => {
                let bytes: [u8; 8] = val.as_ref().try_into().map_err(|_| CompassError::SerializationError("Invalid balance bytes".to_string()))?;
                Ok(u64::from_be_bytes(bytes))
            }
            Ok(None) => Ok(0),
            Err(e) => Err(CompassError::DatabaseError(e.to_string())),
        }
    }

    pub fn set_balance(&self, wallet_id: &str, asset: &str, amount: u64) -> Result<(), CompassError> {
        let key = format!("bal:{}:{}", wallet_id, asset);
        let bytes = amount.to_be_bytes();
        self.db.insert(key.as_bytes(), &bytes).map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn update_balance(&self, wallet_id: &str, asset: &str, amount: u64) -> Result<(), CompassError> {
        let current = self.get_balance(wallet_id, asset)?;
        let new_bal = current.saturating_add(amount); 
        self.set_balance(wallet_id, asset, new_bal)
    }

    // --- Wallets (Phase 2 Migration) ---
    pub fn save_wallet(&self, wallet: &crate::wallet::Wallet) -> Result<(), CompassError> {
        self.put(&format!("wallet:{}", wallet.owner), wallet)
    }

    pub fn get_wallet(&self, owner: &str) -> Result<Option<crate::wallet::Wallet>, CompassError> {
        self.get(&format!("wallet:{}", owner))
    }

    pub fn get_all_wallets(&self) -> Vec<crate::wallet::Wallet> {
        self.get_by_prefix("wallet:")
    }

    // --- Vaults (Phase 2) ---
    pub fn save_vault(&self, id: &str, vault: &crate::vault::Vault) -> Result<(), CompassError> {
        self.put(&format!("vault:{}", id), vault)
    }
    
    pub fn get_all_vaults(&self) -> Vec<crate::vault::Vault> {
        self.get_by_prefix("vault:")
    }

    pub fn mark_deposit_processed(&self, tx_hash: &str) -> Result<(), CompassError> {
        self.put(&format!("deposit:{}", tx_hash), &true)
    }

    pub fn is_deposit_processed(&self, tx_hash: &str) -> bool {
        match self.get::<bool>(&format!("deposit:{}", tx_hash)) {
            Ok(Some(true)) => true,
            _ => false,
        }
    }

    pub fn save_oracle_price_info(&self, ticker: &str, info: &(rust_decimal::Decimal, u64)) -> Result<(), CompassError> {
        self.put(&format!("price:{}", ticker), info)
    }
    
    // Note: To return HashMap of prices, we might need a specific method since get_by_prefix returns Vec<T>
    // But we lose the key (Ticker).
    // Just helper function:
    pub fn get_all_prices(&self) -> Vec<(String, (rust_decimal::Decimal, u64))> {
        let mut out = Vec::new();
        for item in self.db.scan_prefix("price:") {
            if let Ok((key_bytes, val_bytes)) = item {
                if let Ok(p) = bincode::deserialize::<(rust_decimal::Decimal, u64)>(&val_bytes) {
                     if let Ok(k_str) = std::str::from_utf8(&key_bytes) {
                        // Key is "price:BTC", need to strip prefix
                        if let Some(ticker) = k_str.strip_prefix("price:") {
                             out.push((ticker.to_string(), p));
                        }
                     }
                }
            }
        }
        out
    }

    // --- Betting (Phase 2) ---
    pub fn save_active_bet(&self, bet: &crate::layer3::betting::PredictionBet) -> Result<(), CompassError> {
        self.put(&format!("bet:active:{}", bet.timestamp), bet)
    }

    pub fn delete_active_bet(&self, timestamp: u64) -> Result<(), CompassError> {
        self.delete(&format!("bet:active:{}", timestamp))
    }

    pub fn save_settled_bet(&self, bet: &crate::layer3::betting::PredictionBet) -> Result<(), CompassError> {
        self.put(&format!("bet:settled:{}", bet.timestamp), bet)
    }

    pub fn get_active_bets(&self) -> Vec<crate::layer3::betting::PredictionBet> {
        self.get_by_prefix("bet:active:")
    }

    pub fn get_settled_bets(&self) -> Vec<crate::layer3::betting::PredictionBet> {
        self.get_by_prefix("bet:settled:")
    }

    pub fn save_betting_stats(&self, staked: u64, won: u64, lost: u64) -> Result<(), CompassError> {
        self.put("bet_stats", &(staked, won, lost))
    }

    pub fn get_betting_stats(&self) -> Result<Option<(u64, u64, u64)>, CompassError> {
        self.get("bet_stats")
    }

    // --- Market (Phase 2) ---
    pub fn save_order_book(&self, pair: &str, book: &crate::market::OrderBook) -> Result<(), CompassError> {
        self.put(&format!("market:book:{}", pair), book)
    }

    pub fn get_all_order_books(&self) -> Vec<crate::market::OrderBook> {
        self.get_by_prefix("market:book:")
    }

    pub fn save_market_meta(&self, next_id: u64) -> Result<(), CompassError> {
        self.put("market:meta", &next_id)
    }

    pub fn get_market_meta(&self) -> Result<Option<u64>, CompassError> {
        self.get("market:meta")
    }

    // NFT Listings
    pub fn save_nft_listing(&self, listing: &crate::market::NFTListing) -> Result<(), CompassError> {
        self.put(&format!("market:listing:{}", listing.token_id), listing)
    }

    pub fn get_all_nft_listings(&self) -> Vec<crate::market::NFTListing> {
        self.get_by_prefix("market:listing:")
    }

    pub fn delete_nft_listing(&self, token_id: &str) -> Result<(), CompassError> {
        self.delete(&format!("market:listing:{}", token_id))
    }

    // Persisted NFTs (Verification)
    pub fn get_all_nfts(&self) -> Vec<crate::layer3::model_nft::ModelNFT> {
        self.get_by_prefix("nft:")
    }

    // Compute Jobs
    pub fn save_compute_job(&self, job: &crate::layer3::compute::ComputeJob) -> Result<(), CompassError> {
        self.put(&format!("compute_job:{}", job.job_id), job)
    }

    pub fn get_compute_job(&self, job_id: &str) -> Result<Option<crate::layer3::compute::ComputeJob>, CompassError> {
        self.get(&format!("compute_job:{}", job_id))
    }

    pub fn get_all_compute_jobs(&self) -> Vec<crate::layer3::compute::ComputeJob> {
        self.get_by_prefix("compute_job:")
    }

    pub fn get_pending_compute_jobs(&self) -> Vec<crate::layer3::compute::ComputeJob> {
        use crate::layer3::compute::ComputeJobStatus;
        self.get_all_compute_jobs()
            .into_iter()
            .filter(|job| job.status == ComputeJobStatus::Pending)
            .collect()
    }

    pub fn delete_compute_job(&self, job_id: &str) -> Result<(), CompassError> {
        self.delete(&format!("compute_job:{}", job_id))
    }

    // 1. Blocks
    pub fn save_block(&self, block: &crate::block::Block) -> Result<(), CompassError> {
        let hash = &block.header.hash;
        if hash.is_empty() { return Err(CompassError::InvalidState("Block has no hash".to_string())); }
        
        self.put(&format!("block:{}", hash), block)?;
        self.put(&format!("height:{}", block.header.index), hash)?;
        Ok(())
    }

    pub fn get_block(&self, hash: &str) -> Result<Option<crate::block::Block>, CompassError> {
        self.get(&format!("block:{}", hash))
    }

    pub fn get_block_by_height(&self, height: u64) -> Result<Option<crate::block::Block>, CompassError> {
        if let Ok(Some(hash)) = self.get::<String>(&format!("height:{}", height)) {
            self.get_block(&hash)
        } else {
            Ok(None)
        }
    }

    // 2. Nonces
    pub fn get_nonce(&self, wallet_id: &str) -> Result<u64, CompassError> {
        let key = format!("nonce:{}", wallet_id);
        Ok(self.get::<u64>(&key)?.unwrap_or(0))
    }

    pub fn set_nonce(&self, wallet_id: &str, nonce: u64) -> Result<(), CompassError> {
        let key = format!("nonce:{}", wallet_id);
        self.put(&key, &nonce)
    }

    // 3. Validators
    pub fn get_validator_stats(&self, validator: &str) -> Result<crate::rpc::types::ValidatorStats, CompassError> {
         let key = format!("stats:{}", validator);
         Ok(self.get(&key)?.unwrap_or_default())
    }

    pub fn set_validator_stats(&self, validator: &str, stats: &crate::rpc::types::ValidatorStats) -> Result<(), CompassError> {
        let key = format!("stats:{}", validator);
        self.put(&key, stats)
    }

    pub fn get_active_validators(&self) -> Result<Vec<String>, CompassError> {
        self.get::<Vec<String>>("chain_info:validators")
            .map(|v| v.unwrap_or_else(|| vec!["admin".to_string()]))
    }

    pub fn set_active_validators(&self, validators: &[String]) -> Result<(), CompassError> {
        self.put("chain_info:validators", validators)
    }

    pub fn get_validator_pubkey(&self, validator_id: &str) -> Result<Option<String>, CompassError> {
        self.get::<String>(&format!("val_pubkey:{}", validator_id))
    }

    pub fn set_validator_pubkey(&self, validator_id: &str, pubkey: &str) -> Result<(), CompassError> {
        self.put(&format!("val_pubkey:{}", validator_id), &pubkey.to_string())
    }

    // 4. Prefix Scan
    pub fn get_by_prefix<T: for<'a> Deserialize<'a>>(&self, prefix: &str) -> Vec<T> {
        let mut items = Vec::new();
        for item in self.db.scan_prefix(prefix) {
             if let Ok((_key, value)) = item {
                 if let Ok(deserialized) = bincode::deserialize::<T>(&value) {
                     items.push(deserialized);
                 }
             }
        }
        items
    }

    // 5. Oracle
    pub fn save_oracle_job(&self, job: &crate::rpc::types::OracleVerificationJob) -> Result<(), CompassError> {
        self.put(&format!("oracle_job:{}", job.job_id), job)
    }

    pub fn get_oracle_job(&self, job_id: &str) -> Result<Option<crate::rpc::types::OracleVerificationJob>, CompassError> {
        self.get(&format!("oracle_job:{}", job_id))
    }

    pub fn get_pending_oracle_jobs(&self) -> Vec<crate::rpc::types::OracleVerificationJob> {
        self.get_by_prefix("oracle_job:")
    }

    pub fn delete_oracle_job(&self, job_id: &str) -> Result<(), CompassError> {
        self.delete(&format!("oracle_job:{}", job_id))
    }

    // Recurring
    pub fn save_recurring_job(&self, job: &crate::rpc::types::RecurringOracleJob) -> Result<(), CompassError> {
        self.put(&format!("recurring_job:{}", job.job_id), job)
    }
    
    pub fn get_recurring_job(&self, job_id: &str) -> Result<Option<crate::rpc::types::RecurringOracleJob>, CompassError> {
         self.get(&format!("recurring_job:{}", job_id))
    }

    pub fn get_all_recurring_jobs(&self) -> Vec<crate::rpc::types::RecurringOracleJob> {
        self.get_by_prefix("recurring_job:")
    }

    pub fn delete_recurring_job(&self, job_id: &str) -> Result<(), CompassError> {
        self.delete(&format!("recurring_job:{}", job_id))
    }


    // Flush
    pub fn flush(&self) -> Result<(), CompassError> {
        self.db.flush().map_err(|e| CompassError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}
