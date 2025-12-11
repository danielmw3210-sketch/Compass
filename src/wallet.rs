use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;

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
}

impl Wallet {
    /// Create a new wallet with a given type (Generates new keys)
    pub fn new(owner: &str, wallet_type: WalletType) -> Self {
        use crate::crypto::KeyPair;
        let mnemonic = KeyPair::generate_mnemonic();
        let kp = KeyPair::from_mnemonic(&mnemonic).unwrap_or_else(|_| KeyPair::new());
        
        Wallet {
            owner: owner.to_string(),
            balances: HashMap::new(),
            wallet_type,
            nonce: 0,
            mnemonic: Some(mnemonic),
            public_key: kp.public_key_hex(),
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

    pub fn save(&self, path: &str) {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json).expect("Unable to save wallet");
    }

    pub fn load(path: &str, owner: &str, wallet_type: WalletType) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap()
        } else {
            Wallet::new(owner, wallet_type)
        }
    }
}

/// A manager for multiple wallets
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletManager {
    pub wallets: Vec<Wallet>,
}

impl WalletManager {
    pub fn new() -> Self {
        WalletManager { wallets: Vec::new() }
    }

    pub fn create_wallet(&mut self, creator: &Wallet, owner: &str, wallet_type: WalletType) {
        if creator.can_create_wallet() {
            let wallet = Wallet::new(owner, wallet_type);
            self.wallets.push(wallet);
        } else {
            println!("{} is not authorized to create wallets", creator.owner);
        }
    }

    pub fn get_wallet(&self, owner: &str) -> Option<&Wallet> {
        self.wallets.iter().find(|w| w.owner == owner)
    }

    pub fn get_wallet_mut(&mut self, owner: &str) -> Option<&mut Wallet> {
        self.wallets.iter_mut().find(|w| w.owner == owner)
    }

    /// Credit coins to a wallet (mint or transfer)
    pub fn credit(&mut self, owner: &str, asset: &str, amount: u64) {
        if let Some(wallet) = self.get_wallet_mut(owner) {
            *wallet.balances.entry(asset.to_string()).or_insert(0) += amount;
        } else {
            let mut balances = HashMap::new();
            balances.insert(asset.to_string(), amount);
            self.wallets.push(Wallet {
                owner: owner.to_string(),
                balances,
                wallet_type: WalletType::User,
                nonce: 0,
                mnemonic: None,
                public_key: String::new(),
            });
        }
    }

    /// Debit coins from a wallet (spend/transfer)
    pub fn debit(&mut self, owner: &str, asset: &str, amount: u64) -> bool {
        if let Some(wallet) = self.get_wallet_mut(owner) {
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
        if let Some(wallet) = self.get_wallet_mut(owner) {
            wallet.nonce = nonce;
        }
    }

    pub fn get_balance(&self, owner: &str, asset: &str) -> u64 {
        self.get_wallet(owner)
            .and_then(|w| w.balances.get(asset))
            .cloned()
            .unwrap_or(0)
    }

    pub fn save(&self, path: &str) {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json).expect("Unable to save wallets");
    }

    pub fn load(path: &str) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap()
        } else {
            WalletManager::new()
        }
    }
}