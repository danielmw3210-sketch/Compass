use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use ndarray::Array2; // Requires 'ndarray' dependency


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
    pub verifiers: Vec<String>,     // List of nodes that verified this result <--- NEW
    pub verification_status: ComputeJobStatus,
    pub timestamp: u64,
    pub started_at: Option<u64>,   // When worker claimed job
    pub completed_at: Option<u64>,
    pub compute_rate: u64,         // Measured performance (kOps/s)
    pub min_duration: u64,         // Minimum seconds required (anti-cheat)
    pub inputs: Vec<u8>,           // Input data (serialized tensor or text)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ComputeJobStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Verified, // Confirmed by verifiers
    Disputed, // Conflicting results
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkProof {
    pub worker_id: String,
    pub input_matrix_hash: String,
    pub output_matrix_hash: String,
    pub compute_rate: u64,
    pub signature: String, // Worker signs the hash
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComputeVerify {
    pub job_id: String,
    pub proof: WorkProof,
    pub verifier_id: String, // Who verified it? (Can be empty if this IS the worker's claim)
    pub is_match: bool,      // True if verifier agrees
}

impl ComputeJob {
    pub fn new(job_id: String, creator: String, model_id: String, inputs: Vec<u8>, reward_amount: u64) -> Self {
        Self {
            job_id,
            creator,
            model_id,
            max_compute_units: 5000,
            reward_amount,
            status: ComputeJobStatus::Pending,
            worker_id: None,
            result_hash: None,
            verifiers: Vec::new(),
            verification_status: ComputeJobStatus::Pending,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            started_at: None,
            completed_at: None,
            compute_rate: 0,
            min_duration: 10, // Default: 10 seconds minimum
            inputs,
        }
    }
    
    pub fn assign_to_worker(&mut self, worker_id: String) {
        self.worker_id = Some(worker_id);
        self.status = ComputeJobStatus::InProgress;
        self.started_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
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

    /// Deterministic Matrix Multiplication (Legacy PoW)
    pub fn execute_deterministic_task(&self, size: usize) -> Result<String, String> {
        let seed_str = format!("{}_seed", self.job_id);
        let seed = seed_str.as_bytes();
        let a = generate_deterministic_matrix(size, seed, 0);
        let b = generate_deterministic_matrix(size, seed, 1);
        let c = a.dot(&b);
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", c));
        Ok(hex::encode(hasher.finalize()))
    }

    /// Real AI Inference using ONNX Runtime
    pub fn execute_inference(&self) -> Result<String, String> {
        use ort::session::{Session, builder::GraphOptimizationLevel};
        use ort::value::Value;
        use ndarray::Array4; 
        
        let model_path = format!("models/{}.onnx", self.model_id);
        if !std::path::Path::new(&model_path).exists() {
            return Err(format!("Model file not found: {}", model_path));
        }

        // Load Model
        let mut session = Session::builder()
            .map_err(|e| format!("Failed to create session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Failed to set optimization: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| format!("Failed to set threads: {}", e))?
            .commit_from_file(&model_path)
            .map_err(|e| format!("Failed to load model from {}: {}", model_path, e))?;

        // Prepare Inputs
        // Use standard 1x3x224x224 float tensor (ImageNet standard)
        let input_tensor = if self.inputs.len() > 0 {
             Array4::<f32>::zeros((1, 3, 224, 224))
        } else {
             Array4::<f32>::from_elem((1, 3, 224, 224), 0.5)
        };
        
        // Use explicit shape and raw data to avoid ndarray version mismatch issues with ort
        let shape = vec![1, 3, 224, 224];
        let data = input_tensor.into_raw_vec();

        let input_name = session.inputs.first()
            .map(|i| i.name.clone())
            .ok_or("Model has no inputs".to_string())?;
            
        // Get output name BEFORE running session to avoid borrow conflicts
        let output_name = session.outputs.first()
            .map(|o| o.name.clone())
            .ok_or("Model has no outputs".to_string())?;

        let input_value = Value::from_array((shape, data))
             .map_err(|e| format!("Failed to create ORT value: {}", e))?;

        // Run Inference
        let outputs = session.run(ort::inputs![input_name => input_value])
            .map_err(|e| format!("ORT Run Error: {}", e))?; // inputs! macro returns Vec, session.run takes it.

        // 5. Hash the Output
        // We take the first output tensor using the name we extracted earlier
        let output_val = outputs.get(&output_name)
             .ok_or("Failed to get output tensor".to_string())?;
             
        // Extract data to hash
        let (_, output_slice) = output_val.try_extract_tensor::<f32>()
             .map_err(|e| format!("Failed to extract output tensor: {}", e))?;
             
        let mut hasher = Sha256::new();
        for val in output_slice {
            hasher.update(val.to_le_bytes());
        }
        let result_hash = hex::encode(hasher.finalize());
        
        Ok(result_hash)
    }
}

fn generate_deterministic_matrix(size: usize, seed: &[u8], salt: u8) -> Array2<f32> {
    let mut data = Vec::with_capacity(size * size);
    let mut cycle_seed = seed.iter().cycle();
    
    for i in 0..size*size {
        let s = *cycle_seed.next().unwrap() as f32;
        let v = (s + salt as f32 + i as f32).sin(); // Pseudo-random deterministic
        data.push(v);
    }
    
    Array2::from_shape_vec((size, size), data).unwrap()
}
