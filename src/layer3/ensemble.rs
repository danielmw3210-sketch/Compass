//! Model Ensemble System (Week 4 Enhancement)
//! 
//! Combines predictions from multiple models using weighted voting:
//! - Random Forest: Fast baseline (20% weight)
//! - LSTM: Temporal patterns (40% weight)
//! - Could add LightGBM when available (40% weight)

use std::error::Error;
use std::collections::HashMap;

/// Ensemble prediction combining multiple models
pub struct ModelEnsemble {
    weights: HashMap<String, f64>,
    performance_history: HashMap<String, Vec<bool>>, // Track recent predictions
    adaptive_weights: bool,
}

impl ModelEnsemble {
    /// Create new ensemble with default weights
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert("random_forest".to_string(), 0.5);  // 50% RF initially
        weights.insert("lstm".to_string(), 0.5);            // 50% LSTM initially
        // weights.insert("lightgbm".to_string(), 0.4);     // Add when available
        
        Self {
            weights,
            performance_history: HashMap::new(),
            adaptive_weights: true,
        }
    }
    
    /// Predict using weighted ensemble
    /// 
    /// Returns the final prediction (0=SELL, 1=HOLD, 2=BUY)
    pub fn predict_ensemble(
        &self,
        rf_prediction: u32,
        lstm_prediction: u32,
        // lgbm_prediction: Option<u32>, // Add when LightGBM available
    ) -> u32 {
        let rf_weight = self.weights.get("random_forest").unwrap_or(&0.5);
        let lstm_weight = self.weights.get("lstm").unwrap_or(&0.5);
        
        // Weighted voting
        let weighted_scores = [
            rf_prediction as f64 * rf_weight,
            lstm_prediction as f64 * lstm_weight,
        ];
        
        // Sum and round to nearest class
        let total: f64 = weighted_scores.iter().sum();
        let final_prediction = total.round() as u32;
        
        // Clamp to valid range [0, 2]
        final_prediction.min(2)
    }
    
    /// Update performance tracking
    pub fn record_prediction(&mut self, model_name: &str, correct: bool) {
        self.performance_history
            .entry(model_name.to_string())
            .or_insert_with(Vec::new)
            .push(correct);
        
        // Keep only last 100 predictions
        if let Some(history) = self.performance_history.get_mut(model_name) {
            if history.len() > 100 {
                history.remove(0);
            }
        }
    }
    
    /// Dynamically adjust weights based on recent performance
    pub fn update_weights(&mut self) {
        if !self.adaptive_weights {
            return;
        }
        
        let mut accuracies: HashMap<String, f64> = HashMap::new();
        
        // Calculate accuracy for each model
        for (model_name, history) in &self.performance_history {
            if history.len() >= 10 {  // Need at least 10 predictions
                let correct_count = history.iter().filter(|&&x| x).count();
                let accuracy = correct_count as f64 / history.len() as f64;
                accuracies.insert(model_name.clone(), accuracy);
            }
        }
        
        if accuracies.is_empty() {
            return;
        }
        
        // Normalize accuracies to sum to 1.0 (weights)
        let total_accuracy: f64 = accuracies.values().sum();
        
        if total_accuracy > 0.0 {
            for (model_name, accuracy) in accuracies {
                self.weights.insert(model_name, accuracy / total_accuracy);
            }
        }
    }
    
    /// Get current model weights
    pub fn get_weights(&self) -> HashMap<String, f64> {
        self.weights.clone()
    }
    
    /// Get model accuracy statistics
    pub fn get_stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        for (model_name, history) in &self.performance_history {
            if !history.is_empty() {
                let correct_count = history.iter().filter(|&&x| x).count();
                let accuracy = correct_count as f64 / history.len() as f64;
                stats.insert(model_name.clone(), accuracy);
            }
        }
        
        stats
    }
}

impl Default for ModelEnsemble {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function for ensemble prediction with fallback
/// 
/// This provides a robust prediction pipeline:
/// 1. Try ensemble (RF + LSTM)
/// 2. Fallback to RF if LSTM fails
/// 3. Default to HOLD if all fail
pub fn predict_with_fallback(
    ticker: &str,
    features: &[f64],
    ensemble: &ModelEnsemble,
) -> Result<u32, Box<dyn Error>> {
    // Try Random Forest
    let rf_pred = match crate::layer3::signal_model::predict_signal(ticker, features) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Random Forest prediction failed: {}", e);
            1  // Default to HOLD
        }
    };
    
    // Try LSTM (if ONNX model exists)
    let lstm_pred = match predict_lstm(ticker, features) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("LSTM prediction skipped: {}", e);
            rf_pred  // Use RF prediction as fallback
        }
    };
    
    // Ensemble vote
    let final_pred = ensemble.predict_ensemble(rf_pred, lstm_pred);
    
    Ok(final_pred)
}

/// LSTM prediction (placeholder - implement with tract-onnx)
/// 
/// TODO: In production, this should:
/// 1. Load ONNX model for the ticker
/// 2. Prepare 60-step sequence
/// 3. Run inference
/// 4. Return class prediction
fn predict_lstm(_ticker: &str, _features: &[f64]) -> Result<u32, Box<dyn Error>> {
    // Placeholder implementation
    // In production, use tract-onnx to run the LSTM ONNX model
    
    Err("LSTM inference not yet implemented - use tract-onnx".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ensemble_basic() {
        let ensemble = ModelEnsemble::new();
        
        // Both models predict BUY (2)
        let pred = ensemble.predict_ensemble(2, 2);
        assert_eq!(pred, 2);
        
        // RF=SELL(0), LSTM=BUY(2) -> Average = HOLD(1)
        let pred = ensemble.predict_ensemble(0, 2);
        assert_eq!(pred, 1);
    }
    
    #[test]
    fn test_adaptive_weights() {
        let mut ensemble = ModelEnsemble::new();
        
        // Simulate RF being more accurate
        for _ in 0..20 {
            ensemble.record_prediction("random_forest", true);
            ensemble.record_prediction("lstm", false);
        }
        
        ensemble.update_weights();
        
        let weights = ensemble.get_weights();
        let rf_weight = weights.get("random_forest").unwrap();
        let lstm_weight = weights.get("lstm").unwrap();
        
        // RF should have higher weight
        assert!(rf_weight > lstm_weight);
    }
    
    #[test]
    fn test_prediction_history() {
        let mut ensemble = ModelEnsemble::new();
        
        ensemble.record_prediction("random_forest", true);
        ensemble.record_prediction("random_forest", true);
        ensemble.record_prediction("random_forest", false);
        
        let stats = ensemble.get_stats();
        let accuracy = stats.get("random_forest").unwrap();
        
        assert!((accuracy - 0.666).abs() < 0.01);  // 2/3 = 66.6%
    }
}
