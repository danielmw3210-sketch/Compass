use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralManager {
    // Map: Entity ID (Validator/Worker/Model) -> Staked Amount
    pub stakes: HashMap<String, u64>,
    // Insurance Fund Balance
    pub insurance_fund: u128,
}

impl CollateralManager {
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            insurance_fund: 0,
        }
    }

    pub fn stake(&mut self, entity: String, amount: u64) {
        *self.stakes.entry(entity).or_insert(0) += amount;
    }

    pub fn unstake(&mut self, entity: &str, amount: u64) -> Result<(), String> {
        if let Some(balance) = self.stakes.get_mut(entity) {
            if *balance >= amount {
                *balance -= amount;
                return Ok(());
            }
        }
        Err("Insufficient stake".to_string())
    }

    pub fn slash(&mut self, entity: &str, amount: u64) -> Result<u64, String> {
        if let Some(balance) = self.stakes.get_mut(entity) {
            let slash_amount = std::cmp::min(*balance, amount);
            *balance -= slash_amount;
            self.insurance_fund += slash_amount as u128; // Slashing goes to insurance
            return Ok(slash_amount);
        }
        Err("Entity has no stake".to_string())
    }

    pub fn reward(&mut self, entity: String, amount: u64) {
        *self.stakes.entry(entity).or_insert(0) += amount;
    }
}
