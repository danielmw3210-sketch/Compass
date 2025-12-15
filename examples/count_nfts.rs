// Quick utility to count NFTs in the database
use rust_compass::storage::Storage;

fn main() {
    let storage = Storage::new("compass_db_leader").expect("Failed to open database");
    
    println!("🔍 Scanning database for Model NFTs...\n");
    
    // Try to count NFTs (keys starting with "model_nft:")
    let mut count = 0;
    let mut found_keys = Vec::new();
    
    // Use the internal sled database to scan
    // Since Storage doesn't expose iter(), we'll try querying known patterns
    // Or we can check if specific NFT IDs exist
    
    // Check for some known NFT patterns based on the code
    for i in 1..=1000 {
        let test_id = format!("oracle-bridge-nn-v{}", i);
        if let Ok(Some(_)) = storage.get_model_nft(&test_id) {
            count += 1;
            found_keys.push(test_id);
        }
    }
    
    // Also check for other common patterns
    for prefix in ["NN_TRAIN_", "GPT-", "BERT-", "model_"] {
        for i in 1..=1000 {
            let test_id = format!("{}{}", prefix, i);
            if let Ok(Some(_)) = storage.get_model_nft(&test_id) {
                if !found_keys.contains(&test_id) {
                    count += 1;
                    found_keys.push(test_id.clone());
                }
            }
        }
    }
    
    println!("\n📊 RESULTS:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if count > 0 {
        println!("✅ Total Model NFTs found: {}\n", count);
        println!("NFT IDs:");
        for (idx, key) in found_keys.iter().enumerate().take(20) {
            println!("  {}. {}", idx + 1, key);
        }
        if found_keys.len() > 20 {
            println!("  ... and {} more", found_keys.len() - 20);
        }
    } else {
        println!("⚠️  No Model NFTs found in database");
        println!("   This is expected if you haven't minted any AI models yet.");
    }
}
