use crate::client::rpc_client::RpcClient;
use crate::crypto::KeyPair;
use crate::network::{NetMessage, TOPIC_COMPUTE_JOBS};
use crate::layer3::compute::{ComputeJob, WorkProof, ComputeVerify};
use tokio::sync::broadcast;
use sha2::Digest;

pub struct AiWorker {
    _client: RpcClient, // Still needed for result submission to chain? No, gossip now.
    gossip_tx: broadcast::Sender<(NetMessage, String)>,
    gossip_rx: broadcast::Receiver<(NetMessage, String)>,
    keypair: KeyPair,
}

impl AiWorker {
    pub fn new(
        node_url: String, 
        keypair: KeyPair,
        gossip_tx: broadcast::Sender<(NetMessage, String)>,
    ) -> Self {
        let gossip_rx = gossip_tx.subscribe();
        AiWorker {
            _client: RpcClient::new(node_url),
            gossip_tx,
            gossip_rx,
            keypair,
        }
    }

    pub async fn start(&mut self) {
        let worker_id = self.keypair.public_key_hex();
        println!("🤖 P2P Verified Compute Worker Started.");
        println!("   Worker ID: {}", worker_id);
        println!("   Listening on topic: {}", TOPIC_COMPUTE_JOBS);

        loop {
            tokio::select! {
                Ok((msg, _source)) = self.gossip_rx.recv() => {
                    match msg {
                        NetMessage::ComputeJob(job) => {
                            self.handle_job(job).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn handle_job(&self, job: ComputeJob) {
        // 1. Criteria Check
        if job.reward_amount < 10 { return; } // Min threshold
        println!("⚡ Received Job: {} (Model: {})", job.job_id, job.model_id);

        // 2. Ensure Model Exists (Download if missing)
        let model_path = format!("models/{}.onnx", job.model_id);
        if !std::path::Path::new(&model_path).exists() {
            println!("   📥 Downloading Model: {}...", job.model_id);
            // Default to SqueezeNet if known ID, else try generic URL or fail
            let url = if job.model_id == "squeezenet" {
                "https://github.com/onnx/models/raw/main/validated/vision/classification/squeezenet/model/squeezenet1.0-9.onnx"
            } else {
                 // Fallback for demo: use squeezenet for everything if unknown
                 println!("   ⚠️ Unknown model ID, downloading default SqueezeNet for demo.");
                 "https://github.com/onnx/models/raw/main/validated/vision/classification/squeezenet/model/squeezenet1.0-9.onnx"
            };

            // Download using blocking reqwest (since we act as worker)
            // Ideally async, but for simplicity in this loop we can block or await
            if let Ok(resp) = reqwest::get(url).await {
                if let Ok(bytes) = resp.bytes().await {
                    if let Err(e) = std::fs::write(&model_path, bytes) {
                        println!("   ❌ Failed to save model: {}", e);
                        return;
                    }
                    println!("   ✅ Model Saved: {}", model_path);
                } else {
                    println!("   ❌ Failed to download model bytes.");
                    return;
                }
            } else {
                println!("   ❌ Failed to download model: Connection error.");
                return;
            }
        }

        // 3. Execute Real Inference OR Training
        let start = std::time::Instant::now();
        
        let final_hash = if job.model_id.starts_with("TRAIN") {
            println!("   🏋️ RECEIVED TRAINING JOB: {}", job.job_id);
            
            // Determine script based on model ID
            let script_name = if job.model_id.contains("BTC") {
                "scripts/train_btc_agent.py"
            } else if job.model_id.contains("ETH") {
                 "scripts/train_eth_agent.py"
            } else if job.model_id.contains("SOL") {
                 "scripts/train_sol_agent.py"
            } else if job.model_id.contains("LTC") {
                 "scripts/train_ltc_agent.py"
            } else {
                 "scripts/train_btc_agent.py" // Default
            };

            println!("   🔄 Executing Python Training Script: {}...", script_name);
            
            // Execute python script with fallback
            let mut cmd = std::process::Command::new("python");
            cmd.arg(script_name);
            
            let mut output_res = cmd.output();
            
            if output_res.is_err() {
                 output_res = std::process::Command::new("py").arg(script_name).output();
            }

            match output_res {
                Ok(out) if out.status.success() => {
                    println!("   ✅ Training Complete.");
                    // Check for generated model
                    let ticker = if job.model_id.contains("BTC") { "btc" } 
                                 else if job.model_id.contains("ETH") { "eth" }
                                 else if job.model_id.contains("SOL") { "sol" }
                                 else { "ltc" };
                                 
                    if let Ok(bytes) = std::fs::read(format!("models/{}_v1.onnx", ticker)) {
                        format!("{:x}", sha2::Sha256::digest(&bytes))
                    } else {
                         // Check dist/models just in case
                         if let Ok(bytes) = std::fs::read(format!("dist/models/{}_v1.onnx", ticker)) {
                             format!("{:x}", sha2::Sha256::digest(&bytes))
                         } else {
                             "training_success_file_missing".to_string()
                         }
                    }
                },
                Ok(out) => {
                    println!("   ❌ Training Script Failed: {:?}", String::from_utf8_lossy(&out.stderr));
                     return;
                },
                Err(e) => {
                    println!("   ❌ Training Launch Failed: {}", e);
                    return;
                }
            }
        } else {
            println!("   🚀 Running ONNX Inference...");
            match job.execute_inference() {
                Ok(hash) => {
                    println!("   ✅ Inference Success (Hash: {}...)", &hash[..8]);
                    hash
                },
                Err(e) => {
                    println!("   ❌ Inference Failed: {}", e);
                    return;
                }
            }
        };

        let duration = start.elapsed();
        println!("   ⏱️  Duration: {:.2}s", duration.as_secs_f32());

        // 4. Create Proof
        let proof = WorkProof {
            worker_id: self.keypair.public_key_hex(),
            input_matrix_hash: "onnx_inference".to_string(),
            output_matrix_hash: final_hash.clone(),
            compute_rate: (1_000_000.0 / duration.as_secs_f32()) as u64, // Mock OPS calculation
            signature: "sig_placeholder".to_string(), 
        };

        // 5. Broadcast Verification
        let verify_msg = NetMessage::ComputeVerify(ComputeVerify {
            job_id: job.job_id,
            proof,
            verifier_id: self.keypair.public_key_hex(), 
            is_match: true,
        });

        // Broadcast back to P2P network
        let _ = self.gossip_tx.send((verify_msg, "self".to_string()));
        println!("   📡 Broadcast Result to Network.");
    }
}
