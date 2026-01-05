// v2.0 Phase 5: Model Marketplace
// P2P trading system for AI model NFTs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelListing {
    pub listing_id: String,
    pub model_id: String,
    pub seller_account: String,
    pub price_compass: u64,
    pub listed_at: u64,
    pub status: ListingStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelMarketplace {
    pub listings: HashMap<String, ModelListing>,
    pub sales_history: Vec<SaleRecord>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaleRecord {
    pub model_id: String,
    pub seller_account: String,
    pub buyer_account: String,
    pub price_compass: u64,
    pub timestamp: u64,
}

impl ModelMarketplace {
    pub fn new() -> Self {
        Self {
            listings: HashMap::new(),
            sales_history: Vec::new(),
        }
    }
    
    /// Create a new marketplace listing
    pub fn list_model(
        &mut self,
        model_id: String,
        seller_account: String,
        price_compass: u64,
    ) -> Result<String, String> {
        // Generate listing ID
        let listing_id = format!("listing_{}_{}", model_id, chrono::Utc::now().timestamp());
        
        // Check if already listed
        if self.listings.values().any(|l| l.model_id == model_id && l.status == ListingStatus::Active) {
            return Err("Model already listed".to_string());
        }
        
        let listing = ModelListing {
            listing_id: listing_id.clone(),
            model_id,
            seller_account,
            price_compass,
            listed_at: chrono::Utc::now().timestamp() as u64,
            status: ListingStatus::Active,
        };
        
        self.listings.insert(listing_id.clone(), listing);
        Ok(listing_id)
    }
    
    /// Purchase a listed model
    pub fn buy_model(
        &mut self,
        listing_id: &str,
        buyer_account: String,
    ) -> Result<(String, String, u64), String> {
        let listing = self.listings.get_mut(listing_id)
            .ok_or("Listing not found")?;
        
        if listing.status != ListingStatus::Active {
            return Err("Listing not active".to_string());
        }
        
        if listing.seller_account == buyer_account {
            return Err("Cannot buy your own model".to_string());
        }
        
        // Mark as sold
        listing.status = ListingStatus::Sold;
        
        // Record sale
        self.sales_history.push(SaleRecord {
            model_id: listing.model_id.clone(),
            seller_account: listing.seller_account.clone(),
            buyer_account: buyer_account.clone(),
            price_compass: listing.price_compass,
            timestamp: chrono::Utc::now().timestamp() as u64,
        });
        
        Ok((listing.model_id.clone(), listing.seller_account.clone(), listing.price_compass))
    }
    
    /// Cancel an active listing
    pub fn cancel_listing(
        &mut self,
        listing_id: &str,
        seller_account: &str,
    ) -> Result<(), String> {
        let listing = self.listings.get_mut(listing_id)
            .ok_or("Listing not found")?;
        
        if listing.seller_account != seller_account {
            return Err("Not the seller".to_string());
        }
        
        if listing.status != ListingStatus::Active {
            return Err("Listing not active".to_string());
        }
        
        listing.status = ListingStatus::Cancelled;
        Ok(())
    }
    
    /// Get all active listings
    pub fn get_active_listings(&self) -> Vec<&ModelListing> {
        self.listings.values()
            .filter(|l| l.status == ListingStatus::Active)
            .collect()
    }
    
    /// Get sales history for a model
    pub fn get_model_history(&self, model_id: &str) -> Vec<&SaleRecord> {
        self.sales_history.iter()
            .filter(|s| s.model_id == model_id)
            .collect()
    }
}
