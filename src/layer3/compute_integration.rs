// v2.0 Phase 4: COMPUTE Token Integration Helper
// Extension methods for integrating COMPUTE rewards with BalanceStore

use crate::account::balance::BalanceStore;
use std::sync::{Arc, Mutex};

/// COMPUTE to COMPASS conversion rate (100:1)
pub const COMPUTE_TO_COMPASS_RATIO: u64 = 100;

/// Mint COMPUTE tokens to worker's account
pub fn mint_compute_reward(
    balance_store: &Arc<Mutex<BalanceStore>>,
    worker_account: &str,
    compute_amount: u64,
) -> Result<(), String> {
    let mut bal_store = balance_store.lock()
        .map_err(|e| format!("Failed to lock BalanceStore: {}", e))?;
    
    bal_store.credit(
        &worker_account.to_string(),
        &"COMPUTE".to_string(),
        compute_amount,
    ).map_err(|e| format!("{:?}", e))?;
    
    tracing::info!("💰 Minted {} COMPUTE to {}", compute_amount, worker_account);
    Ok(())
}

/// Convert COMPUTE tokens to COMPASS  
/// Burns COMPUTE and mints COMPASS at 100:1 ratio
pub fn convert_compute_to_compass(
    balance_store: &Arc<Mutex<BalanceStore>>,
    account: &str,
    compute_amount: u64,
) -> Result<u64, String> {
    if compute_amount < COMPUTE_TO_COMPASS_RATIO {
        return Err(format!(
            "Minimum {} COMPUTE required for conversion",
            COMPUTE_TO_COMPASS_RATIO
        ));
    }
    
    let compass_amount = compute_amount / COMPUTE_TO_COMPASS_RATIO;
    
    let mut bal_store = balance_store.lock()
        .map_err(|e| format!("Failed to lock BalanceStore: {}", e))?;
    
    // Check balance
    let current_balance = bal_store.get_balance(&account.to_string(), &"COMPUTE".to_string());
    if current_balance < compute_amount {
        return Err(format!(
            "Insufficient COMPUTE balance. Have: {}, Need: {}",
            current_balance, compute_amount
        ));
    }
    
    // Burn COMPUTE
    bal_store.debit(
        &account.to_string(),
        &"COMPUTE".to_string(),
        compute_amount,
    ).map_err(|e| format!("{:?}", e))?;
    
    // Mint COMPASS
    bal_store.credit(
        &account.to_string(),
        &"COMPASS".to_string(),
        compass_amount,
    ).map_err(|e| format!("{:?}", e))?;
    
    tracing::info!(
        "🔄 Converted {} COMPUTE → {} COMPASS for {}",
        compute_amount, compass_amount, account
    );
    
    Ok(compass_amount)
}

/// Get multi-layer balance summary for an account
pub fn get_account_balances(
    balance_store: &Arc<Mutex<BalanceStore>>,
    account: &str,
) -> Result<serde_json::Value, String> {
    let bal_store = balance_store.lock()
        .map_err(|e| format!("Failed to lock BalanceStore: {}", e))?;
    
    // Get balances for all known assets
    let compass_balance = bal_store.get_balance(&account.to_string(), &"COMPASS".to_string());
    let compute_balance = bal_store.get_balance(&account.to_string(), &"COMPUTE".to_string());
    
    // TODO: Query Layer 2 assets (cBTC, cLTC, cSOL) from VaultManager
    
    Ok(serde_json::json!({
        "account": account,
        "layer1": {
            "COMPASS": compass_balance
        },
        "layer2": {
            // Will be populated by vault queries
        },
        "layer3": {
            "COMPUTE": compute_balance
        }
    }))
}
