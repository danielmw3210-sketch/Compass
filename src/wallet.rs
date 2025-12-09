use serde::{Serialize, Deserialize};
use std::fs;

/// Different roles a wallet can have
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum WalletType {
    Admin,     // full privileges
    User,      // normal participant
    Validator, // can mint/validate blocks
    Verifier,  // can verify GPU computations
}

/// A single wallet
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wallet {
    pub owner: String,
    pub balance: u64,
    pub wallet_type: WalletType,
    pub nonce: u64, // NEW: replay protection
}

impl Wallet {
    /// Create a new wallet with a given type
    pub fn new(owner: &str, wallet_type: WalletType) -> Self {
        Wallet {
            owner: owner.to_string(),
            balance: 0,
            wallet_type,
            nonce: 0,
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
    pub fn credit(&mut self, owner: &str, amount: u64) {
        if let Some(wallet) = self.get_wallet_mut(owner) {
            wallet.balance += amount;
        } else {
            self.wallets.push(Wallet {
                owner: owner.to_string(),
                balance: amount,
                wallet_type: WalletType::User,
                nonce: 0,
            });
        }
    }

    /// Debit coins from a wallet (spend/transfer)
    pub fn debit(&mut self, owner: &str, amount: u64) -> bool {
        if let Some(wallet) = self.get_wallet_mut(owner) {
            if wallet.balance >= amount {
                wallet.balance -= amount;
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

    pub fn get_balance(&self, owner: &str) -> u64 {
        self.get_wallet(owner).map(|w| w.balance).unwrap_or(0)
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