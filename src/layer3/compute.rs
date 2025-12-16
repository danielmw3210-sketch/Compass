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
        
        println!("DEBUG [execute_inference]: model_id = '{}', job_id = '{}'", self.model_id, self.job_id);
        
        // Detect Model Type
        // Detect Model Type
        if self.model_id.starts_with("signal_") && self.model_id.ends_with("_v2") {
            // --- V2 Signal Model Inference ---
             println!("DEBUG: Executing V2 Signal Inference for {}", self.job_id);
             
             // 1. Parse Ticker from Model ID (signal_btc_v2 -> btc)
             let parts: Vec<&str> = self.model_id.split('_').collect();
             let ticker_short = parts.get(1).unwrap_or(&"btc");
             let ticker = format!("{}USDT", ticker_short.to_uppercase());
             
             // 2. Deserialize Inputs (Vec<Vec<f64>> [Close, Volume])
             let x: Vec<Vec<f64>> = serde_json::from_slice(&self.inputs)
                .map_err(|e| format!("Invalid inputs for signal model: {}", e))?;
                
             // 3. Extract Prices and Volumes
             let mut prices = Vec::with_capacity(x.len());
             let mut volumes = Vec::with_capacity(x.len());
             for row in x {
                 if row.len() >= 2 {
                     prices.push(row[0]);
                     volumes.push(row[1]);
                 }
             }
             
             // 4. Compute Features
             let features = crate::layer3::signal_model::compute_inference_features(&prices, &volumes)
                 .map_err(|e| format!("Feature calculation failed: {}", e))?;
                 
             // 5. Predict
             let prediction = crate::layer3::signal_model::predict_signal(&ticker, &features)
                 .map_err(|e| format!("Prediction failed: {}", e))?;
                 
             // 6. Return JSON 
             // prediction is u32 (0=SELL, 1=HOLD, 2=BUY)
             let result_json = serde_json::json!({
                 "prediction": prediction as f64,
                 "hash": format!("signal_v2_{}_{}", ticker, prediction)
             });
             
             return Ok(result_json.to_string());
        }

        if self.model_id == "model_sol_v1" {
            // --- Pure Rust SmartCore Inference ---
            println!("DEBUG: Taking NATIVE RUST path for model_sol_v1");
            let model_path = "models/sol_v1.bin";
            println!("🧠 [Rust AI] Loading Native Model: {}", model_path);
            
            let file = std::fs::File::open(&model_path)
                .map_err(|e| format!("Failed to open native model: {}", e))?;
            let mut reader = std::io::BufReader::new(file);
            
            // Import SmartCore types locally to avoid conflicts
            use smartcore::ensemble::random_forest_regressor::RandomForestRegressor;
            use smartcore::linalg::basic::matrix::DenseMatrix;
            
            let rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> = bincode::deserialize_from(&mut reader)
                 .map_err(|e| format!("Failed to deserialize SmartCore model: {}", e))?;
                 
            // Create Input (Last 5 lags from inputs)
            // Inputs are serialized bytes. For now, we assume simple JSON or f64 array.
            // But wait, the Scheduler sends a 30-step sequence (JSON) for everything.
            // We need to extract the last 5 prices/returns from that JSON.
            
            let sequence: Vec<(f64, f64)> = serde_json::from_slice(&self.inputs)
                .map_err(|e| format!("Failed to parse input JSON: {}", e))?;
                
            if sequence.len() < 6 {
                return Err("Input sequence too short for lag features".to_string());
            }
            
            // Feature Engineering (Must match training.rs)
            // Lags 1-5
            // But wait, training data was PRICES.
            // Let's assume input is Price.
            // Lags of Price?
            // "training.rs": x_data.push(close_prices[i + k]); -> Lagged Prices.
            // So we take the LAST 5 prices.
            
            let len = sequence.len();
            let mut features = Vec::new();
            // Lags: [t-5, t-4, t-3, t-2, t-1] to predict t
            // training.rs loop: for k in 0..lags { x.push(p[i+k]) } -> p[i], p[i+1]...
            // If we want to predict Next, we need the most recent 5.
            for i in 0..5 {
                features.push(sequence[len - 5 + i].0); // .0 is Price
            }
            
            let x = DenseMatrix::new(1, 5, features, false);
            let pred = rf.predict(&x).map_err(|e| format!("Inference failed: {}", e))?;
            
            return Ok(pred[0].to_string());
        }

        // --- ONNX Inference (Legacy) ---
        println!("DEBUG: Taking ONNX path for model_id = '{}'", self.model_id);
        
        // Skip stale/legacy model IDs that don't exist (cleanup)
        if self.model_id.contains("_TRAIN") || self.model_id.starts_with("train_") {
            // Training jobs should NOT go through inference - they were handled by GUI training loop
            return Err(format!("Skipping training job '{}' in inference path (handled separately)", self.model_id));
        }
        
        let model_path = format!("models/{}.onnx", self.model_id);
    
        if !std::path::Path::new(&model_path).exists() {
             let cwd = std::env::current_dir().unwrap_or_default();
             // Keep debug print
             println!("DEBUG: CWD is {:?}", cwd);
             println!("DEBUG: Looking for {:?}", model_path);
            return Err(format!("Model file not found: {} (CWD: {:?})", model_path, cwd));
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
        let (shape, data) = if self.model_id.contains("price_decision") {
            println!("DEBUG: Executing Inference for {}", self.job_id);
            
            // Try parsing as simple tuple (V1) or Sequence (V2)
            // If V2, inputs should be Vec<Vec<f64>>
            if let Ok(seq) = serde_json::from_slice::<Vec<Vec<f64>>>(&self.inputs) {
                // Determine what the model wants based on ID
                // Legacy "price_decision" models (NN) expect Rank 2 [Batch, Features]
                // Newer "models" like Transformer expect Rank 3 [Batch, Seq, Features]
                let expected_rank = if self.model_id.contains("price_decision") {
                    2
                } else {
                    3 
                };
                
                if expected_rank == 2 {
                    // Model expects [Batch, Features] (Rank 2) but we have Sequence [Batch, Seq, Features]
                    // Take last step.
                    let last_step = seq.last().unwrap();
                    
                    // FIXED: Legacy 'price_decision_v2' only takes Price (1 feature), not Volume.
                    // Error was: "Got 2 Expected 1" at index 1.
                    let features: Vec<f32> = vec![last_step[0] as f32]; 
                    
                    (vec![1, 1], features)
                } else {
                    // Model expects [Batch, Seq, Features] (Rank 3)
                    let mut data = Vec::new();
                    for step in seq {
                        data.push(step[0] as f32); 
                        data.push(step[1] as f32);
                    }
                    (vec![1, 30, 2], data)
                }
            } else {
                 // Fallback V1 JSON
                 let (p, v): (f64, f64) = serde_json::from_slice(&self.inputs)
                     .map_err(|e| format!("Invalid inputs for price decision: {}", e))?;
                 (vec![1, 2], vec![p as f32, v as f32])
            }
        } else {
             // XGBoost / Other (Rank 2)
              let (p, v): (f64, f64) = serde_json::from_slice(&self.inputs).unwrap_or((0.0, 0.0));
              (vec![1, 2], vec![p as f32, v as f32])
        };
        
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
             
        println!("DEBUG: ONNX Output Values: {:?}", output_slice); // <--- NEW DEBUG

        // 6. Return JSON with Prediction & Hash
        // For Price Model, output is a single float (Predicted Price)
        let prediction = output_slice.first().cloned().unwrap_or(0.0);
        
        let mut hasher = Sha256::new();
        for val in output_slice {
            hasher.update(val.to_le_bytes());
        }
        let hash_str = hex::encode(hasher.finalize());
        
        // Return JSON: { "prediction": 123.45, "hash": "abc..." }
        let result_json = serde_json::json!({
            "prediction": prediction,
            "hash": hash_str
        });
        
        Ok(result_json.to_string())
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
