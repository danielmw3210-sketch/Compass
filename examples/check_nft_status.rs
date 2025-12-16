use rust_compass::storage::Storage;

fn main() {
    let db_paths = vec![
        "compass_leader.db",
        "compass_db_leader",
        "compass_data.db",
        "compass_data",
        "compass.db",
    ];

    for path in db_paths {
        println!("\n📂 Checking Database: {}", path);
        if !std::path::Path::new(path).exists() {
            println!("   ⚠️ Path does not exist");
            continue;
        }

        let storage = match Storage::new(path) {
            Ok(s) => s,
            Err(e) => {
                println!("   ❌ Failed to open: {}", e);
                continue;
            }
        };

        println!("   🔍 Checking content...");

        // Check for Model NFTs
        let nfts = storage.get_all_nfts();
        
        if nfts.is_empty() {
            println!("   ❌ No NFTs found");
        } else {
            println!("   ✅ Found {} NFT(s):", nfts.len());
            for nft in &nfts {
                println!("      - {} ({}) Owner: {}", nft.token_id, nft.name, nft.current_owner);
            }
        }

        // Check for Epoch States
        if let Ok(states) = storage.get_all_epoch_states() {
            println!("\n=== Epoch States (Training Progress) ===");
            if states.is_empty() {
                println!("   ❌ No epoch states found");
            } else {
                for state in &states {
                    let acc = state.overall_accuracy() * 100.0;
                    let epochs = state.epochs_completed;
                    let min_acc = state.config.min_accuracy_to_mint * 100.0;
                    let min_epochs = state.config.mint_at_epoch.unwrap_or(10);
                    
                    println!("\n   📊 Model: {} ({})", state.ticker, state.model_id);
                    println!("      - Progress: {}/{} Epochs", epochs, min_epochs);
                    println!("      - Accuracy: {:.2}% (Target: {:.2}%)", acc, min_acc);
                    
                    if state.nft_minted {
                        println!("      - Status: ✅ ALREADY MINTED");
                    } else if state.should_mint() {
                         println!("      - Status: 🟢 READY TO MINT (Should be minting now!)");
                    } else {
                         println!("      - Status: ⏳ TRAINING (Not enough epochs/accuracy)");
                    }
                }
            }
        }
    }
}
