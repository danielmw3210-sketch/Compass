//! Paper Trading Engine
//! 
//! Simulates real trading based on model signals to verify profitability
//! before launching P2P trading platform.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single paper trade with entry, exit, and P&L
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaperTrade {
    pub trade_id: String,
    pub ticker: String,
    pub model_id: String,
    pub timeframe: String,
    
    // Trade details
    pub signal: TradingSignal,
    pub entry_price: f64,
    pub entry_time: u64,
    pub position_size: f64,  // In USD
    
    // Exit details (filled when closed)
    pub exit_price: Option<f64>,
    pub exit_time: Option<u64>,
    pub pnl: Option<f64>,  // Profit/Loss in USD
    pub pnl_percentage: Option<f64>,
    pub is_profitable: Option<bool>,
    
    // Metadata
    pub prediction_id: String,
    pub status: TradeStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TradingSignal {
    Buy,
    Sell,
    Hold,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TradeStatus {
    Open,
    Closed,
    Cancelled,
}

impl PaperTrade {
    pub fn new(
        ticker: &str,
        model_id: &str,
        timeframe: &str,
        signal: TradingSignal,
        entry_price: f64,
        position_size: f64,
        prediction_id: &str,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let trade_id = format!("TRADE_{}_{}", ticker, now);
        
        Self {
            trade_id,
            ticker: ticker.to_string(),
            model_id: model_id.to_string(),
            timeframe: timeframe.to_string(),
            signal,
            entry_price,
            entry_time: now,
            position_size,
            exit_price: None,
            exit_time: None,
            pnl: None,
            pnl_percentage: None,
            is_profitable: None,
            prediction_id: prediction_id.to_string(),
            status: TradeStatus::Open,
        }
    }
    
    /// Close the trade and calculate P&L
    pub fn close(&mut self, exit_price: f64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.exit_price = Some(exit_price);
        self.exit_time = Some(now);
        
        // Calculate P&L based on signal direction
        let price_change = exit_price - self.entry_price;
        let pnl = match self.signal {
            TradingSignal::Buy => {
                // Long position: profit when price goes up
                (price_change / self.entry_price) * self.position_size
            }
            TradingSignal::Sell => {
                // Short position: profit when price goes down
                (-price_change / self.entry_price) * self.position_size
            }
            TradingSignal::Hold => 0.0,  // No position, no P&L
        };
        
        let pnl_pct = (pnl / self.position_size) * 100.0;
        
        self.pnl = Some(pnl);
        self.pnl_percentage = Some(pnl_pct);
        self.is_profitable = Some(pnl > 0.0);
        self.status = TradeStatus::Closed;
    }
}

/// Trading portfolio tracking balance and positions
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradingPortfolio {
    pub portfolio_id: String,
    pub starting_capital: f64,
    pub current_balance: f64,
    pub total_pnl: f64,
    
    // Trade tracking
    pub open_trades: HashMap<String, PaperTrade>,  // ticker -> trade
    pub closed_trades: Vec<PaperTrade>,
    
    // Performance metrics
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub win_rate: f64,
    pub total_profit: f64,
    pub total_loss: f64,
    pub max_balance: f64,
    pub max_drawdown: f64,
    
    // Per-model breakdown
    pub model_performance: HashMap<String, ModelPerformance>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelPerformance {
    pub model_id: String,
    pub trades: u64,
    pub wins: u64,
    pub losses: u64,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_pnl_per_trade: f64,
}

impl TradingPortfolio {
    pub fn new(starting_capital: f64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            portfolio_id: format!("PORTFOLIO_{}", now),
            starting_capital,
            current_balance: starting_capital,
            total_pnl: 0.0,
            open_trades: HashMap::new(),
            closed_trades: Vec::new(),
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_profit: 0.0,
            total_loss: 0.0,
            max_balance: starting_capital,
            max_drawdown: 0.0,
            model_performance: HashMap::new(),
        }
    }
    
    /// Open a new trade
    pub fn open_trade(&mut self, trade: PaperTrade) -> Result<(), String> {
        // Check if already have open position for this ticker
        if self.open_trades.contains_key(&trade.ticker) {
            return Err(format!("Already have open position for {}", trade.ticker));
        }
        
        // Check if we have enough balance
        if trade.position_size > self.current_balance {
            return Err(format!("Insufficient balance: {} > {}", trade.position_size, self.current_balance));
        }
        
        // Reserve capital for position
        self.current_balance -= trade.position_size;
        self.open_trades.insert(trade.ticker.clone(), trade);
        
        Ok(())
    }
    
    /// Close an open trade
    pub fn close_trade(&mut self, ticker: &str, exit_price: f64) -> Result<(), String> {
        let mut trade = self.open_trades.remove(ticker)
            .ok_or_else(|| format!("No open trade for {}", ticker))?;
        
        trade.close(exit_price);
        
        // Update balance
        let pnl = trade.pnl.unwrap_or(0.0);
        self.current_balance += trade.position_size + pnl;
        self.total_pnl += pnl;
        
        // Update statistics
        self.total_trades += 1;
        if trade.is_profitable.unwrap_or(false) {
            self.winning_trades += 1;
            self.total_profit += pnl;
        } else {
            self.losing_trades += 1;
            self.total_loss += pnl.abs();
        }
        
        self.win_rate = if self.total_trades > 0 {
            (self.winning_trades as f64 / self.total_trades as f64) * 100.0
        } else {
            0.0
        };
        
        // Track max balance and drawdown
        if self.current_balance > self.max_balance {
            self.max_balance = self.current_balance;
        }
        
        let drawdown_pct = ((self.max_balance - self.current_balance) / self.max_balance) * 100.0;
        if drawdown_pct > self.max_drawdown {
            self.max_drawdown = drawdown_pct;
        }
        
        // Update per-model performance
        self.update_model_performance(&trade);
        
        self.closed_trades.push(trade);
        
        Ok(())
    }
    
    fn update_model_performance(&mut self, trade: &PaperTrade) {
        let perf = self.model_performance
            .entry(trade.model_id.clone())
            .or_insert_with(|| ModelPerformance {
                model_id: trade.model_id.clone(),
                trades: 0,
                wins: 0,
                losses: 0,
                win_rate: 0.0,
                total_pnl: 0.0,
                avg_pnl_per_trade: 0.0,
            });
        
        perf.trades += 1;
        if trade.is_profitable.unwrap_or(false) {
            perf.wins += 1;
        } else {
            perf.losses += 1;
        }
        
        perf.win_rate = (perf.wins as f64 / perf.trades as f64) * 100.0;
        perf.total_pnl += trade.pnl.unwrap_or(0.0);
        perf.avg_pnl_per_trade = perf.total_pnl / perf.trades as f64;
    }
    
    /// Get summary statistics
    pub fn get_summary(&self) -> String {
        format!(
            "📊 Portfolio Summary\n\
             Starting Capital: ${:.2}\n\
             Current Balance: ${:.2}\n\
             Total P&L: ${:.2} ({:.1}%)\n\
             \n\
             Total Trades: {}\n\
             Wins: {} | Losses: {}\n\
             Win Rate: {:.1}%\n\
             \n\
             Total Profit: ${:.2}\n\
             Total Loss: ${:.2}\n\
             Max Drawdown: {:.1}%\n\
             \n\
             Sharpe Ratio: {:.2}",
            self.starting_capital,
            self.current_balance,
            self.total_pnl,
            (self.total_pnl / self.starting_capital) * 100.0,
            self.total_trades,
            self.winning_trades,
            self.losing_trades,
            self.win_rate,
            self.total_profit,
            self.total_loss,
            self.max_drawdown,
            self.calculate_sharpe_ratio()
        )
    }
    
    /// Calculate Sharpe ratio (simplified)
    pub fn calculate_sharpe_ratio(&self) -> f64 {
        if self.closed_trades.is_empty() {
            return 0.0;
        }
        
        let pnl_values: Vec<f64> = self.closed_trades
            .iter()
            .filter_map(|t| t.pnl_percentage)
            .collect();
        
        if pnl_values.is_empty() {
            return 0.0;
        }
        
        let mean = pnl_values.iter().sum::<f64>() / pnl_values.len() as f64;
        let variance = pnl_values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / pnl_values.len() as f64;
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            0.0
        } else {
            mean / std_dev
        }
    }
}
