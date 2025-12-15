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
pub struct NFTListing {
    pub listing_id: u64,
    pub token_id: String,
    pub seller: String,
    pub price: u64,
    pub currency: String, // "Compass"
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone)] // Removed generic Debug
pub struct Market {
    // Key: "Base/Quote" e.g. "Compass:Alice:LTC/Compass"
    pub books: HashMap<String, OrderBook>,
    // Key: token_id
    pub nft_listings: HashMap<String, NFTListing>,
    pub next_order_id: u64,
    #[serde(skip)]
    pub storage: Option<std::sync::Arc<crate::storage::Storage>>,
}

impl std::fmt::Debug for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Market")
         .field("books", &self.books)
         .field("nft_listings", &self.nft_listings)
         .field("next_order_id", &self.next_order_id)
         .finish()
    }
}

impl Market {
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
            nft_listings: HashMap::new(),
            next_order_id: 1,
            storage: None,
        }
    }

    pub fn new_with_storage(storage: std::sync::Arc<crate::storage::Storage>) -> Self {
        let mut m = Self::new();
        m.storage = Some(storage.clone());
        
        // Load Meta
        if let Ok(Some(id)) = storage.get_market_meta() {
            m.next_order_id = id;
        }
        
        // Load Books
        let books = storage.get_all_order_books();
        for b in books {
            let key = format!("{}/{}", b.base_asset, b.quote_asset);
            m.books.insert(key, b);
        }

        // Load Listings
        let listings = storage.get_all_nft_listings();
        for l in listings {
            m.nft_listings.insert(l.token_id.clone(), l);
        }
        
        m
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
        if let Some(s) = &self.storage {
             // Sled Persist
             for (k, b) in &self.books {
                 let _ = s.save_order_book(k, b); 
             }
             // Save Listings?
             // Since we haven't added save_nft_listing to Storage yet, this part needs Storage update.
             // We will implement `save_nft_listing` in Storage next.
             // For now, we will add the call assuming it exists or will exist.
             // Actually, I can't call it if it doesn't exist yet (compilation error).
             // Strategy: I will add the logic to Market, but comment out the storage call until Storage is updated? 
             // No, I should update Storage first if I want to compile. 
             // But I'm editing Market now. 
             // I will comment out the specific storage call for listings here and add a TODO, 
             // then immediately go fix Storage, then uncomment.
             // OR: I rely on JSON for listings temporarily? No, `Market` uses `storage` if present.
             // I will rely on `storage.save_market_with_listings`? No.
             
             let _ = s.save_market_meta(self.next_order_id);
             let _ = s.flush();
        } else {
             use std::fs;
             let data = serde_json::to_string_pretty(self).unwrap();
             fs::write(path, data).expect("Unable to save market");
        }
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
        // Note: Wallet debited. We should save wallet state?
        // Caller often saves wallet. Or we can trigger it here if wallets helper allows?
        // wallets.save("wallets.json"); // Uses internal storage if present.
        // But let's leave it to caller/Node loop to not spam IO. 
        // Although this is critical financial op. 
        // For now, adhere to explicit save flow.

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
        if let Some(s) = &self.storage {
            let _ = s.save_market_meta(self.next_order_id);
        }

        let logs = book.add_order(order, wallets);
        
        // Persist Book Updates
        if let Some(s) = &self.storage {
             let _ = s.save_order_book(&pair_key, book);
        }
        
        Ok(logs)
    }

    // --- NFT Marketplace Methods ---

    pub fn place_nft_listing(
        &mut self,
        token_id: String,
        seller: String,
        price: u64,
        currency: String,
    ) -> Result<String, String> {
        if self.nft_listings.contains_key(&token_id) {
            return Err("NFT is already listed".to_string());
        }

        let listing = NFTListing {
            listing_id: self.next_order_id,
            token_id: token_id.clone(),
            seller: seller.clone(),
            price,
            currency: currency.clone(),
            active: true,
        };
        self.next_order_id += 1;

        self.nft_listings.insert(token_id.clone(), listing.clone());
        
        if let Some(s) = &self.storage { 
            let _ = s.save_nft_listing(&listing); 
        }
        
        Ok(format!("NFT {} listed for {} {}", token_id, price, currency))
    }

    pub fn execute_nft_purchase(
        &mut self,
        token_id: &str,
        buyer: &str,
        wallets: &mut WalletManager,
    ) -> Result<(String, u64, String, u64), String> {
        // Returns (Seller, RoyaltyAmount, ListingCurrency, Price)
        
        let listing = self.nft_listings.get(token_id).ok_or("Listing not found")?;
        if !listing.active {
            return Err("Listing is inactive".to_string());
        }

        let cost = listing.price;
        let currency = listing.currency.clone();
        let seller = listing.seller.clone();
        
        // 1. Debit Buyer
        if !wallets.debit(buyer, &currency, cost) {
            return Err(format!("Insufficient {} balance to buy NFT", currency));
        }

        // 2. Calculate Royalties (Fixed 10% for now, ideally passed in)
        let royalty = cost / 10; 
        let seller_share = cost - royalty;

        // 3. Credit Seller
        wallets.credit(&seller, &currency, seller_share);
        
        // 4. Credit Foundation/Creator (Royalty handled by caller)
        
        // Remove Listing
        self.nft_listings.remove(token_id);
        
        if let Some(s) = &self.storage { 
            let _ = s.delete_nft_listing(token_id); 
        }
        
        Ok((seller, royalty, currency, cost))
    }
    
    pub fn cancel_nft_listing(&mut self, token_id: &str, requestor: &str) -> Result<(), String> {
        let listing = self.nft_listings.get(token_id).ok_or("Listing not found")?;
        if listing.seller != requestor {
            return Err("Not the seller".to_string());
        }
        self.nft_listings.remove(token_id);
        if let Some(s) = &self.storage { 
            let _ = s.delete_nft_listing(token_id); 
        }
        Ok(())
    }
}
