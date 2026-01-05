use rust_compass::storage::Storage;

fn main() {
    let db_paths = vec![
        "compass_data.db",
        "compass_db_leader",
    ];

    for path in db_paths {
        println!("\n📂 Checking Database: {}", path);
        if !std::path::Path::new(path).exists() { continue; }

        let storage = match Storage::new(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Check for OLD prefix
        println!("   🔍 Scanning for 'nft:' prefix (Legacy)...");
        let mut found = 0;
        let prefix_old = b"nft:";
        let iter_old = storage.db.scan_prefix(prefix_old);
        for item in iter_old {
             match item {
                 Ok((key, _)) => {
                     let k_str = String::from_utf8_lossy(&key);
                     println!("      Found Legacy NFT: {}", k_str);
                     found += 1;
                 },
                 Err(_) => {}
             }
        }
        if found == 0 { println!("      No legacy NFTs found."); }
        
        // Check for NEW prefix
        println!("   🔍 Scanning for 'model_nft:' prefix (New)...");
        let mut found_new = 0;
        let prefix_new = b"model_nft:";
        let iter_new = storage.db.scan_prefix(prefix_new);
        for item in iter_new {
             match item {
                 Ok((key, _)) => {
                     let k_str = String::from_utf8_lossy(&key);
                     println!("      Found Modern NFT: {}", k_str);
                     found_new += 1;
                 },
                 Err(_) => {}
             }
        }
        if found_new == 0 { println!("      No modern NFTs found."); }
    }
}
