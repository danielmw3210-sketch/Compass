// Reset nft_minted flags for testing
use rust_compass::storage::Storage;
use rust_compass::layer3::price_oracle::ModelEpochState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Resetting nft_minted flags...\n");
    
    // Use the same path as the node
    let storage = Storage::new("./data/primary")?;
    
    let tickers = vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"];
    let model_ids = vec!["signal_btc_v2", "signal_eth_v2", "signal_sol_v2", "signal_ltc_v2"];
    
    for (ticker, model_id) in tickers.iter().zip(model_ids.iter()) {
        if let Some(mut epoch_state) = storage.get_epoch_state(ticker, model_id)? {
            if epoch_state.nft_minted {
                println!("📝 Resetting {}:{}", ticker, model_id);
                println!("   Was: Epoch {}, {:.1}% accuracy, nft_minted={}", 
                    epoch_state.epochs_completed,
                    epoch_state.overall_accuracy() * 100.0,
                    epoch_state.nft_minted);
                
                epoch_state.nft_minted = false;
                epoch_state.nft_token_id = None;
                
                storage.save_epoch_state(&epoch_state)?;
                
                println!("   ✅ Reset complete - ready to mint!\n");
            } else {
                println!("ℹ️  {}:{} - nft_minted already false\n", ticker, model_id);
            }
        } else {
            println!("⚠️  No epoch state found for {}:{}\n", ticker, model_id);
        }
    }
    
    println!("✅ All flags reset! Refresh your GUI Portfolio page.");
    Ok(())
}
