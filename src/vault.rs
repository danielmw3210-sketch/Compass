use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use hex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vault {
    pub collateral_asset: String, // e.g., "SOL"
    pub compass_asset: String,    // e.g., "Compass-SOL"
    pub vault_address: String,    // External Chain Address (e.g. Solana Addr)
    pub exchange_rate: u64,       // Amount of Compass per 1 unit of Collateral
    pub collateral_balance: u64,  // Total External Asset Locked
    pub minted_supply: u64,       // Total Compass-Asset Minted
    pub accumulated_fees: u64,    // Fees collected
    pub mint_fee_rate: f64,       // e.g. 0.0025
    pub redeem_fee_rate: f64,     // e.g. 0.0050
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VaultManager {
    // Keyed by compass_asset name (e.g., "Compass-SOL")
    pub vaults: HashMap<String, Vault>,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
        }
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
    pub fn save(&self, path: &str) {
        let data = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, data).expect("Unable to save vaults");
    }

    /// Register a new vault type (e.g. "Compass-SOL")
    pub fn register_vault(&mut self, collateral: &str, compass_asset: &str, address: &str, rate: u64) -> Result<(), String> {
        if self.vaults.contains_key(compass_asset) {
            return Err("Vault already exists for this asset".to_string());
        }
        let vault = Vault {
            collateral_asset: collateral.to_string(),
            compass_asset: compass_asset.to_string(),
            vault_address: address.to_string(),
            exchange_rate: rate,
            collateral_balance: 0,
            minted_supply: 0,
            accumulated_fees: 0,
            mint_fee_rate: 0.0025,   // 0.25% Default
            redeem_fee_rate: 0.0050, // 0.50% Default
        };
        self.vaults.insert(compass_asset.to_string(), vault);
        Ok(())
    }



    /// Get info for dual-nature display
    pub fn get_asset_info(&self, compass_asset: &str) -> Option<(u64, u64, String)> {
        self.vaults.get(compass_asset).map(|v| (v.collateral_balance, v.minted_supply, v.collateral_asset.clone()))
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
        oracle_pubkey_hex: &str
    ) -> Result<(String, u64), String> { // Returns (Asset Name, Minted Amount)
        // 1. Verify Signature FIRST
        let pubkey_bytes = hex::decode(oracle_pubkey_hex).map_err(|_| "Invalid pubkey hex")?;
        let pubkey = PublicKey::from_bytes(&pubkey_bytes).map_err(|_| "Invalid pubkey bytes")?;
        
        let sig_bytes = hex::decode(oracle_sig_hex).map_err(|_| "Invalid sig hex")?;
        let signature = Signature::from_bytes(&sig_bytes).map_err(|_| "Invalid sig bytes")?;

        // Proof includes User Intent (Mint Amount) + Owner Identity
        let msg = format!("DEPOSIT:{}:{}:{}:{}:{}", collateral_ticker, collateral_amount, tx_hash, requested_mint_amount, owner_id);
        pubkey.verify(msg.as_bytes(), &signature).map_err(|_| "Invalid Oracle Signature! Deposit not verified.")?;

        // 2. Determine Asset Name (Traceable Owner)
        // e.g. "Compass:Daniel:LTC"
        let asset_name = format!("Compass:{}:{}", owner_id, collateral_ticker);

        // 3. Get or Create Personal Vault
        let vault = self.vaults.entry(asset_name.clone()).or_insert_with(|| {
            Vault {
                collateral_asset: collateral_ticker.to_string(),
                compass_asset: asset_name.clone(),
                vault_address: "SharedOracleVault".to_string(), // In reality, might be specific per user, but for now shared vault address.
                exchange_rate: 0, // Dynamic, calculated per state
                collateral_balance: 0,
                minted_supply: 0,
                accumulated_fees: 0,
                mint_fee_rate: 0.0025,
                redeem_fee_rate: 0.0050,
            }
        });

        // 4. Fee Logic (0.25%)
        let fee = (collateral_amount as f64 * vault.mint_fee_rate) as u64;
        let net_collateral = collateral_amount - fee;

        // 5. Update State
        vault.collateral_balance += net_collateral;
        vault.accumulated_fees += fee;
        vault.minted_supply += requested_mint_amount;
        
        // Update implied rate (just for display/tracking, not enforcement since user decides)
        if vault.collateral_balance > 0 {
             vault.exchange_rate = vault.minted_supply / vault.collateral_balance;
        }

        println!("   [Fee] Charged {} Units ({}%)", fee, vault.mint_fee_rate * 100.0);
        println!("   [Mint] Created {} {}", requested_mint_amount, asset_name);
        println!("   [Backing] 1 {} backed by ~{:.8} {}", asset_name, (net_collateral as f64 / requested_mint_amount as f64), collateral_ticker);

        Ok((asset_name, requested_mint_amount))
    }

    /// Called when user burns Compass to redeem collateral
    /// Returns the amount of Collateral to release
    pub fn burn_and_redeem(&mut self, compass_asset: &str, burn_amount: u64) -> Result<u64, String> {
        let vault = self.vaults.get_mut(compass_asset).ok_or("Vault not found")?;
        
        if burn_amount > vault.minted_supply {
            return Err("Burn amount exceeds minted supply (Critical Error)".to_string());
        }

        // 1. Calculate Collateral Value of the burnt tokens
        if vault.exchange_rate == 0 { return Err("Invalid exchange rate".to_string()); }
        let gross_collateral_value = burn_amount / vault.exchange_rate;

        if gross_collateral_value == 0 {
             return Err("Burn amount too small to redeem any collateral".to_string());
        }

        if vault.collateral_balance < gross_collateral_value {
             return Err("Critical Error: Vault Undercollateralized! Cannot redeem.".to_string());
        }

        // 2. Calculate Fee (0.5%)
        let fee = (gross_collateral_value as f64 * vault.redeem_fee_rate) as u64;
        let net_payout = gross_collateral_value - fee;

        // 3. Update Vault State
        vault.minted_supply -= burn_amount;
        vault.collateral_balance -= gross_collateral_value; // Deduct gross (User + Fee)
        vault.accumulated_fees += fee; // Keep fee

        println!("   [Redeem] Burning {} Compass -> Releasing {} Collateral", burn_amount, gross_collateral_value);
        println!("   [Fee] Charged {} Units ({}%)", fee, vault.redeem_fee_rate * 100.0);
        println!("   [Payout] {} Units", net_payout);

        Ok(net_payout)
    }
}
