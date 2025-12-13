#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use aes_gcm::{
    aead::{Aead, KeyInit}, // Aes256Gcm trait imports
    Aes256Gcm, Nonce, // Key is generic
};
use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::Sha256;
use rand::{Rng, thread_rng};

use crate::error::CompassError;

/// Different roles a wallet can have
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum WalletType {
    Admin,     // full privileges
    User,      // normal participant
    Validator, // can mint/validate blocks
    Verifier,  // can verify GPU computations
}

/// A single wallet with multi-asset support
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wallet {
    pub owner: String,
    pub balances: HashMap<String, u64>,
    pub wallet_type: WalletType,
    pub nonce: u64,
    #[serde(default)]
    pub mnemonic: Option<String>,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub encrypted_mnemonic: Option<Vec<u8>>,
    #[serde(default)]
    pub encryption_salt: Option<Vec<u8>>,
    #[serde(default)]
    pub is_encrypted: bool,
}

impl Wallet {
    /// Create a new wallet with a given type (Generates new keys)
    pub fn new(owner: &str, wallet_type: WalletType) -> Self {
        use crate::crypto::KeyPair;
        let mnemonic = KeyPair::generate_mnemonic();
        let kp = KeyPair::from_mnemonic(&mnemonic).unwrap_or_else(|_| KeyPair::generate());

        Wallet {
            owner: owner.to_string(),
            balances: HashMap::new(),
            wallet_type,
            nonce: 0,
            mnemonic: Some(mnemonic),
            public_key: kp.public_key_hex(),
            encrypted_mnemonic: None,
            encryption_salt: None,
            is_encrypted: false,
        }
    }

    /// Create a watch-only wallet or account entry (Node side)
    pub fn new_account(owner: &str) -> Self {
        Wallet {
            owner: owner.to_string(),
            balances: HashMap::new(),
            wallet_type: WalletType::User,
            nonce: 0,
            mnemonic: None,
            public_key: String::new(), // In real app, account ID *is* pubkey
            encrypted_mnemonic: None,
            encryption_salt: None,
            is_encrypted: false,
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.wallet_type, WalletType::Admin)
    }

    pub fn can_create_wallet(&self) -> bool {
        self.is_admin()
    }

    pub fn can_mint(&self) -> bool {
        self.is_admin() || matches!(self.wallet_type, WalletType::Validator)
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn load(path: &str, owner: &str, wallet_type: WalletType) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap()
        } else {
            Wallet::new(owner, wallet_type)
        }
    }
    pub fn get_keypair(&self) -> Option<crate::crypto::KeyPair> {
        if let Some(ref m) = self.mnemonic {
            crate::crypto::KeyPair::from_mnemonic(m).ok()
        } else {
            None
        }
    }

    pub fn encrypt_wallet(&mut self, password: &str) -> Result<(), CompassError> {
        if self.mnemonic.is_none() {
            return Err(CompassError::InvalidState("No mnemonic to encrypt".to_string()));
        }
        let mnemonic_str = self.mnemonic.as_ref().unwrap();

        // Generate Salt
        let mut salt = [0u8; 16];
        thread_rng().fill(&mut salt);
        
        // Derive Key PBKDF2
        let mut key = [0u8; 32]; // AES-256
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, 100_000, &mut key);

        // Encrypt
        let cipher = Aes256Gcm::new(&key.into());
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, mnemonic_str.as_bytes())
            .map_err(|e| CompassError::InvalidState(format!("Encryption failure: {:?}", e)))?;

        // Store
        // We need to store Nonce too. Let's prepend nonce to ciphertext or store separate?
        // Simplest: Store Nonce + Ciphertext in encrypted_mnemonic
        let mut final_blob = Vec::new();
        final_blob.extend_from_slice(&nonce_bytes);
        final_blob.extend_from_slice(&ciphertext);

        self.encrypted_mnemonic = Some(final_blob);
        self.encryption_salt = Some(salt.to_vec());
        self.is_encrypted = true;
        self.mnemonic = None; // Clear plaintext

        println!("Wallet encrypted for user {}", self.owner);
        Ok(())
    }

    pub fn decrypt_wallet(&mut self, password: &str) -> Result<(), CompassError> {
        if !self.is_encrypted {
             return Ok(()); // Already decrypted
        }
        let blob = self.encrypted_mnemonic.as_ref().ok_or(CompassError::InvalidState("No encrypted data".to_string()))?;
        let salt = self.encryption_salt.as_ref().ok_or(CompassError::InvalidState("No salt".to_string()))?;

        if blob.len() < 12 {
            return Err(CompassError::InvalidState("Invalid blob size".to_string()));
        }

        let nonce_bytes = &blob[0..12];
        let ciphertext = &blob[12..];

        // Derive Key
        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 100_000, &mut key);

        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext)
             .map_err(|_| CompassError::InvalidState("Decryption failed (Wrong password?)".to_string()))?;

        let mnemonic_str = String::from_utf8(plaintext)
             .map_err(|_| CompassError::InvalidState("Invalid UTF8".to_string()))?;

        self.mnemonic = Some(mnemonic_str);
        self.is_encrypted = false;
        // Keep encrypted fields for re-locking? Or clear them?
        // Usually we clear them if we want to change password, but if we just unlock in memory...
        // Let's clear them to avoid inconsistency if we allow editing.
        self.encrypted_mnemonic = None;
        self.encryption_salt = None;

        println!("Wallet decrypted for user {}", self.owner);
        Ok(())
    }
}

/// A manager for multiple wallets
/// A manager for multiple wallets
#[derive(Serialize, Deserialize, Clone)] // Removed generic Debug derive due to skip field complexity usually, but Arc is Debug? No Storage isn't.
pub struct WalletManager {
    // Key: Owner ID (String) -> Value: Wallet
    pub wallets: HashMap<String, Wallet>,
    #[serde(skip)]
    pub storage: Option<std::sync::Arc<crate::storage::Storage>>,
}

impl std::fmt::Debug for WalletManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletManager")
         .field("wallets", &self.wallets)
         .field("storage", &"Option<Storage>")
         .finish()
    }
}

impl WalletManager {
    pub fn new() -> Self {
        WalletManager {
            wallets: HashMap::new(),
            storage: None,
        }
    }

    pub fn new_with_storage(storage: std::sync::Arc<crate::storage::Storage>) -> Self {
        let mut wm = WalletManager {
            wallets: HashMap::new(),
            storage: Some(storage.clone()),
        };
        // Load existing wallets from DB
        let wallets = storage.get_all_wallets();
        for w in wallets {
            wm.wallets.insert(w.owner.clone(), w);
        }
        wm
    }

    pub fn create_wallet(&mut self, creator: &Wallet, owner: &str, wallet_type: WalletType) {
        if creator.can_create_wallet() {
            if self.wallets.contains_key(owner) {
                println!("Wallet for {} already exists", owner);
                return;
            }
            let wallet = Wallet::new(owner, wallet_type);
            self.wallets.insert(owner.to_string(), wallet.clone());
            
            // Auto-persist new creation
            if let Some(s) = &self.storage {
                let _ = s.save_wallet(&wallet);
            }
        } else {
            println!("{} is not authorized to create wallets", creator.owner);
        }
    }

    pub fn get_wallet(&self, owner: &str) -> Option<&Wallet> {
        self.wallets.get(owner)
    }

    pub fn get_wallet_mut(&mut self, owner: &str) -> Option<&mut Wallet> {
        self.wallets.get_mut(owner)
    }

    /// Credit coins to a wallet (mint or transfer)
    pub fn credit(&mut self, owner: &str, asset: &str, amount: u64) {
        if let Some(wallet) = self.wallets.get_mut(owner) {
            *wallet.balances.entry(asset.to_string()).or_insert(0) += amount;
            // Note: We intentionally don't auto-save here to batch updates, user calls save()
        } else {
            let mut balances = HashMap::new();
            balances.insert(asset.to_string(), amount);
            use crate::crypto::KeyPair;
            let mnemonic = KeyPair::generate_mnemonic();
            let kp = KeyPair::from_mnemonic(&mnemonic).unwrap_or_else(|_| KeyPair::generate());

            let wallet = Wallet {
                owner: owner.to_string(),
                balances,
                wallet_type: WalletType::User,
                nonce: 0,
                mnemonic: Some(mnemonic),
                public_key: kp.public_key_hex(),
                encrypted_mnemonic: None,
                encryption_salt: None,
                is_encrypted: false,
            };
            self.wallets.insert(owner.to_string(), wallet);
        }
    }

    /// Debit coins from a wallet (spend/transfer)
    pub fn debit(&mut self, owner: &str, asset: &str, amount: u64) -> bool {
        if let Some(wallet) = self.wallets.get_mut(owner) {
            let balance = wallet.balances.entry(asset.to_string()).or_insert(0);
            if *balance >= amount {
                *balance -= amount;
                return true;
            }
        }
        false
    }

    /// Get current nonce
    pub fn get_nonce(&self, owner: &str) -> u64 {
        self.get_wallet(owner).map(|w| w.nonce).unwrap_or(0)
    }

    /// Set nonce (after successful transfer)
    pub fn set_nonce(&mut self, owner: &str, nonce: u64) {
        if let Some(wallet) = self.wallets.get_mut(owner) {
            wallet.nonce = nonce;
        }
    }

    pub fn get_balance(&self, owner: &str, asset: &str) -> u64 {
        self.get_wallet(owner)
            .and_then(|w| w.balances.get(asset))
            .cloned()
            .unwrap_or(0)
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(s) = &self.storage {
            // Persist all dirty state to Sled
            for wallet in self.wallets.values() {
                let _ = s.save_wallet(wallet);
            }
            let _ = s.flush();
            Ok(())
        } else {
            let json = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            fs::write(path, json)
        }
    }

    pub fn load(path: &str) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_else(|_| WalletManager::new())
        } else {
            WalletManager::new()
        }
    }
}
