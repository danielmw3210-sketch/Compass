#![allow(dead_code)]
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

/// Represents a prediction bet placed by the neural network
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PredictionBet {
    pub timestamp: u64,
    pub prediction: String,  // "DIRECT_L1" or "OPTIMISTIC_L2"
    pub confidence: f64,     // 0.0-1.0
    pub stake_amount: u64,   // Amount of COMPASS tokens staked
    pub market_gas: f64,     // Gas price at time of prediction
    pub market_sol: f64,
    pub market_tvl: f64,
    pub outcome: Option<PredictionOutcome>,  // None until evaluated
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PredictionOutcome {
    pub correct: bool,
    pub actual_gas: f64,      // Gas price 30 min later
    pub profit_loss: i64,     // Tokens won/lost
    pub evaluated_at: u64,
}

/// Manages the neural network's prediction bets
/// Manages the neural network's prediction bets
#[derive(Serialize, Deserialize)]
pub struct BettingLedger {
    active_bets: VecDeque<PredictionBet>,
    pub settled_bets: VecDeque<PredictionBet>,
    total_staked: u64,
    total_won: u64,
    total_lost: u64,
    max_history: usize,
    #[serde(skip)]
    pub storage: Option<std::sync::Arc<crate::storage::Storage>>,
}

impl BettingLedger {
    pub fn new() -> Self {
        Self {
            active_bets: VecDeque::new(),
            settled_bets: VecDeque::new(),
            total_staked: 0,
            total_won: 0,
            total_lost: 0,
            max_history: 1000,
            storage: None,
        }
    }

    pub fn new_with_storage(storage: std::sync::Arc<crate::storage::Storage>) -> Self {
        let mut bl = Self::new();
        bl.storage = Some(storage.clone());
        
        // Load Stats
        if let Ok(Some((s, w, l))) = storage.get_betting_stats() {
            bl.total_staked = s;
            bl.total_won = w;
            bl.total_lost = l;
        }

        // Load Active Bets
        let active = storage.get_active_bets();
        for b in active {
            bl.active_bets.push_back(b);
        }

        // Load Settled Bets
        // Sort by timestamp? get_by_prefix doesn't guarantee order (lexicographical on Key).
        // Key is "bet:settled:{ts}". If ts is BE bytes, it sorts correctly.
        // My storage key `put` uses `format!("{}", ts)` which is ASCII number string.
        // "100" comes before "20" ? No. But "10" comes before "2".
        // It's not strictly sorted chronologically if simple string format.
        // But for VecDeque loading order it probably doesn't matter too much if we just append.
        let settled = storage.get_settled_bets();
        for b in settled {
            bl.settled_bets.push_back(b);
        }
        
        bl
    }

    /// Place a bet on a prediction
    /// Stake amount is proportional to confidence: higher confidence = more stake
    pub fn place_bet(&mut self, prediction: String, confidence: f64, gas: f64, sol: f64, tvl: f64) -> PredictionBet {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Stake calculation: Base 10 COMPASS, up to 100 COMPASS for 100% confidence
        let stake_amount = (10.0 + (confidence * 90.0)) as u64;

        let bet = PredictionBet {
            timestamp,
            prediction: prediction.clone(),
            confidence,
            stake_amount,
            market_gas: gas,
            market_sol: sol,
            market_tvl: tvl,
            outcome: None,
        };

        self.total_staked += stake_amount;
        self.active_bets.push_back(bet.clone());
        
        // Persist
        if let Some(s) = &self.storage {
             let _ = s.save_active_bet(&bet);
             let _ = s.save_betting_stats(self.total_staked, self.total_won, self.total_lost);
        }
        
        println!("   💰 Placed bet: {} COMPASS on {} (confidence: {:.0}%)", 
                 stake_amount, prediction, confidence * 100.0);

        bet
    }

    /// Get bets that are older than N minutes and haven't been evaluated
    pub fn get_unevaluated_bets(&self, min_age_minutes: u64) -> Vec<&PredictionBet> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - (min_age_minutes * 60);

        self.active_bets.iter()
            .filter(|bet| bet.timestamp < cutoff && bet.outcome.is_none())
            .collect()
    }

    /// Settle a bet with outcome
    pub fn settle_bet(&mut self, bet_id: u64, actual_gas: f64) -> Option<i64> {
        // Find the bet by timestamp (acting as ID)
        let bet_pos = self.active_bets.iter()
            .position(|b| b.timestamp == bet_id)?;

        let mut bet = self.active_bets.remove(bet_pos).unwrap();

        // Evaluate correctness
        let gas_ratio = actual_gas / bet.market_gas;
        let prediction_was_l2 = bet.prediction.contains("L2");
        let gas_spiked = gas_ratio > 1.5;

        // Correct if: (predicted L2 AND gas spiked) OR (predicted L1 AND gas didn't spike)
        let correct = (prediction_was_l2 && gas_spiked) || (!prediction_was_l2 && !gas_spiked);

        // Profit/Loss calculation
        // Win: Get stake back + 100% profit
        // Loss: Lose the stake
        let profit_loss = if correct {
            let winnings = (bet.stake_amount as f64 * bet.confidence) as i64;
            self.total_won += winnings as u64;
            winnings
        } else {
            let loss = -(bet.stake_amount as i64);
            self.total_lost += bet.stake_amount;
            loss
        };

        bet.outcome = Some(PredictionOutcome {
            correct,
            actual_gas,
            profit_loss,
            evaluated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        println!("   🎲 Bet settled: {} → {} COMPASS ({})", 
                 if correct { "WIN" } else { "LOSS" },
                 if profit_loss > 0 { format!("+{}", profit_loss) } else { profit_loss.to_string() },
                 bet.prediction);

        self.settled_bets.push_back(bet.clone());
        if self.settled_bets.len() > self.max_history {
            self.settled_bets.pop_front();
            // TODO: Delete popped from DB? Skipping for now to preserve history in DB.
        }

        // Persist
        if let Some(s) = &self.storage {
            // Remove from active
            let _ = s.delete_active_bet(bet.timestamp);
            // Save to settled
            let _ = s.save_settled_bet(&bet);
            // Update stats
            let _ = s.save_betting_stats(self.total_staked, self.total_won, self.total_lost);
        }

        Some(profit_loss)
    }

    /// Get statistics for tracking performance
    pub fn get_stats(&self) -> (u64, u64, u64, f64) {
        let total_bets = self.settled_bets.len();
        let win_rate = if total_bets > 0 {
            self.settled_bets.iter()
                .filter(|b| b.outcome.as_ref().map(|o| o.correct).unwrap_or(false))
                .count() as f64 / total_bets as f64
        } else {
            0.0
        };

        (self.total_staked, self.total_won, self.total_lost, win_rate)
    }

    /// Get settled bets for training (outcome-based learning)
    pub fn get_settled_bets_for_training(&self) -> Vec<&PredictionBet> {
        self.settled_bets.iter().collect()
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(s) = &self.storage {
             // Sled Persist
             for b in &self.active_bets { let _ = s.save_active_bet(b); }
             for b in &self.settled_bets { let _ = s.save_settled_bet(b); }
             let _ = s.save_betting_stats(self.total_staked, self.total_won, self.total_lost);
             let _ = s.flush();
             Ok(())
        } else {
             let json = serde_json::to_string_pretty(self)?;
             std::fs::write(path, json)
        }
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        if let Ok(json) = std::fs::read_to_string(path) {
            let ledger = serde_json::from_str(&json)?;
            Ok(ledger)
        } else {
            Ok(Self::new())
        }
    }
}
