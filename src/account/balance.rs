//! Balance tracking for multi-layer, multi-asset system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::types::{AccountId, Asset};

/// Balance store for all accounts and assets across all layers
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BalanceStore {
    /// Map of (AccountId, Asset) -> Balance
    balances: HashMap<(AccountId, Asset), u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BalanceError {
    InsufficientFunds,
    AccountNotFound,
    InvalidAmount,
    Overflow,
}

impl BalanceStore {
    /// Create a new empty balance store
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }
    
    /// Get balance for an account and asset
    pub fn get_balance(&self, account: &AccountId, asset: &Asset) -> u64 {
        self.balances
            .get(&(account.clone(), asset.clone()))
            .copied()
            .unwrap_or(0)
    }
    
    /// Get all balances for an account
    pub fn get_all_balances(&self, account: &AccountId) -> HashMap<Asset, u64> {
        self.balances
            .iter()
            .filter(|((acc, _), _)| acc == account)
            .map(|((_, asset), balance)| (asset.clone(), *balance))
            .collect()
    }
    
    /// Get all accounts holding a specific asset
    pub fn get_holders(&self, asset: &Asset) -> Vec<(AccountId, u64)> {
        self.balances
            .iter()
            .filter(|((_, ast), balance)| ast == asset && **balance > 0)
            .map(|((acc, _), balance)| (acc.clone(), *balance))
            .collect()
    }
    
    /// Credit (add) balance to an account
    pub fn credit(&mut self, account: &AccountId, asset: &Asset, amount: u64) -> Result<(), BalanceError> {
        if amount == 0 {
            return Ok(());
        }
        
        let key = (account.clone(), asset.clone());
        let current = self.balances.get(&key).copied().unwrap_or(0);
        
        // Check for overflow
        let new_balance = current.checked_add(amount)
            .ok_or(BalanceError::Overflow)?;
        
        self.balances.insert(key, new_balance);
        Ok(())
    }
    
    /// Debit (subtract) balance from an account
    pub fn debit(&mut self, account: &AccountId, asset: &Asset, amount: u64) -> Result<(), BalanceError> {
        if amount == 0 {
            return Ok(());
        }
        
        let key = (account.clone(), asset.clone());
        let current = self.balances.get(&key).copied().unwrap_or(0);
        
        if current < amount {
            return Err(BalanceError::InsufficientFunds);
        }
        
        let new_balance = current - amount;
        if new_balance == 0 {
            self.balances.remove(&key);
        } else {
            self.balances.insert(key, new_balance);
        }
        
        Ok(())
    }
    
    /// Transfer balance from one account to another
    pub fn transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        asset: &Asset,
        amount: u64,
    ) -> Result<(), BalanceError> {
        // Debit from sender
        self.debit(from, asset, amount)?;
        
        // Credit to recipient
        if let Err(e) = self.credit(to, asset, amount) {
            // Rollback on error
            self.credit(from, asset, amount).ok();
            return Err(e);
        }
        
        Ok(())
    }
    
    /// Set balance directly (for genesis/admin operations)
    pub fn set_balance(&mut self, account: &AccountId, asset: &Asset, amount: u64) {
        let key = (account.clone(), asset.clone());
        if amount == 0 {
            self.balances.remove(&key);
        } else {
            self.balances.insert(key, amount);
        }
    }
    
    /// Get total supply of an asset
    pub fn total_supply(&self, asset: &Asset) -> u64 {
        self.balances
            .iter()
            .filter(|((_, ast), _)| ast == asset)
            .map(|(_, balance)| balance)
            .sum()
    }
    
    /// Get number of unique accounts
    pub fn account_count(&self) -> usize {
        self.balances
            .keys()
            .map(|(acc, _)| acc)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}

impl Default for BalanceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_balance_operations() {
        let mut store = BalanceStore::new();
        
        // Test credit
        store.credit(&"alice".to_string(), &"COMPASS".to_string(), 1000).unwrap();
        assert_eq!(store.get_balance(&"alice".to_string(), &"COMPASS".to_string()), 1000);
        
        // Test debit
        store.debit(&"alice".to_string(), &"COMPASS".to_string(), 300).unwrap();
        assert_eq!(store.get_balance(&"alice".to_string(), &"COMPASS".to_string()), 700);
        
        // Test insufficient funds
        assert!(store.debit(&"alice".to_string(), &"COMPASS".to_string(), 1000).is_err());
    }
    
    #[test]
    fn test_transfer() {
        let mut store = BalanceStore::new();
        store.credit(&"alice".to_string(), &"COMPASS".to_string(), 1000).unwrap();
        
        store.transfer(&"alice".to_string(), &"bob".to_string(), &"COMPASS".to_string(), 400).unwrap();
        
        assert_eq!(store.get_balance(&"alice".to_string(), &"COMPASS".to_string()), 600);
        assert_eq!(store.get_balance(&"bob".to_string(), &"COMPASS".to_string()), 400);
    }
    
    #[test]
    fn test_total_supply() {
        let mut store = BalanceStore::new();
        store.credit(&"alice".to_string(), &"COMPASS".to_string(), 1000).unwrap();
        store.credit(&"bob".to_string(), &"COMPASS".to_string(), 500).unwrap();
        
        assert_eq!(store.total_supply(&"COMPASS".to_string()), 1500);
    }
}
