/// Handle submitOraclePrice (v2.0)
/// Allows registered oracles to submit price feeds
async fn handle_submit_oracle_price(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::oracle::registry::OraclePriceSubmission;
    
    let submission: OraclePriceSubmission = serde_json::from_value(params)
        .map_err(|e| RpcError {
            code: -32602,
            message: format!("Invalid params: {}", e),
        })?;
    
    // 1. Verify oracle is registered and active
    {
        let oracle_reg = safe_lock(&state.chain.lock().unwrap().oracle_registry)?;
        
        if !oracle_reg.is_oracle(&submission.oracle_account) {
            return Err(RpcError {
                code: -32001,
                message: "Oracle not registered or inactive".to_string(),
            });
        }
    }
    
    // 2. TODO: Verify signature (for now, accept if registered)
    
    // 3. Store price in vault manager (existing system)
    {
        let mut chain = safe_lock(&state.chain)?;
        
        // Update oracle price in vault manager
        chain.vault_manager.oracle_prices.insert(
            submission.ticker.clone(),
            (submission.price.into(), submission.timestamp),
        );
        
        // Save vault state
        chain.vault_manager.save("").map_err(|e| RpcError {
            code: -32603,
            message: format!("Failed to save oracle price: {}", e),
        })?;
    }
    
    info!("🔮 Oracle Price Update: {} = ${:.2} (from {})", 
        submission.ticker, submission.price, submission.oracle_account);
    
    Ok(serde_json::json!({
        "status": "accepted",
        "ticker": submission.ticker,
        "price": submission.price,
        "oracle": submission.oracle_account
    }))
}
