/// Mint an NFT from a trained model's epoch state
/// FIXED: Now submits transaction through Gulf Stream instead of direct storage write
pub async fn handle_mint_model_nft(
    state: RpcState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use crate::layer3::model_nft::{ModelNFT, ModelStats};
    use crate::layer3::price_oracle::ModelEpochState;
    
    #[derive(serde::Deserialize)]
    struct Params {
        ticker: String,
        model_id: String,
    }
    
    let req: Params = serde_json::from_value(params)
        .map_err(|e| RpcError { code: -32602, message: format!("Invalid params: {}", e) })?;
    
    let chain = safe_lock(&state.chain)?;
    
    // Load current epoch state
    let epoch_state = chain.storage.get_epoch_state(&req.ticker, &req.model_id)
        .map_err(|e| RpcError { code: -32603, message: format!("DB error: {}", e) })?
        .ok_or_else(|| RpcError { 
            code: -32603, 
            message: format!("No epoch state found for {}:{}", req.ticker, req.model_id) 
        })?;
    
    // Verify minting conditions
    let epochs_completed = epoch_state.epochs_completed;
    let accuracy = epoch_state.overall_accuracy();
    let min_epochs = epoch_state.config.mint_at_epoch.unwrap_or(10);
    let min_accuracy = epoch_state.config.min_accuracy_to_mint;
    
    if epochs_completed < min_epochs || accuracy < min_accuracy {
        return Err(RpcError {
            code: -32603,
            message: format!(
                "Cannot mint: Model {}:{} has not met requirements (need {}+ epochs with {:.0}%+ accuracy, currently {} epochs at {:.1}%)",
                req.ticker,
                req.model_id,
                min_epochs,
                min_accuracy * 100.0,
                epochs_completed,
                accuracy * 100.0
            )
        });
    }
    
    // Get generation number
    let existing_nfts = chain.storage.get_all_nfts();
    let generation = existing_nfts.iter()
        .filter(|nft| nft.name.contains(&req.ticker) && nft.weights_uri.contains(&req.model_id))
        .count() as u32;
    
    // Create ModelStats from epoch state
    let stats = ModelStats {
        accuracy: epoch_state.overall_accuracy(),
        win_rate: epoch_state.overall_accuracy(),
        total_predictions: epoch_state.total_predictions as usize,
        profitable_predictions: epoch_state.total_correct as usize,
        total_profit: 0,
        training_samples: epoch_state.total_predictions as usize,
        training_epochs: epoch_state.epochs_completed as usize,
        final_loss: 0.0,
        training_duration: 0,
        data_hash: format!("oracle-{}-{}-gen{}", req.ticker, req.model_id, generation),
    };
    
    // Create NFT token ID with generation
    let token_id = format!("NFT_{}_{}_{}", req.model_id, req.ticker, generation);
    
    // Create NFT to extract metadata
    let mut nft = ModelNFT::from_job(
        &token_id,
        &req.ticker,
        "admin".to_string(),
        &stats,
    );
    
    // Set generation and parent info
    nft.generation = generation;
    if generation > 0 {
        if let Some(parent) = existing_nfts.iter()
            .filter(|n| n.name.contains(&req.ticker) && n.generation == generation - 1)
            .next() {
            nft.parent_models = vec![parent.token_id.clone()];
        }
    }
    
    let nft_name = nft.name.clone();
    let nft_description = nft.description.clone();
    let nft_accuracy = nft.accuracy;
    let nft_estimated_value = nft.estimated_value();
    
    drop(chain); // Release lock before transaction submission
    
    // === FIX: Submit through Gulf Stream instead of direct write ===
    let mint_params = MintModelNFTParams {
        creator: "admin".to_string(),
        model_id: token_id.clone(),
        name: nft_name.clone(),
        description: nft_description,
        signature: "mint_via_rpc".to_string(),
    };
    
    let payload = crate::network::TransactionPayload::MintModelNFT(mint_params);
    let raw_tx = safe_serialize(&payload)?;
    let tx_hash = sha2::Sha256::digest(&raw_tx).to_vec();
    
    // Submit to Gulf Stream
    {
        let mut gs = safe_lock(&state.gulf_stream)?;
        let added = gs.add_transaction(tx_hash.clone(), raw_tx.clone(), 0);
        if !added {
            return Err(RpcError {
                code: -32603,
                message: "Transaction rejected by Gulf Stream".to_string(),
            });
        }
    }
    
    // Broadcast to network
    let msg = crate::network::NetMessage::SubmitTx(payload);
    let _ = state.cmd_tx.send(crate::network::NetworkCommand::Broadcast(msg)).await;
    
    info!("🎨 NFT Mint Transaction Submitted via Gulf Stream: {} (Gen {}, {:.1}% accuracy)", 
        token_id, generation, nft_accuracy * 100.0);
    
    // Reset epoch state for next generation
    {
        let chain = safe_lock(&state.chain)?;
        let new_epoch_state = ModelEpochState::new(
            &req.model_id,
            &req.ticker,
            epoch_state.config.clone()
        );
        
        chain.storage.save_epoch_state(&new_epoch_state)
            .map_err(|e| RpcError { code: -32603, message: format!("Failed to reset epoch state: {}", e) })?;
        
        info!("♻️ Reset epoch state for next generation training");
    }
    
    Ok(serde_json::json!({
        "success": true,
        "token_id": token_id,
        "name": nft_name,
        "generation": generation,
        "accuracy": nft_accuracy,
        "epochs_trained": stats.training_epochs,
        "estimated_value": nft_estimated_value,
        "tx_hash": hex::encode(tx_hash),
        "message": format!("✅ NFT mint transaction submitted via Gulf Stream! Token will appear once processed. Generation {} training reset.", generation + 1),
        "note": "Transaction is in mempool and will be processed by the transaction handler"
    }))
}
