#![allow(dead_code)]
use ndarray::{Array1, Array2};
use ndarray_rand::RandomExt;
use ndarray_rand::rand_distr::Uniform;
use serde::{Serialize, Deserialize};

/// 2025 Cutting-Edge: Attention mechanism for temporal market patterns
#[derive(Clone)]
pub struct AttentionLayer {
    q_weights: Array2<f64>,  // Query
    k_weights: Array2<f64>,  // Key
    v_weights: Array2<f64>,  // Value
}

impl AttentionLayer {
    pub fn new(input_dim: usize, hidden_dim: usize) -> Self {
        Self {
            q_weights: Array2::random((input_dim, hidden_dim), Uniform::new(-0.1, 0.1)),
            k_weights: Array2::random((input_dim, hidden_dim), Uniform::new(-0.1, 0.1)),
            v_weights: Array2::random((input_dim, hidden_dim), Uniform::new(-0.1, 0.1)),
        }
    }

    /// Compute attention scores between query and keys
    pub fn forward(&self, input: &Array1<f64>) -> Array1<f64> {
        // Simplified self-attention for single input
        let query = input.dot(&self.q_weights);
        let key = input.dot(&self.k_weights);
        let value = input.dot(&self.v_weights);

        // Attention score = softmax(Q·K^T / sqrt(d))
        let attention_score = query.dot(&key) / (key.len() as f64).sqrt();
        let attention_weight = 1.0 / (1.0 + (-attention_score).exp());  // Sigmoid as simplified softmax

        // Weighted value
        &value * attention_weight
    }
}

/// 2025 State-of-the-Art: Mixture of Experts layer
pub struct MixtureOfExperts {
    experts: Vec<Array2<f64>>,  // Multiple specialist networks
    gating: Array2<f64>,        // Router that picks experts
}

impl MixtureOfExperts {
    pub fn new(input_dim: usize, output_dim: usize, num_experts: usize) -> Self {
        let mut experts = Vec::new();
        for _ in 0..num_experts {
            experts.push(Array2::random((input_dim, output_dim), Uniform::new(-0.1, 0.1)));
        }
        
        Self {
            experts,
            gating: Array2::random((input_dim, num_experts), Uniform::new(-0.1, 0.1)),
        }
    }

    pub fn forward(&self, input: &Array1<f64>) -> Array1<f64> {
        // Gating network decides which experts to use
        let gate_scores = input.dot(&self.gating);
        let gate_probs = Self::softmax(&gate_scores);

        // Combine expert outputs weighted by gating
        let mut output = Array1::zeros(self.experts[0].ncols());
        for (expert_idx, expert) in self.experts.iter().enumerate() {
            let expert_output = input.dot(expert);
            output = output + &expert_output * gate_probs[expert_idx];
        }
        output
    }

    fn softmax(x: &Array1<f64>) -> Array1<f64> {
        let max_val = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exps: Array1<f64> = x.mapv(|v| (v - max_val).exp());
        let sum: f64 = exps.sum();
        exps / sum
    }
}

/// Meta-Learning: Curriculum scheduler that decides what to focus on
#[derive(Serialize, Deserialize, Clone)]
pub struct CurriculumScheduler {
    task_difficulties: Vec<f64>,     // Estimated difficulty for each task
    task_performance: Vec<f64>,      // Recent performance on each task
    focus_weights: Vec<f64>,         // Current attention weights
}

impl CurriculumScheduler {
    pub fn new(num_tasks: usize) -> Self {
        Self {
            task_difficulties: vec![0.5; num_tasks],
            task_performance: vec![0.5; num_tasks],
            focus_weights: vec![1.0 / num_tasks as f64; num_tasks],
        }
    }

    /// Meta-learning neuron decides what to focus on
    /// Tasks with low performance but learnable difficulty get priority
    pub fn update_curriculum(&mut self, task_losses: &[f64]) {
        for (i, &loss) in task_losses.iter().enumerate() {
            // Update performance (exponential moving average)
            self.task_performance[i] = 0.9 * self.task_performance[i] + 0.1 * (1.0 - loss);
            
            // Update difficulty estimate
            self.task_difficulties[i] = loss;
            
            // Focus weight: prioritize tasks that are:
            // 1. Underperforming (low performance)
            // 2. But not impossibly hard (medium difficulty)
            let learning_potential = (1.0 - self.task_performance[i]) * 
                                    (1.0 - (self.task_difficulties[i] - 0.5).abs());
            self.focus_weights[i] = learning_potential;
        }

        // Normalize focus weights
        let total: f64 = self.focus_weights.iter().sum();
        if total > 0.0 {
            for w in &mut self.focus_weights {
                *w /= total;
            }
        }
    }

    pub fn get_focus_weights(&self) -> &[f64] {
        &self.focus_weights
    }

    pub fn get_priority_task(&self) -> (usize, f64) {
        let (idx, &weight) = self.focus_weights.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();
        (idx, weight)
    }
}

/// Advanced optimizer: Adam-like with momentum and adaptive learning rates
pub struct AdamOptimizer {
    m_w1: Array2<f64>,  // First moment (momentum)
    v_w1: Array2<f64>,  // Second moment (variance)
    m_w2: Array2<f64>,
    v_w2: Array2<f64>,
    beta1: f64,
    beta2: f64,
    epsilon: f64,
    t: f64,  // Timestep
}

impl AdamOptimizer {
    pub fn new(w1_shape: (usize, usize), w2_shape: (usize, usize)) -> Self {
        Self {
            m_w1: Array2::zeros(w1_shape),
            v_w1: Array2::zeros(w1_shape),
            m_w2: Array2::zeros(w2_shape),
            v_w2: Array2::zeros(w2_shape),
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            t: 0.0,
        }
    }

    pub fn step(&mut self, w1_grad: &Array2<f64>, w2_grad: &Array2<f64>, lr: f64) 
        -> (Array2<f64>, Array2<f64>) 
    {
        self.t += 1.0;

        // Update W1
        self.m_w1 = &self.m_w1 * self.beta1 + w1_grad * (1.0 - self.beta1);
        self.v_w1 = &self.v_w1 * self.beta2 + &(w1_grad * w1_grad) * (1.0 - self.beta2);

        let m_hat_w1 = &self.m_w1 / (1.0 - self.beta1.powf(self.t));
        let v_hat_w1 = &self.v_w1 / (1.0 - self.beta2.powf(self.t));

        let w1_update = &m_hat_w1 / &(v_hat_w1.mapv(|v| v.sqrt() + self.epsilon)) * lr;

        // Update W2
        self.m_w2 = &self.m_w2 * self.beta1 + w2_grad * (1.0 - self.beta1);
        self.v_w2 = &self.v_w2 * self.beta2 + &(w2_grad * w2_grad) * (1.0 - self.beta2);

        let m_hat_w2 = &self.m_w2 / (1.0 - self.beta1.powf(self.t));
        let v_hat_w2 = &self.v_w2 / (1.0 - self.beta2.powf(self.t));

        let w2_update = &m_hat_w2 / &(v_hat_w2.mapv(|v| v.sqrt() + self.epsilon)) * lr;

        (w1_update, w2_update)
    }
}
