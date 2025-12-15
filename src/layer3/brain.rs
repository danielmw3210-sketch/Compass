use candle_core::{Device, Result, Tensor, DType};
use candle_nn::{Linear, Module, VarBuilder, VarMap};

/// Fallback MLP Brain (Randomized)
pub struct SimpleBrain {
    layers: Vec<Linear>,
    pub input_dim: usize,
    pub output_dim: usize,
    pub varmap: VarMap,
}

impl SimpleBrain {
    /// Initialize a random brain
    pub fn new_random(input_dim: usize, output_dim: usize) -> Result<Self> {
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        
        // Simple MLP: Input -> 64 -> 64 -> Output
        let hidden_dim = 64;
        
        let l1 = candle_nn::linear(input_dim, hidden_dim, vs.pp("l1"))?;
        let l2 = candle_nn::linear(hidden_dim, hidden_dim, vs.pp("l2"))?;
        let l3 = candle_nn::linear(hidden_dim, output_dim, vs.pp("l3"))?;
        
        Ok(Self {
            layers: vec![l1, l2, l3],
            input_dim,
            output_dim,
            varmap,
        })
    }
    
    /// Forward pass (Inference)
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let mut x = input.clone();
        
        // Layer 1 + ReLU
        x = self.layers[0].forward(&x)?.relu()?;
        
        // Layer 2 + ReLU
        x = self.layers[1].forward(&x)?.relu()?;
        
        // Output Layer (Sigmoid for probability 0.0-1.0)
        x = self.layers[2].forward(&x)?;
        
        // Sigmoid activation manually
        let output = (x.neg()?.exp()? + 1.0)?.recip()?; 
        
        Ok(output)
    }
    
    pub fn save_weights(&self, path: &str) -> Result<()> {
        self.varmap.save(path)
    }
    
    pub fn load_weights(&mut self, path: &str) -> Result<()> {
        self.varmap.load(path)
    }
}

// ==============================================================================
//  ORT (ONNX Runtime) Implementation
// ==============================================================================
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Value;
use std::sync::Once;

static INIT: Once = Once::new();

/// Real ONNX Brain using 'ort' crate
pub struct OnnxBrain {
    session: Session,
}

impl OnnxBrain {
    pub fn new(path: &str) -> Result<Self> {
        // Initialize ORT environment once
        INIT.call_once(|| {
             let _ = ort::init()
                 .with_name("CompassAI")
                 .commit();
        });

        let session = Session::builder()
            .map_err(|e| candle_core::Error::Msg(format!("ORT Builder Error: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| candle_core::Error::Msg(format!("ORT Opt Error: {}", e)))?
            .with_intra_threads(4)
            .map_err(|e| candle_core::Error::Msg(format!("ORT Threads Error: {}", e)))?
            .commit_from_file(path)
            .map_err(|e| candle_core::Error::Msg(format!("Failed to load ONNX model: {}", e)))?;

        Ok(Self { session })
    }

    pub fn forward(&mut self, input: &Tensor) -> Result<Tensor> {
        // 1. Convert Candle Tensor -> Vec<f32>
        let input_vec: Vec<f32> = input.flatten_all()?.to_vec1()?;
        
        // 2. Create ORT Value from (Shape, Data) tuple
        // This avoids complex ndarray trait bounds
        let shape = vec![1, input_vec.len()];
        let input_value = Value::from_array((shape, input_vec.clone()))
            .map_err(|e| candle_core::Error::Msg(format!("ORT Value Error: {}", e)))?;

        // 3. Run ORT Inference
        let outputs = self.session.run(ort::inputs![input_value])
            .map_err(|e| candle_core::Error::Msg(format!("ORT Run Error: {}", e)))?;

        // 4. Extract Output (First tensor found)
        let (_, output_val) = outputs.iter().next()
             .ok_or(candle_core::Error::Msg("No output from ONNX".into()))?;
        
        // Extract data
        let (_, output_slice) = output_val.try_extract_tensor::<f32>()
             .map_err(|e| candle_core::Error::Msg(format!("Failed to extract tensor: {}", e)))?;
        
        let output_data: Vec<f32> = output_slice.to_vec();

         // 5. Convert back to Candle Tensor
         let out_len = output_data.len();
         Tensor::from_vec(output_data, (1, out_len), &Device::Cpu)
    }
}
