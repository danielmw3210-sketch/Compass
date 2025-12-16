// Clear all NFTs from the database
use rust_compass::storage::Storage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args().nth(1).unwrap_or_else(|| "compass_db".to_string());
    
    println!("🗑️  Opening database: {}", db_path);
    let storage = Storage::new(&db_path)?;
    
    // Scan for all nft: keys
    let mut deleted = 0;
    for result in storage.db.scan_prefix(b"nft:") {
        let (key, _) = result?;
        let key_str = String::from_utf8_lossy(&key);
        println!("   Deleting: {}", key_str);
        storage.db.remove(&key)?;
        deleted += 1;
    }
    
    storage.db.flush()?;
    println!("✅ Deleted {} NFTs from database", deleted);
    
    Ok(())
}
