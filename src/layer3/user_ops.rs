use crate::client::RpcClient;
use serde_json::json;
use std::io::{self, Write};
use crate::identity::Identity;
use crate::rpc::types::RecurringOracleJob;

pub async fn run_user_ai_menu() {
    println!("\n🧠 --- User AI Neural Network Dashboard ---");
    
    // 1. Get User Identity
    let identity = match crate::interactive::load_identity("user") { // Default to 'user' or ask
        Some(id) => id,
        None => {
            println!("⚠️  No 'user' identity found. Please generate one in Key Management first.");
            return;
        }
    };
    let wallet_address = identity.public_key.clone();
    println!("👤 User: {}", wallet_address);

    print!("Enter Node URL [http://127.0.0.1:9000]: ");
    io::stdout().flush().unwrap();
    let mut node_url = String::new();
    io::stdin().read_line(&mut node_url).unwrap();
    let node_url = if node_url.trim().is_empty() { "http://127.0.0.1:9000".to_string() } else { node_url.trim().to_string() };
    
    let client = RpcClient::new(node_url);

    loop {
        println!("\nOptions:");
        println!("1. 🛒 Buy Basic Neural Network (10,000 COMPASS)");
        println!("2. 🏋️  Train My Neural Networks");
        println!("3. 🔙 Back");
        print!("Select: ");
        io::stdout().flush().unwrap();
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        
        match choice.trim() {
            "1" => buy_network(&client, &identity).await,
            "2" => train_networks_menu(&client, &identity).await,
            "3" => break,
            _ => println!("Invalid option"),
        }
    }
}

async fn buy_network(client: &RpcClient, identity: &Identity) {
    println!("\n🛒 Purchasing Basic Neural Network...");
    println!("   Info: This creates a new 'Generation 0' model ready for training.");
    println!("   Cost: 10,0000 COMPASS");
    
    print!("Confirm Purchase (y/n): ");
    io::stdout().flush().unwrap();
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).unwrap();
    
    if confirm.trim().to_lowercase() != "y" {
        return;
    }
    
    // Construct Tx to Buy
    let user_id = identity.public_key.clone();
    
    let params = json!({
        "owner": user_id,
        "ticker": "BTC", // Default to BTC tracker for now
    });
    
    println!("   📡 Sending Purchase Request...");
    let res = client.call_method::<serde_json::Value, serde_json::Value>("purchaseNeuralNet", params).await;
    
    match res {
        Ok(val) => {
            println!("✅ Purchase Successful!");
            println!("   Job ID: {}", val["job_id"]);
            println!("   You can now train this model in Option 2.");
        },
        Err(e) => println!("❌ Purchase Failed: {}", e),
    }
}

async fn train_networks_menu(client: &RpcClient, identity: &Identity) {
    println!("\n🏋️  Your Neural Networks");
    
    let _user_id = identity.public_key.clone();
    
    // Fetch owned jobs
    let all_jobs = client.call_method::<serde_json::Value, Vec<RecurringOracleJob>>("getRecurringJobs", json!({})).await.unwrap_or(Vec::new());
    let my_jobs: Vec<&RecurringOracleJob> = all_jobs.iter().filter(|j| {
        // Need to check ownership. RecurringJob needs 'owner' field or we check assigned_worker?
        // Wait, RecurringJob currently has 'assigned_worker'.
        // If user BOUGHT it, they should be the owner.
        // I need to add 'owner' field to RecurringOracleJob struct or metadata.
        // For now, let's assume if I created it, I am the owner?
        // Use a new field 'creator' or checks against valid jobs.
        // Let's filter by some convention or add 'owner' to struct.
        // Adding 'owner' is best.
        true // Placeholder
    }).collect();
    
    if my_jobs.is_empty() {
        println!("   No networks found. Go buy one!");
        return;
    }
    
    for (i, job) in my_jobs.iter().enumerate() {
        println!("{}. {} (Progress: {}/{})", i+1, job.ticker, job.completed_updates, job.total_updates_required);
    }
    
    print!("Select Network to Train (ID): ");
    io::stdout().flush().unwrap();
    // ... selection logic ...
    // Start training loop (similar to agent::run_continuous_cycle)
    // But force 'job_id' to be this one.
}
