use ed25519_dalek::{Signature, Verifier};
use hex;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;

pub mod keys;
pub use keys::VaultKeyManager;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vault {
    pub collateral_asset: String, // e.g., "SOL"
    pub compass_asset: String,    // e.g., "Compass-SOL"
    pub vault_address: String,    // External Chain Address (e.g. Solana Addr)
    pub exchange_rate: Decimal,   // Amount of Compass per 1 unit of Collateral. Default 1.0
    pub collateral_balance: u64,  // Total External Asset Locked
    pub minted_supply: u64,       // Total Compass-Asset Minted
    pub accumulated_fees: u64,    // Fees collected
    pub mint_fee_rate: Decimal,       // e.g. 0.0025
    pub redeem_fee_rate: Decimal,     // e.g. 0.0050
    #[serde(default)]
    pub derivation_path: String,  // HD wallet derivation path
}

#[derive(Debug, PartialEq)]
pub enum VaultHealth {
    Safe,
    AtRisk, // < 150%
    Liquidatable, // < 110%
}

impl Vault {
    pub fn check_health(&self, current_price_per_unit: Decimal) -> VaultHealth {
        // current_price_per_unit: e.g. $100 per SOL
        // Collateral Value = balance * price
        // Minted Value = minted_supply * 1.0 (assuming Peg = $1)
        
        // Wait, price is usually "Collateral per Compass" or "Compass per Collateral"?
        // exchange_rate is "Compass per Collateral".
        // Let's use exchange_rate (Oracle Price) directly.
        
        let col_val = Decimal::from(self.collateral_balance) * current_price_per_unit;
        let debt_val = Decimal::from(self.minted_supply);
        
        if debt_val.is_zero() {
            return VaultHealth::Safe;
        }

        let ratio = col_val / debt_val;
        
        if ratio < Decimal::from_str("1.10").unwrap() {
            VaultHealth::Liquidatable
        } else if ratio < Decimal::from_str("1.50").unwrap() {
            VaultHealth::AtRisk
        } else {
            VaultHealth::Safe
        }
    }
}

#[derive(Serialize, Deserialize, Clone)] // Removed generic Debug
pub struct VaultManager {
    // Keyed by compass_asset name (e.g., "Compass-SOL")
    pub vaults: HashMap<String, Vault>,
    #[serde(default)]
    pub processed_deposits: HashSet<String>,
    #[serde(default)]
    pub oracle_prices: HashMap<String, (Decimal, u64)>, // Ticker -> (Price, Timestamp)
    
    #[serde(skip)]
    pub storage: Option<std::sync::Arc<crate::storage::Storage>>,
}

impl std::fmt::Debug for VaultManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultManager")
         .field("vaults", &self.vaults)
         .field("processed_deposits", &self.processed_deposits)
         .field("oracle_prices", &self.oracle_prices)
         .finish()
    }
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
            processed_deposits: HashSet::new(),
            oracle_prices: HashMap::new(),
            storage: None,
        }
    }

    pub fn new_with_storage(storage: std::sync::Arc<crate::storage::Storage>) -> Self {
        let mut vm = VaultManager {
            vaults: HashMap::new(),
            processed_deposits: HashSet::new(),
            oracle_prices: HashMap::new(),
            storage: Some(storage.clone()),
        };
        
        // Load Vaults from DB
        for v in storage.get_all_vaults() {
            vm.vaults.insert(v.compass_asset.clone(), v);
        }
        
        // Load Oracle Prices from DB
        for (ticker, info) in storage.get_all_prices() {
            vm.oracle_prices.insert(ticker, info);
        }
        
        // Note: We don't load ALL processed deposits into RAM if the set is huge. 
        // We might rely on DB checks. But for consistency with JSON logic currently,
        // we leave the HashSet empty or partial? 
        // For simpler migration: We only load what's in JSON during migration. 
        // New deposits go to DB. Checks should verify DB.
        
        vm
    }

    /// Load from file or create new
    pub fn load(path: &str) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_else(|_| Self::new())
        } else {
            Self::new()
        }
    }

    /// Save to file
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(s) = &self.storage {
             // Sled Persist
             for v in self.vaults.values() {
                 let _ = s.save_vault(&v.compass_asset, v);
             }
             for (t, info) in &self.oracle_prices {
                 let _ = s.save_oracle_price_info(t, info);
             }
             // Deposits marked individually usually, but loop here if bulk save?
             for d in &self.processed_deposits {
                 let _ = s.mark_deposit_processed(d);
             }
             let _ = s.flush();
             Ok(())
        } else {
             let data = serde_json::to_string_pretty(self)?;
             fs::write(path, data)
        }
    }

    /// Register a new vault type (e.g. "Compass-SOL")
    pub fn register_vault(
        &mut self,
        collateral: &str,
        compass_asset: &str,
        address: &str,
        rate: u64,
    ) -> Result<(), String> {
        if self.vaults.contains_key(compass_asset) {
            return Err("Vault already exists for this asset".to_string());
        }
        let vault = Vault {
            collateral_asset: collateral.to_string(),
            compass_asset: compass_asset.to_string(),
            vault_address: address.to_string(),
            exchange_rate: Decimal::from(rate),
            collateral_balance: 0,
            minted_supply: 0,
            accumulated_fees: 0,
            mint_fee_rate: Decimal::from_str("0.0025").unwrap(),   // 0.25% Default
            redeem_fee_rate: Decimal::from_str("0.0050").unwrap(), // 0.50% Default
            derivation_path: "".to_string(), // Default empty for now
        };
        self.vaults.insert(compass_asset.to_string(), vault.clone());
        
        if let Some(s) = &self.storage {
            let _ = s.save_vault(&vault.compass_asset, &vault);
        }
        
        Ok(())
    }

    /// Generate deterministic vault address (simplified version)
    /// Returns (address_identifier, derivation_path)
    /// Note: For production, integrate with actual blockchain address generation
    pub fn generate_vault_address(
        owner: &str,
        collateral_asset: &str,
        master_seed: &[u8; 64],
    ) -> (String, String) {
        use sha2::{Sha256, Digest};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Create deterministic indices
        let mut hasher = DefaultHasher::new();
        owner.to_string().hash(&mut hasher);
        let owner_hash = (hasher.finish() % 2_147_483_648) as u32;
        
        let mut hasher = DefaultHasher::new();
        collateral_asset.to_string().hash(&mut hasher);
        let asset_hash = (hasher.finish() % 2_147_483_648) as u32;
        
        // Create derivation path
        let path_str = format!("m/44'/0'/{}'/{}'", owner_hash, asset_hash);
        
        // Generate deterministic address from seed + path
        let mut hasher = Sha256::new();
        hasher.update(master_seed);
        hasher.update(owner.as_bytes());
        hasher.update(collateral_asset.as_bytes());
        let hash = hasher.finalize();
        
        // Create address identifier (for now, hex representation)
        // In production, this would be converted to proper blockchain address format
        let address = match collateral_asset {
            "BTC" => format!("bc1q{}", hex::encode(&hash[..20])),
            "LTC" => format!("ltc1q{}", hex::encode(&hash[..20])),
            "SOL" => format!("sol1{}", hex::encode(&hash[..20])),
            _ => format!("addr1{}", hex::encode(&hash[..20])),
        };
        
        (address, path_str)
    }

    /// Get info for dual-nature display
    pub fn get_asset_info(&self, compass_asset: &str) -> Option<(u64, u64, String)> {
        self.vaults.get(compass_asset).map(|v| {
            (
                v.collateral_balance,
                v.minted_supply,
                v.collateral_asset.clone(),
            )
        })
    }

    /// Claim a deposit and Mint User-Defined Amount
    /// Message signed: "DEPOSIT:<COLLATERAL>:<AMOUNT>:<TX_HASH>:<MINT_AMOUNT>:<OWNER>"
    pub fn deposit_and_mint(
        &mut self,
        collateral_ticker: &str, // e.g. "LTC"
        collateral_amount: u64,
        requested_mint_amount: u64,
        owner_id: &str, // e.g. "Daniel"
        tx_hash: &str,
        oracle_sig_hex: &str,
        oracle_pubkey_hex: &str,
    ) -> Result<(String, u64), String> {
        // Returns (Asset Name, Minted Amount)
        // 1. Verify Oracle Signature
        let msg = format!(
            "DEPOSIT:{}:{}:{}:{}:{}",
            collateral_ticker, collateral_amount, tx_hash, requested_mint_amount, owner_id
        );
        
        // Use standardized verification from crypto module
        if !crate::crypto::verify_with_pubkey_hex(msg.as_bytes(), oracle_sig_hex, oracle_pubkey_hex) {
            return Err("Invalid Oracle Signature! Deposit not verified.".to_string());
        }

        // 1.5 Replay Protection
        // Check RAM
        if self.processed_deposits.contains(tx_hash) {
            return Err("Deposit Transaction already processed!".to_string());
        }
        // Check DB
        if let Some(s) = &self.storage {
             if s.is_deposit_processed(tx_hash) {
                 return Err("Deposit Transaction already processed (DB)!".to_string());
             }
        }
        
        self.processed_deposits.insert(tx_hash.to_string());
        if let Some(s) = &self.storage {
             let _ = s.mark_deposit_processed(tx_hash);
        }

        // 2. Determine Asset Name (Traceable Owner)
        // e.g. "Compass:Daniel:LTC"
        let asset_name = format!("Compass:{}:{}", owner_id, collateral_ticker);

        // 3. Get or Create Personal Vault
        let vault = self.vaults.entry(asset_name.clone()).or_insert_with(|| {
            Vault {
                collateral_asset: collateral_ticker.to_string(),
                compass_asset: asset_name.clone(),
                vault_address: "SharedOracleVault".to_string(),
                exchange_rate: Decimal::ZERO,
                collateral_balance: 0,
                minted_supply: 0,
                accumulated_fees: 0,
                mint_fee_rate: Decimal::from_str("0.0025").unwrap(),
                redeem_fee_rate: Decimal::from_str("0.0050").unwrap(),
                derivation_path: String::new(), // Will be set when using HD wallets
            }
        });

        // 4. Fee Logic (0.25%)
        let col_dec = Decimal::from(collateral_amount);
        let fee_dec = col_dec * vault.mint_fee_rate;
        let fee = fee_dec.to_u64().unwrap_or(0);
        
        let net_collateral = collateral_amount - fee;

        // 5. Update State
        vault.collateral_balance += net_collateral;
        vault.accumulated_fees += fee;
        vault.minted_supply += requested_mint_amount;

        // Update implied rate
        if vault.collateral_balance > 0 {
            // Rate = Minted / Collateral
            let minted_dec = Decimal::from(vault.minted_supply);
            let col_bal_dec = Decimal::from(vault.collateral_balance);
            vault.exchange_rate = minted_dec / col_bal_dec;
        }

        if let Some(s) = &self.storage {
             let _ = s.save_vault(&vault.compass_asset, vault);
        }

        eprintln!(
            "   [Fee] Charged {} Units ({}%)",
            fee,
            vault.mint_fee_rate * Decimal::from(100)
        );
        eprintln!("   [Mint] Created {} {}", requested_mint_amount, asset_name);
        eprintln!(
            "   [Backing] 1 {} backed by ~{:.8} {}",
            asset_name,
            (net_collateral as f64 / requested_mint_amount as f64),
            collateral_ticker
        );

        Ok((asset_name, requested_mint_amount))
    }

    /// Called when user burns Compass to redeem collateral
    /// Returns the amount of Collateral to release
    pub fn burn_and_redeem(
        &mut self,
        compass_asset: &str,
        burn_amount: u64,
    ) -> Result<u64, String> {
        let vault = self
            .vaults
            .get_mut(compass_asset)
            .ok_or("Vault not found")?;

        if burn_amount > vault.minted_supply {
            return Err("Burn amount exceeds minted supply (Critical Error)".to_string());
        }

        // 1. Calculate Collateral Value of the burnt tokens
        if vault.exchange_rate.is_zero() {
            return Err("Invalid exchange rate".to_string());
        }
        
        let burn_dec = Decimal::from(burn_amount);
        // Collateral = Burn / Rate
        let gross_collateral_dec = burn_dec / vault.exchange_rate;
        let gross_collateral_value = gross_collateral_dec.to_u64().unwrap_or(0);

        if gross_collateral_value == 0 {
            return Err("Burn amount too small to redeem any collateral".to_string());
        }

        if vault.collateral_balance < gross_collateral_value {
            return Err("Critical Error: Vault Undercollateralized! Cannot redeem.".to_string());
        }

        // 2. Calculate Fee (0.5%)
        let fee_dec = gross_collateral_dec * vault.redeem_fee_rate;
        let fee = fee_dec.to_u64().unwrap_or(0);
        let net_payout = gross_collateral_value - fee;

        // 3. Update Vault State
        vault.minted_supply -= burn_amount;
        vault.collateral_balance -= gross_collateral_value; // Deduct gross (User + Fee)
        vault.accumulated_fees += fee; // Keep fee
        
        if let Some(s) = &self.storage {
             let _ = s.save_vault(&vault.compass_asset, vault);
        }

        eprintln!(
            "   [Redeem] Burning {} Compass -> Releasing {} Collateral",
            burn_amount, gross_collateral_value
        );
        eprintln!(
            "   [Fee] Charged {} Units ({}%)",
            fee,
            vault.redeem_fee_rate * Decimal::from(100)
        );
        eprintln!("   [Payout] {} Units", net_payout);

        Ok(net_payout)
    }

    /// Update Oracle Price
    pub fn update_oracle_price(
        &mut self,
        ticker: &str,
        price: Decimal,
        timestamp: u64,
        signature_hex: &str,
        pubkey_hex: &str,
    ) -> Result<(), String> {
        // 1. Verify Signature using standardized crypto module
        let msg = format!("PRICE:{}:{}:{}", ticker, price, timestamp);
        
        if !crate::crypto::verify_with_pubkey_hex(msg.as_bytes(), signature_hex, pubkey_hex) {
            return Err("Invalid Oracle Price Signature".to_string());
        }

        // 2. Check Freshness (prevent replay)
        // Hardcoded: Must be newer than current stored price timestamp
        if let Some((_, old_ts)) = self.oracle_prices.get(ticker) {
            if timestamp <= *old_ts {
                return Err("Price update is too old".to_string());
            }
        }
        
        // 3. Update
        self.oracle_prices.insert(ticker.to_string(), (price, timestamp));
        if let Some(s) = &self.storage {
             let _ = s.save_oracle_price_info(ticker, &(price, timestamp));
        }
        Ok(())
    }

    /// Liquidate an undercollateralized vault
    pub fn liquidate(
        &mut self,
        compass_asset: &str,
        burn_amount: u64,
    ) -> Result<u64, String> {
        let vault = self.vaults.get_mut(compass_asset).ok_or("Vault not found")?;

        // 1. Get Global Price
        let ticker = &vault.collateral_asset;
        let (price, timestamp) = *self.oracle_prices.get(ticker).ok_or("No Oracle Price for asset")?;
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        if now > timestamp + 3600 {
            return Err("Oracle Price is Stale (>1 hour old). Cannot Liquidate.".to_string());
        }

        if price.is_zero() { return Err("Invalid Oracle Price".to_string()); }

        let health = vault.check_health(price);
        if health != VaultHealth::Liquidatable {
            return Err("Vault is NOT eligible for liquidation.".to_string());
        }

        // 2. Cap burn amount to total debt
        let debt = vault.minted_supply;
        if burn_amount > debt {
             return Err("Cannot liquidate more than total debt".to_string());
        }

        // 3. Calculate Payout
        let burn_dec = Decimal::from(burn_amount);
        let base_col = burn_dec / price;
        let bonus = base_col * Decimal::from_str("0.10").unwrap();
        let total_payout_dec = base_col + bonus;
        let total_payout = total_payout_dec.to_u64().unwrap_or(0);

        // 4. Check Vault Solvency
        if total_payout > vault.collateral_balance {
            let payout = vault.collateral_balance;
            vault.collateral_balance = 0;
            vault.minted_supply -= burn_amount;
            if let Some(s) = &self.storage {
                let _ = s.save_vault(&vault.compass_asset, vault);
            }
            return Ok(payout);
        }

        // 5. Apply
        vault.minted_supply -= burn_amount;
        vault.collateral_balance -= total_payout;

        if let Some(s) = &self.storage {
             let _ = s.save_vault(&vault.compass_asset, vault);
        }

        Ok(total_payout)
    }
}
