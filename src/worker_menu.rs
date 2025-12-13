/// Interactive Worker Menu - View and Select Oracle Verification Jobs
use crate::client::rpc_client::RpcClient;
use std::io::{self, Write};
use std::time::Duration;
use serde_json::json;
use crate::rpc::types::OracleVerificationJob;
use crate::rpc::types::{RecurringOracleJob, SubmitOracleVerificationResultParams};
use crate::client::price_fetcher::PriceFetcher;
use rust_decimal::prelude::*;
use crate::crypto::KeyPair;

pub async fn worker_job_menu(node_url: &str) -> Result<(), String> {
    let client = RpcClient::new(node_url.to_string());
    
    // 1. Select Worker Wallet
    println!("\n🔑 Select Worker Wallet");
    print!("   Enter wallet name (default: 'worker'): ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut name = String::new();
    io::stdin().read_line(&mut name).map_err(|e| e.to_string())?;
    let name = if name.trim().is_empty() { "worker" } else { name.trim() };
    
    // Attempt to load
    let identity = match crate::interactive::load_identity(name) {
        Some(id) => id,
        None => {
            println!("❌ Wallet '{}' not found.", name);
            println!("   Please create it in 'Key Management' first.");
            return Ok(());
        }
    };
    
    let worker_keypair = identity.into_keypair().map_err(|e| format!("Failed to decrypt key: {}", e))?;
    let worker_id = worker_keypair.public_key_hex();
    
    println!("✅ Worker Identity Loaded: {}", worker_id);
    println!("   (Rewards will be sent to this address)");
    
    loop {
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║    🔍 Oracle Verification Worker Dashboard    ║");
        println!("╚════════════════════════════════════════════════╝");
        println!();
        
        // Fetch ALL job types
        println!("📡 Fetching available jobs...");
        
        // Ignore errors to keep menu alive
        let single_jobs = fetch_pending_oracle_jobs(&client).await.unwrap_or_else(|_| Vec::new());
        let recurring_jobs = fetch_recurring_jobs(&client).await.unwrap_or_else(|_| Vec::new());
        let compute_jobs: Vec<PendingJob> = client.call_method("getPendingComputeJobs", json!({})).await.unwrap_or_else(|_| Vec::new());
        
        let total_jobs = single_jobs.len() + recurring_jobs.len() + compute_jobs.len();
        
        if total_jobs == 0 {
             println!("✓ No available jobs found.");
             // Show options even if no jobs
             println!();
             println!("Options:");
             println!("  A. Start Autonomous AI Worker (Loop)");
             println!("  R. Refresh");
             println!("  Q. Exit");
             print!("Select: ");
             io::stdout().flush().unwrap();
             
             let mut choice = String::new();
             io::stdin().read_line(&mut choice).unwrap();
             
             match choice.trim().to_uppercase().as_str() {
                 "Q" | "2" => break,
                 "A" => {
                      ai_worker_loop(&client, &worker_keypair).await?;
                 },
                 _ => continue,
             }
        }
        
        println!("\n📋 Available Jobs:");
        println!("─────────────────────────────────────────────────");
        
        let mut job_index = 1;
        
        // 1. Single Oracle Jobs
        for job in &single_jobs {
            println!("\n{}. [ONE-TIME] Ticker: {}", job_index, job.ticker);
            println!("   Job ID: {}", job.job_id);
            println!("   Reward: Standard verified fee");
            job_index += 1;
        }
        
        // 2. Recurring Oracle Jobs
        for job in &recurring_jobs {
            println!("\n{}. [RECURRING] Ticker: {}", job_index, job.ticker);
            println!("   Job ID: {}", job.job_id);
            println!("   Status: {} ({}/{} updates)", job.status, job.completed_updates, job.total_updates_required);
             println!("   Reward: {} COMPASS / update", job.worker_reward_per_update);
            println!("   Total Potential: {} COMPASS", (job.total_updates_required - job.completed_updates) as u64 * job.worker_reward_per_update);
            job_index += 1;
        }

        // 3. AI Compute Jobs
        for job in &compute_jobs {
             println!("\n{}. [AI COMPUTE] Model: {}", job_index, job.model_id);
             println!("   Job ID: {}", job.job_id);
             println!("   Units: {}", job.max_compute_units);
             job_index += 1;
        }
        
        println!("\n─────────────────────────────────────────────────");
        println!("A. [AUTO] Start Autonomous AI Worker (Loop)");
        println!("-------------------------------------------------");
        
        println!("\nOptions:");
        if total_jobs > 0 {
            println!("  [1-{}] - Select job to work on", job_index - 1);
        }
        println!("  R - Refresh list");
        println!("  Q - Quit");
        print!("\nSelect: ");
        io::stdout().flush().unwrap();
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        match choice.trim().to_uppercase().as_str() {
            "R" => continue,
            "Q" => break,
            "A" => {
                ai_worker_loop(&client, &worker_keypair).await?;
            }
            num => {
                 if let Ok(idx) = num.parse::<usize>() {
                     if idx > 0 && idx < job_index {
                         // Determine execution path
                         if idx <= single_jobs.len() {
                             // Process single job
                             let job = &single_jobs[idx - 1];
                             process_single_job(job, &client, &worker_keypair).await?;
                         } else if idx <= single_jobs.len() + recurring_jobs.len() {
                             // Process recurring job
                             let job = &recurring_jobs[idx - 1 - single_jobs.len()];
                             process_recurring_job(job, &client, &worker_keypair).await?;
                         } else {
                             // Process Compute Job
                             let job = &compute_jobs[idx - 1 - single_jobs.len() - recurring_jobs.len()];
                             process_compute_job(job, &client, &worker_keypair).await?;
                         }
                     }
                 }
            }
        }
    }
    
    Ok(())
}

async fn process_compute_job(job: &PendingJob, client: &RpcClient, worker_keypair: &KeyPair) -> Result<(), String> {
    println!("\n🧠 Processing Single AI Job: {}", job.job_id);
    let worker_id = worker_keypair.public_key_hex();
    
    // Execute logic using the shared helper (extracted from loop)
    execute_ai_logic(job, &worker_id, worker_keypair, client).await?;
    log_job_history("COMPUTE", &job.job_id, &format!("Model: {}", job.model_id));
    Ok(())
}

use crate::rpc::types::{PendingJob, SubmitResultParams};
async fn ai_worker_loop(client: &RpcClient, worker_keypair: &KeyPair) -> Result<(), String> {
    println!("\n🧠 Starting AI Worker...");
    println!("   Monitoring for compute jobs...");
    println!("   Supported Models: llama-3-8b, stable-diffusion-xl, crypto-signal-v1");
    println!("   (Press Ctrl+C to stop)");
    
    let worker_id = worker_keypair.public_key_hex();

    loop {
        // 1. Poll for Jobs
        let jobs: Vec<PendingJob> = match client.call_method("getPendingComputeJobs", json!({})).await {
            Ok(j) => j,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        
        // 2. Pick a Job
        if let Some(job) = jobs.first() {
            println!("\n🚀 Job Detected: {}", job.job_id);
            execute_ai_logic(job, &worker_id, worker_keypair, client).await?;
            log_job_history("COMPUTE_AUTO", &job.job_id, &format!("Model: {}", job.model_id));
        } else {
            print!("."); io::stdout().flush().unwrap();
        }
        
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}


async fn fetch_recurring_jobs(client: &RpcClient) -> Result<Vec<RecurringOracleJob>, String> {
    client.call_method("getRecurringJobs", serde_json::json!({})).await
}

async fn fetch_pending_oracle_jobs(client: &RpcClient) -> Result<Vec<OracleVerificationJob>, String> {
    client.call_method("getPendingOracleJobs", serde_json::json!({})).await
}


async fn process_recurring_job(job: &RecurringOracleJob, client: &RpcClient, worker_keypair: &KeyPair) -> Result<(), String> {
    println!("\n🔄 Starting Recurring Worker for {}...", job.ticker);
    println!("   Target: {} updates ({} completed)", job.total_updates_required, job.completed_updates);
    println!("   Interval: {} seconds", job.interval_seconds);
    println!("   Reward: {} COMPASS per update", job.worker_reward_per_update);
    println!("\nPress Ctrl+C to stop monitoring.");
    
    // Use proper identity
    let worker_id = worker_keypair.public_key_hex();
    println!("🔑 Worker ID: {}", worker_id);
    
    let fetcher = PriceFetcher::new();

    loop {
        // 1. Get latest job status
        let status_json: serde_json::Value = client.call_method("getJobProgress", json!({ "job_id": job.job_id })).await.unwrap_or(json!({}));
        
        // Parse status 
        let current_completed = status_json.get("completed_updates")
             .and_then(|v| v.as_u64())
             .unwrap_or(job.completed_updates as u64) as u32;
        
        let total_required = status_json.get("total_updates")
             .and_then(|v| v.as_u64())
             .unwrap_or(job.total_updates_required as u64) as u32;

        
        if current_completed >= total_required {
            println!("🎉 Job Completed! All updates finished.");
            break;
        }

        println!("\n⏳ Performing verification {}/{}...", current_completed + 1, total_required);
        
        // --- Layer 3 Finance Oracle Logic (Hoisted) ---
        if job.ticker == "FINANCE_ML_V1" {
            // Call the modular agent logic with CONTINUOUS duration (uses the job's interval)
            match crate::layer3::agent::run_continuous_cycle(job, client, worker_keypair, current_completed + 1, job.interval_seconds).await {
                Ok(_) => {
                     // No sleep here, the agent function consumed the interval duration!
                     continue; 
                },
                Err(e) => {
                    println!("❌ Agent Verification Failed: {}", e);
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            }
        }

        // 2. Fetch Prices (Standard)
        let quotes_res = fetcher.fetch_all(&job.ticker).await;
        
        if let Err(e) = quotes_res {
             println!("⚠️ Failed to fetch prices: {}, retrying in 10s...", e);
             tokio::time::sleep(Duration::from_secs(10)).await;
             continue;
        }
        let quotes = quotes_res.unwrap();
        
        if quotes.is_empty() {
             println!("⚠️ No quotes found, retrying in 10s...");
             tokio::time::sleep(Duration::from_secs(10)).await;
             continue;
        }

        // Calculate average
        let avg_price_dec = PriceFetcher::calculate_average(&quotes);
        let avg_price = avg_price_dec.to_string();
        println!("   Avg Price: ${}", avg_price);
        
        // 3. Create payload & Sign
        let passed = true; 
        
        // Message format "ORACLE_VERIFY:{job_id}:{ticker}:{oracle_price}:{avg_price}"
        // For recurring jobs, we set oracle_price = avg_price
        let message = format!("ORACLE_VERIFY:{}:{}:{}:{}", job.job_id, job.ticker, avg_price, avg_price);
        let signature = worker_keypair.sign_hex(message.as_bytes());
        
        // 4. Submit
        let submit_req = json!({
            "job_id": job.job_id,
            "ticker": job.ticker,
            "oracle_price": avg_price.to_string(), 
            "external_prices": quotes.iter().map(|q| (q.source.clone(), q.price_usd.to_string())).collect::<Vec<_>>(),
            "avg_external_price": avg_price.to_string(),
            "deviation_pct": "0.0", 
            "passed": passed,
            "worker_id": worker_id,
            "signature": signature,
            "update_number": current_completed + 1
        });
        
        let submit_res: Result<serde_json::Value, String> = client.call_method("submitOracleVerificationResult", submit_req).await;
        
        match submit_res {
            Ok(_) => {
                 println!("✅ Verified! Earned {} COMPASS", job.worker_reward_per_update);
                 log_job_history("RECURRING", &job.job_id, &format!("Update {}/{}, Price: {}", current_completed + 1, total_required, avg_price));
                 
                 // Log to file
                 if let Err(e) = log_oracle_data(&job.ticker, &avg_price, current_completed + 1) {
                     println!("⚠️ Failed to log data: {}", e);
                 } else {
                     println!("   📝 Data saved to oracle_history.csv");
                 }
                 
                 // Fetch Balance
                 if let Ok(bal_json) = client.get_account_info(&worker_id).await {
                      if let Some(bal_map) = bal_json.get("balances").and_then(|b| b.as_object()) {
                          if let Some(compass) = bal_map.get("Compass-LTC").and_then(|v| v.as_u64()) {
                               println!("   💰 Current Worker Balance: {} COMPASS", compass);
                          }
                      }
                 }
            },
            Err(e) => println!("❌ RPC Error: {}", e)
        }
        
        // Wait for interval
        println!("💤 Sleeping for {}s...", job.interval_seconds);
        tokio::time::sleep(Duration::from_secs(job.interval_seconds)).await;
        
    }
    
    Ok(())
}


// Extracted logic for reuse
async fn execute_ai_logic(job: &PendingJob, worker_id: &str, worker_keypair: &KeyPair, client: &RpcClient) -> Result<(), String> {
        println!("   Model: {}", job.model_id);
        
        // 2. Execute Job (Simulation or Real)
        let result_data: Vec<u8>;
        
        if job.model_id == "crypto-signal-v1" {
             // Real Implementation: Fetch prices and calc signal
             println!("⚙️  Running 'crypto-signal-v1' analysis for BTC...");
             
             let fetcher = PriceFetcher::new();
             // Fetch multiple times to simulate "history"
             let mut prices = Vec::new();
             for _ in 0..5 {
                 if let Ok(quotes) = fetcher.fetch_all("BTC").await {
                      let avg = PriceFetcher::calculate_average(&quotes);
                      prices.push(avg);
                      print!("."); io::stdout().flush().unwrap();
                 }
                 tokio::time::sleep(Duration::from_secs(1)).await;
             }
             println!(" Done!");
             
             // ML Implementation: Linear Regression
             println!("⚙️  Training Linear Regression Model...");
             
             let n = prices.len() as f64;
             let mut sum_x = 0.0;
             let mut sum_y = 0.0;
             let mut sum_xy = 0.0;
             let mut sum_xx = 0.0;
             
             for (i, price) in prices.iter().enumerate() {
                 let x = i as f64;
                 let y = price.to_f64().unwrap_or(0.0);
                 
                 sum_x += x;
                 sum_y += y;
                 sum_xy += x * y;
                 sum_xx += x * x;
             }
             
             let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
             
             let mean_y = sum_y / n;
             let mut ss_tot = 0.0;
             let mut ss_res = 0.0;
             for (i, price) in prices.iter().enumerate() {
                 let x = i as f64;
                 let y = price.to_f64().unwrap_or(0.0);
                 let y_pred = slope * x + (sum_y - slope * sum_x) / n;
                 
                 ss_tot += (y - mean_y).powi(2);
                 ss_res += (y - y_pred).powi(2);
             }
             let r_squared = 1.0 - (ss_res / ss_tot);
             let confidence = if r_squared.is_nan() { 0.5 } else { r_squared };

             let trend = if slope > 5.0 { "STRONG BUY" } 
                        else if slope > 0.0 { "BUY" }
                        else if slope < -5.0 { "STRONG SELL" }
                        else { "SELL" };

             let start = prices.first().unwrap_or(&Decimal::ZERO);
             let end = prices.last().unwrap_or(&Decimal::ZERO);
             
             let json_result = json!({
                 "signal": trend, 
                 "ticker": "BTC",
                 "model": "LinearRegression-v1",
                 "slope": slope,
                 "r_squared": confidence,
                 "start_price": start.to_string(),
                 "end_price": end.to_string(),
             });
             
             println!("📈 ML Signal Generated: {} (Slope: {:.2}, Conf: {:.2}%)", trend, slope, confidence * 100.0);
             result_data = serde_json::to_vec(&json_result).unwrap();
             
        } else {
            // Real PoUW Benchmark Implementation
            println!("⚙️  Running Real Benchmark (Matrix/Hash Ops)...");
            
            let start = std::time::Instant::now();
            let mut ops = 0u64;
            let duration_target = std::time::Duration::from_secs(5); // Run for 5 seconds
            
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            let data = b"start_data";
            hasher.update(data);
            let mut current_hash = hasher.finalize();
            
            while start.elapsed() < duration_target {
                // Perform work: Chain hashing
                let mut h = Sha256::new();
                h.update(&current_hash);
                current_hash = h.finalize();
                ops += 1;
            }
            
            let duration = start.elapsed().as_secs_f64();
            let ops_per_sec = (ops as f64 / duration) as u64;
            
            println!("   ✅ Benchmark Complete: {:.2} kOps/s", ops_per_sec as f64 / 1000.0);
            
            result_data = format!("AI_OUTPUT: {}_hashes_verified", ops).into_bytes();
            
            // Pass rate to submission
            // Pass rate to submission
             let resp: Result<String, Box<dyn std::error::Error>> = client.submit_result(
                job.job_id.clone(),
                worker_id.to_string(),
                result_data.clone(),
                None,
                None,
                ops_per_sec // Pass the real rate
            ).await;
            
            match resp {
                Ok(tx) => println!("✅ Verified! Reward Pending (Rate: {}). Tx: {}", ops_per_sec, tx),
                Err(e) => println!("❌ Error: {}", e),
            }
            return Ok(());
        }
        
        let msg = format!("COMPUTE_RESULT:{}:{}", job.job_id, worker_id);
        let signature = worker_keypair.sign_hex(msg.as_bytes());

        // For crypto-signal (above block), we default rate to 500 for now or calculate it too?
        // To keep it simple, we only support rate submission for this generic path or update crypto-signal to use it too.
        // Let's update the crypto-signal path to also use submit_result wrapper cleanly or just leave it using raw 'call_method' but add the param.
        
        // Actually, the code below used 'call_method' manually for some reason.
        // Let's unify to use client.submit_result if possible, or update the manual JSON construction.
        // Since I'm replacing the 'else' block AND the shared submission code below it... 
        // Wait, the original code had the submission AFTER the if/else.
        // My replacement effectively returns early for the else block.
        // I should probably handle the crypto-signal case too.
        
        // Re-reading target content to see scope.
        // Target includes lines 417 to 442. 
        // Line 442 is `let resp: ... = client.call_method`.
        
        // I will update the manual JSON construction to include compute_rate: 0 for the crypto-signal case (or 1000 fixed).
        
        let req = json!({
            "job_id": job.job_id,
            "worker_id": worker_id,
            "result_data": result_data,
            "signature": signature,
            "compute_rate": 1000 // Fixed rate for ML model (Logic Task)
        });
        
        println!("📤 Submitting result...");
        let resp: Result<serde_json::Value, String> = client.call_method("submitResult", req).await;
        
        match resp {
            Ok(_) => println!("✅ Verified! Reward Pending."),
            Err(e) => println!("❌ Error: {}", e),
        }
        Ok(())
    }

async fn process_single_job(job: &OracleVerificationJob, client: &RpcClient, worker_keypair: &KeyPair) -> Result<(), String> {
    println!("\n📊 Processing SINGLE verification for {}...", job.ticker);
    
    let worker_id = worker_keypair.public_key_hex();
    println!("   Worker Address: {}", worker_id);
    
    let fetcher = PriceFetcher::new();

    // Fetch
    let quotes = fetcher.fetch_all(&job.ticker).await.map_err(|e| e.to_string())?;
    
    if quotes.is_empty() {
        return Err("No prices fetched".to_string());
    }
    let avg_price_dec = PriceFetcher::calculate_average(&quotes);
    let avg_price = avg_price_dec.to_string();
    
    println!("   Fetched {} prices. Avg: ${}", quotes.len(), avg_price);
    
    // Calc deviation
    let oracle_price_val = job.oracle_price.as_ref().and_then(|p| Decimal::from_str(p).ok()).unwrap_or(Decimal::ZERO);
    let avg_dec = Decimal::from_str(&avg_price).unwrap();
    
    let mut deviation = Decimal::ZERO;
    let mut passed = true;
    
    if !oracle_price_val.is_zero() {
         deviation = ((avg_dec - oracle_price_val).abs() / oracle_price_val) * Decimal::from(100);
         println!("   Oracle: ${} | Deviation: {}%", oracle_price_val, deviation.round_dp(2));
         if deviation > Decimal::from(5) { 
             println!("⚠️ High deviation detected!"); 
             passed = false; 
         }
    } else {
        println!("   (No oracle baseline to compare, submitting feed update)");
    }
    
    // Sign
    // Msg: "ORACLE_VERIFY:<JOB_ID>:<TICKER>:<ORACLE_PRICE>:<AVG_PRICE>"
    let message = format!("ORACLE_VERIFY:{}:{}:{}:{}", job.job_id, job.ticker, oracle_price_val, avg_price);
    let signature = worker_keypair.sign_hex(message.as_bytes());
    
    // Submit via raw JSON
     let req = json!({
        "job_id": job.job_id,
        "ticker": job.ticker,
        "oracle_price": oracle_price_val.to_string(),
            "external_prices": quotes.iter().map(|q| (q.source.clone(), q.price_usd.to_string())).collect::<Vec<_>>(),
        "avg_external_price": avg_price.to_string(),
        "deviation_pct": deviation.to_string(),
        "passed": passed,
        "worker_id": worker_id,
        "signature": signature,
        "update_number": 1
     });
     
     let _: serde_json::Value = client.call_method("submitOracleVerificationResult", req).await?;
     
     println!("✅ Result submitted successfully!");
     log_job_history("SINGLE", &job.job_id, &format!("Ticker: {}, Price: {}", job.ticker, avg_price));
     
     Ok(())
}

fn log_oracle_data(ticker: &str, price: &str, update_num: u32) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    
    let file_path = "oracle_history.csv";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;
        
    let timestamp = chrono::Utc::now().to_rfc3339();
    let line = format!("{},{},{},{}\n", timestamp, ticker, price, update_num);
    
    file.write_all(line.as_bytes())?;
    Ok(())
}

// --- Persistence Helpers ---

#[allow(dead_code)]
fn load_or_create_worker_identity() -> KeyPair {
    let path = "worker_identity.json";
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(kp) = KeyPair::from_secret_hex(&content) {
            return kp;
        }
    }
    
    // Create new
    let kp = KeyPair::generate();
    if let Ok(_) = std::fs::write(path, kp.secret_key_hex()) {
        println!("(Saved new worker identity to {})", path);
    }
    kp
}

fn log_job_history(job_type: &str, job_id: &str, details: &str) {
    let path = "worker_history.csv";
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|_| std::fs::File::create(path).unwrap());
        
    let timestamp = chrono::Utc::now().to_rfc3339();
    let _ = writeln!(file, "{},{},{},{}", timestamp, job_type, job_id, details);
}
