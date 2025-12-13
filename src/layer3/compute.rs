use serde::{Serialize, Deserialize};

/// Compute job for neural network training/inference
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComputeJob {
    pub job_id: String,
    pub creator: String,           // Admin pubkey
    pub model_id: String,          // "neural-net-recurring"
    pub max_compute_units: u64,    // Duration in iterations
    pub reward_amount: u64,        // COMPUTE tokens to award
    pub status: ComputeJobStatus,
    pub worker_id: Option<String>, // Assigned worker
    pub result_hash: Option<String>, // Hash of computation result
    pub timestamp: u64,
    pub completed_at: Option<u64>,
    pub compute_rate: u64,         // Measured performance (kOps/s)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ComputeJobStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ComputeJob {
    pub fn new(job_id: String, creator: String, model_id: String, reward_amount: u64) -> Self {
        Self {
            job_id,
            creator,
            model_id,
            max_compute_units: 5000,
            reward_amount,
            status: ComputeJobStatus::Pending,
            worker_id: None,
            result_hash: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            completed_at: None,
            compute_rate: 0,
        }
    }
    
    pub fn assign_to_worker(&mut self, worker_id: String) {
        self.worker_id = Some(worker_id);
        self.status = ComputeJobStatus::InProgress;
    }
    
    pub fn complete(&mut self, result_hash: String, compute_rate: u64) {
        self.result_hash = Some(result_hash);
        self.compute_rate = compute_rate;
        self.status = ComputeJobStatus::Completed;
        self.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }
}
