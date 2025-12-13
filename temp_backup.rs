        "2" => {
            println!("Wiping 'compass_db_verifier'...");
             let _ = std::fs::remove_dir_all("compass_db_verifier");
            println!("Done.");
        },
        "3" => crate::genesis::generate_admin_config(),
        _ => {},
    }
}

async fn run_ai_worker() {
    println!("\n--- Layer 3 AI Worker ---");
    println!("This worker will contribute GPU/CPU compute to AI inference jobs.");
    
    // Get configuration
    print!("Enter node RPC URL [http://127.0.0.1:9000]: ");
    io::stdout().flush().unwrap();
    let mut node_url = String::new();
    io::stdin().read_line(&mut node_url).unwrap();
    let node_url = node_url.trim();
    let node_url = if node_url.is_empty() {
        "http://127.0.0.1:9000".to_string()
    } else {
        node_url.to_string()
    };
    
    print!("Enter model ID [gpt-4o-mini]: ");
    io::stdout().flush().unwrap();
    let mut model_id = String::new();
    io::stdin().read_line(&mut model_id).unwrap();
    let model_id = model_id.trim();
    let model_id = if model_id.is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        model_id.to_string()
    };
    
    println!("\n🚀 Starting AI Worker...");
    println!("   Node: {}", node_url);
    println!("   Model: {}", model_id);
    println!("\nWorker will:");
    println!("  1. Poll for pending AI compute jobs");
    println!("  2. Compute hash power proof (anti-spam)");
    println!("  3. Execute AI inference on your hardware");
    println!("  4. Submit results and earn rewards");
    println!("\nPress Ctrl+C to stop.\n");
    
    // Start worker
    let worker = crate::client::AiWorker::new(node_url, model_id);
    worker.start().await;
}
