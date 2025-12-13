#![allow(unused_imports)]
use crate::client::RpcClient;
use serde_json::json;
use std::time::Duration;
use tracing::{info, error};

/// Initialize the Finance Oracle Job (FINANCE_ML_V1)
/// This function ensures the recurring job exists on the chain.
pub async fn init_finance_oracle_job(client: &RpcClient) -> Result<(), String> {
    info!("🚀 Initializing Finance Oracle Layer 3 (FINANCE_ML_V1)...");

    // 1. Check if job already exists along active jobs
    let jobs = client.call_method::<serde_json::Value, Vec<crate::rpc::types::RecurringOracleJob>>("getRecurringJobs", json!({})).await
        .map_err(|e| format!("Failed to fetch jobs: {}", e))?;

    if jobs.iter().any(|j| j.ticker == "FINANCE_ML_V1" && j.status == "Active") {
        info!("✓ FINANCE_ML_V1 job is already active.");
        return Ok(());
    }

    // 2. Submit new job if not found
    let params = json!({
        "ticker": "FINANCE_ML_V1",
        "duration_hours": 1,       // Run for 1 hour (User Request)
        "interval_minutes": 1,      // 1-minute interval for high-frequency training
        "reward_per_update": 10,    // 10 COMPASS per update
        "submitter": "Admin_Layer3_Init"
    });

    let res: serde_json::Value = client.call_method("submitRecurringOracleJob", params).await
        .map_err(|e| format!("Failed to create job: {}", e))?;
    
    info!("✅ Finance Oracle Job Created! Job ID: {}", res["job_id"]);
    Ok(())
}
