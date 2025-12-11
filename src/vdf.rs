use sha2::{Digest, Sha256};

/// Represents the state of the VDF
#[derive(Debug, Clone)]
pub struct VDFState {
    pub current_hash: Vec<u8>,
    pub total_iterations: u64,
}

impl VDFState {
    pub fn new(seed: Vec<u8>) -> Self {
        VDFState {
            current_hash: seed,
            total_iterations: 0,
        }
    }

    /// Run the VDF for a specific number of iterations
    /// Returns the new hash
    pub fn execute(&mut self, iterations: u64) -> Vec<u8> {
        let mut hash = self.current_hash.clone();

        for _ in 0..iterations {
            let mut hasher = Sha256::new();
            hasher.update(&hash);
            hash = hasher.finalize().to_vec();
        }

        self.current_hash = hash.clone();
        self.total_iterations += iterations;
        hash
    }

    /// Static verification function
    pub fn verify(start_hash: &[u8], end_hash: &[u8], iterations: u64) -> bool {
        let mut hash = start_hash.to_vec();
        for _ in 0..iterations {
            let mut hasher = Sha256::new();
            hasher.update(&hash);
            hash = hasher.finalize().to_vec();
        }
        hash == end_hash
    }
}
