use std::process::Command;
use std::thread;
use std::time::Duration;
use crate::client::rpc_client::RpcClient;

pub struct AiWorker {
    client: RpcClient,
    node_url: String,
    model_id: String,
}

impl AiWorker {
    pub fn new(node_url: String, model_id: String) -> Self {
        AiWorker {
            client: RpcClient::new(node_url.clone()),
            node_url,
            model_id,
        }
    }

    pub async fn start(&self) {
        println!("🤖 AI Worker Started. Polling for jobs (Model: {})...", self.model_id);
        
        let worker_id = format!("worker_{}", self.model_id); // Simple ID for now

        loop {
            // Mock Poll: asking RPC for pending jobs?
            // Currently our RPC doesn't have "getPendingJobs". 
            // Realistically, the worker would be a Validator that sees the GulfStream via P2P.
            
            // Poll for pending jobs
            match self.client.get_pending_compute_jobs(Some(self.model_id.clone())).await {
                 Ok(jobs) => {
                     if jobs.is_empty() {
                         // No jobs, sleep
                     } else {
                         for job in jobs {
                             println!("Found Job: {} (Units: {})", job.job_id, job.max_compute_units);
                             
                             // Decode inputs (assuming string for prototype)
                             let input_str = String::from_utf8(job.inputs.clone()).unwrap_or_default();
                             println!("Running Inference on input: {}", input_str);
                             
                             // Execute Python Script
                             match self.run_python_inference(&input_str) {
                                 Ok(result) => {
                                     println!("Job Completed! Result: {}", result);
                                     
                                     // Submit Result Transaction back to chain
                                     match self.client.submit_result(
                                         job.job_id.clone(),
                                         worker_id.clone(),
                                         result.as_bytes().to_vec()
                                     ).await {
                                         Ok(tx) => println!("Result committed to chain! TX: {}", tx),
                                         Err(e) => println!("Failed to commit result: {}", e),
                                     }
                                 },
                                 Err(e) => {
                                     println!("Job Failed: {}", e);
                                 }
                             }
                         }
                     }
                 },
                 Err(e) => {
                     println!("Error polling jobs: {}", e);
                 }
            }
            
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    fn run_python_inference(&self, input: &str) -> std::io::Result<String> {
        // ... (existing implementation)
        let output = std::process::Command::new("python")
            .arg("ai_runner.py") // Ensure this script exists in CWD
            .arg(input)
            .output()?;
            
        if output.status.success() {
             Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
             Err(std::io::Error::new(std::io::ErrorKind::Other, String::from_utf8_lossy(&output.stderr)))
        }
    }
}
