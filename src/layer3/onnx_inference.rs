use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::{Value, Tensor};
use ndarray::{Array, Array2, ArrayD, IxDyn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScalerParams {
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
    pub features: Vec<String>,
}

pub struct ModelRegistry {
    sessions: HashMap<String, Arc<Mutex<Session>>>,
    scalers: HashMap<String, ScalerParams>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            scalers: HashMap::new(),
        }
    }

    pub fn load_model(&mut self, ticker: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ticker_lower = ticker.to_lowercase().replace("usdt", "");
        
        // 1. Try Load ONNX
        let model_paths = vec![
            format!("dist/models/{}_v1.onnx", ticker_lower),
            format!("models/{}_v1.onnx", ticker_lower),
        ];

        let mut model_loaded = false;
        for path in &model_paths {
            if std::path::Path::new(path).exists() {
                match Session::builder() {
                    Ok(builder) => {
                         // Optimization level might fail on some systems, try safe default?
                         // Removing .with_optimization_level to be safer or keep it? Keep it.
                         if let Ok(session) = builder.with_optimization_level(GraphOptimizationLevel::Level3)
                                            .and_then(|b| b.with_intra_threads(4))
                                            .and_then(|b| b.commit_from_file(path)) 
                        {
                            self.sessions.insert(ticker.to_string(), Arc::new(Mutex::new(session)));
                            info!("✅ Loaded ONNX model for {}: {}", ticker, path);
                            model_loaded = true;
                            break;
                        }
                    },
                    Err(_) => {}
                }
            }
        }
        
        if !model_loaded {
            warn!("⚠️ ONNX model not found for {}. System will use Heuristic Fallback.", ticker);
        }

        // 2. Try Load Scaler
        let scaler_paths = vec![
            format!("dist/models/{}_scaler.json", ticker_lower),
            format!("models/{}_scaler.json", ticker_lower),
        ];
        
        for path in &scaler_paths {
            if std::path::Path::new(path).exists() {
                if let Ok(scaler_data) = std::fs::read_to_string(path) {
                    if let Ok(scaler) = serde_json::from_str::<ScalerParams>(&scaler_data) {
                         self.scalers.insert(ticker.to_string(), scaler);
                         info!("✅ Loaded scaler for {}: {}", ticker, path);
                         break;
                    }
                }
            }
        }
        
        // Always return Ok, as we have fallback
        Ok(())
    }

    pub fn predict(&self, ticker: &str, sequence: &[Vec<f64>]) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let session_opt = self.sessions.get(ticker);
        let scaler_opt = self.scalers.get(ticker);

        // Check availability
        if let (Some(session_mutex), Some(scaler)) = (session_opt, scaler_opt) {
            // --- AI INFERENCE PATH ---
            
            // 1. Scale input
            let scaled_sequence = self.scale_sequence(sequence, scaler);
            
            // 2. Prepare ONNX input tensor
            // Flatten input to 1D vector first
            let input_shape = vec![1, sequence.len() as i64, 2];
            let input_flat: Vec<f32> = scaled_sequence.iter().flatten().map(|&x| x as f32).collect();
            
            // Lock session for inference
            match session_mutex.lock() {
                Ok(mut session) => {
                     match Value::from_array((input_shape, input_flat)) {
                        Ok(input_tensor) => {
                             // 3. Run inference
                             match session.run(ort::inputs![input_tensor]) {
                                 Ok(outputs) => {
                                     if let Ok(output_tensor) = outputs[0].try_extract_tensor::<f32>() {
                                         let (_, data) = output_tensor;
                                         let predicted_scaled = data[0] as f64;
                                         
                                         // 5. Unscale output
                                         let predicted_price = predicted_scaled * scaler.std[0] + scaler.mean[0];
                                         return Ok(predicted_price);
                                     }
                                 },
                                 Err(e) => warn!("ONNX Run failed: {}", e)
                             }
                        },
                        Err(e) => warn!("Tensor creation failed: {}", e)
                     }
                },
                Err(e) => warn!("Session mutex poisoned: {}", e)
            }
            // If any error in AI path, fall through to heuristic
            warn!("⚠️ AI Inference failed for {}, falling back to heuristic.", ticker);
        }

        // --- HEURISTIC FALLBACK PATH ---
        Ok(self.heuristic_predict(sequence))
    }

    fn heuristic_predict(&self, sequence: &[Vec<f64>]) -> f64 {
        // Simple Momentum Strategy
        // If Price > SMA(10), predict slight uptrend.
        if sequence.is_empty() { return 0.0; }
        
        let closes: Vec<f64> = sequence.iter().map(|k| k[0]).collect();
        let last_price = *closes.last().unwrap_or(&0.0);
        
        if closes.len() < 10 { return last_price; }

        let sum: f64 = closes.iter().skip(closes.len().saturating_sub(10)).sum();
        let sma = sum / 10.0;

        // Predict 0.5% move in direction of trend
        if last_price > sma {
            last_price * 1.005
        } else {
            last_price * 0.995
        }
    }

    fn scale_sequence(&self, sequence: &[Vec<f64>], scaler: &ScalerParams) -> Vec<Vec<f64>> {
        sequence.iter().map(|candle| {
            candle.iter().enumerate().map(|(i, &val)| {
                if i < scaler.mean.len() {
                     (val - scaler.mean[i]) / scaler.std[i]
                } else {
                    val // Should not happen if features match
                }
            }).collect()
        }).collect()
    }
}

/// Convert price prediction to trading signal
pub fn price_to_signal(current_price: f64, predicted_price: f64, threshold: f64) -> u32 {
    let change = (predicted_price - current_price) / current_price;
    
    // Log debug info if needed? No, too verbose.
    if change > threshold {
        2 // BUY
    } else if change < -threshold {
        0 // SELL
    } else {
        1 // HOLD
    }
}
