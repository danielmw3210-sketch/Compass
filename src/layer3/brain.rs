use candle_core::{Device, Result, Tensor, DType};
use candle_nn::{Linear, Module, VarBuilder, VarMap};

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
