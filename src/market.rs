use crate::vault::VaultManager;
use crate::wallet::WalletManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub user: String,
    pub pair_base: String,  // e.g. "Compass:Alice:LTC"
    pub pair_quote: String, // e.g. "Compass" (Genesis) or "SOL"
    pub side: OrderSide,
    pub price: u64,  // Quote units per 1 Base unit
    pub amount: u64, // Base units
    pub amount_filled: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderBook {
    pub base_asset: String,
    pub quote_asset: String,
    pub bids: Vec<Order>, // Buy orders (Sorted High to Low)
    pub asks: Vec<Order>, // Sell orders (Sorted Low to High)
}

impl OrderBook {
    pub fn new(base: &str, quote: &str) -> Self {
        Self {
            base_asset: base.to_string(),
            quote_asset: quote.to_string(),
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Add order and attempt matching
    pub fn add_order(&mut self, mut order: Order, wallets: &mut WalletManager) -> Vec<String> {
        let mut logs = Vec::new();
        logs.push(format!(
            "Order Placed: {:?} {} {} @ {}",
            order.side, order.amount, order.pair_base, order.price
        ));

        // Matching Engine
        if order.side == OrderSide::Buy {
            // Match against Asks (Lowest Sellers first)
            // Sort Asks Ascending Price
            self.asks.sort_by_key(|o| o.price);

            let mut i = 0;
            while i < self.asks.len() && order.amount_filled < order.amount {
                let ask = &mut self.asks[i];
                if ask.price <= order.price {
                    // Match!
                    let fill_amt = std::cmp::min(
                        order.amount - order.amount_filled,
                        ask.amount - ask.amount_filled,
                    );
                    let cost = fill_amt * ask.price;

                    // Execute Swap in Wallets
                    // Buyer (order.user) gets Base, pays Quote
                    // Seller (ask.user) pays Base, gets Quote
                    wallets.credit(&order.user, &self.base_asset, fill_amt);
                    // wallets.debit(&order.user, &self.quote_asset, cost); // Already escrowed? For simplicity, debit now or assume escrow.
                    // Let's assume user must have balance NOW. logic is slightly complex for atomic.
                    // For prototype: we assume "Hold" logic is external or implicit.
                    // To be safe: we actually transfer NOW.
                    wallets.credit(&ask.user, &self.quote_asset, cost);

                    // Note: Sellers base was escrowed or debited?
                    // Buyers quote was escrowed or debited?
                    // Implementation Detail: In `place_order`, we should DEBIT the active asset immediately.
                    // So here we only CREDIT the counterparty.

                    logs.push(format!(
                        "MATCH: Sold {} {} @ {}",
                        fill_amt, self.base_asset, ask.price
                    ));

                    order.amount_filled += fill_amt;
                    ask.amount_filled += fill_amt;

                    // If ask filled, remove later? (Vector remove is O(n), we'll sweep later or remove now)
                    if ask.amount_filled >= ask.amount {
                        // Remove ask
                        self.asks.remove(i);
                        // Don't increment i, shifted
                    } else {
                        i += 1;
                    }
                } else {
                    break; // No more cheaper sellers
                }
            }

            // If remaining, add to Bids
            if order.amount_filled < order.amount {
                self.bids.push(order);
            }
        } else {
            // Sell Side: Match against Bids (Highest Buyers first)
            self.bids.sort_by(|a, b| b.price.cmp(&a.price));

            let mut i = 0;
            while i < self.bids.len() && order.amount_filled < order.amount {
                let bid = &mut self.bids[i];
                if bid.price >= order.price {
                    // Match!
                    let fill_amt = std::cmp::min(
                        order.amount - order.amount_filled,
                        bid.amount - bid.amount_filled,
                    );
                    let cost = fill_amt * bid.price;

                    // Seller (order.user) gets Quote
                    wallets.credit(&order.user, &self.quote_asset, cost);
                    // Buyer (bid.user) gets Base
                    wallets.credit(&bid.user, &self.base_asset, fill_amt);

                    logs.push(format!(
                        "MATCH: Bought {} {} @ {}",
                        fill_amt, self.base_asset, bid.price
                    ));

                    order.amount_filled += fill_amt;
                    bid.amount_filled += fill_amt;

                    if bid.amount_filled >= bid.amount {
                        self.bids.remove(i);
                    } else {
                        i += 1;
                    }
                } else {
                    break;
                }
            }

            if order.amount_filled < order.amount {
                self.asks.push(order);
            }
        }

        logs
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    // Key: "Base/Quote" e.g. "Compass:Alice:LTC/Compass"
    pub books: HashMap<String, OrderBook>,
    pub next_order_id: u64,
}

impl Market {
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
            next_order_id: 1,
        }
    }

    pub fn load(path: &str) -> Self {
        use std::fs;
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_else(|_| Self::new())
        } else {
            Self::new()
        }
    }

    pub fn save(&self, path: &str) {
        use std::fs;
        let data = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, data).expect("Unable to save market");
    }

    pub fn place_order(
        &mut self,
        user: &str,
        side: OrderSide,
        base: &str,
        quote: &str,
        amount: u64,
        price: u64,
        wallets: &mut WalletManager,
    ) -> Result<Vec<String>, String> {
        let pair_key = format!("{}/{}", base, quote);

        // 1. Check Balance / Escrow
        // If Buying: Need Quote Asset (Price * Amount)
        // If Selling: Need Base Asset (Amount)
        let cost = if side == OrderSide::Buy {
            price * amount
        } else {
            0
        };
        let req_asset = if side == OrderSide::Buy { quote } else { base };
        let req_amt = if side == OrderSide::Buy { cost } else { amount };

        if !wallets.debit(user, req_asset, req_amt) {
            return Err(format!("Insufficient {} balance.", req_asset));
        }

        let book = self
            .books
            .entry(pair_key.clone())
            .or_insert_with(|| OrderBook::new(base, quote));

        let order = Order {
            id: self.next_order_id,
            user: user.to_string(),
            pair_base: base.to_string(),
            pair_quote: quote.to_string(),
            side,
            price,
            amount,
            amount_filled: 0,
            timestamp: 0, // TODO: Time
        };
        self.next_order_id += 1;

        let logs = book.add_order(order, wallets);
        Ok(logs)
    }
}
