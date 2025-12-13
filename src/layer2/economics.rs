use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenomicsEngine {
    pub inflation_rate: f64, // Annual inflation rate (e.g. 0.05 for 5%)
    pub total_supply: u128,
    pub total_staked: u128,
    pub burned_tokens: u128,
}

impl TokenomicsEngine {
    pub fn new() -> Self {
        Self {
            inflation_rate: 0.05, // 5% default
            total_supply: 1_000_000_000, // Initial 1 Billion
            total_staked: 0,
            burned_tokens: 0,
        }
    }

    /// Calculate block reward based on current supply and inflation
    pub fn calculate_block_reward(&self) -> u64 {
        let annual_tokens = self.total_supply as f64 * self.inflation_rate;
        let seconds_per_year = 31_536_000.0;
        let tokens_per_second = annual_tokens / seconds_per_year;
        tokens_per_second as u64
    }

    pub fn burn(&mut self, amount: u64) {
        self.burned_tokens += amount as u128;
        // In a real burn, we'd reduce total_supply, but sometimes it's good to track separately
        if self.total_supply >= amount as u128 {
            self.total_supply -= amount as u128;
        }
    }

    pub fn mint_rewards(&mut self, amount: u64) {
        self.total_supply += amount as u128;
    }
}
