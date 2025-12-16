use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::Duration;
use tracing::{info, warn};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::linear::linear_regression::LinearRegression;
use smartcore::numbers::basenum::Number;
use smartcore::linear::linear_regression::LinearRegressionParameters;

#[derive(Clone)]
pub struct AutoTrainer {
    history: Arc<Mutex<VecDeque<f64>>>,
    // Correct SmartCore generics: <TX, TY, X, Y>
    model: Arc<Mutex<Option<LinearRegression<f64, f64, DenseMatrix<f64>, Vec<f64>>>>>,
    last_prediction: Arc<Mutex<f64>>,
}

impl AutoTrainer {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(VecDeque::new())),
            model: Arc::new(Mutex::new(None)),
            last_prediction: Arc::new(Mutex::new(0.0)),
        }
    }

    pub async fn start(&self) {
        let history = self.history.clone();
        let model = self.model.clone();
        let last_prediction = self.last_prediction.clone();

        tokio::spawn(async move {
            info!("🧠 Auto-Trainer Started: Continuous Learning Loop Active");
            
            loop {
                // 1. Fetch Price
                match fetch_price().await {
                    Ok(price) => {
                        let mut hist = history.lock().unwrap();
                        hist.push_back(price);
                        if hist.len() > 1000 {
                            hist.pop_front();
                        }
                        
                        // 2. Train if enough data
                        if hist.len() > 10 {
                            let data: Vec<f64> = hist.iter().cloned().collect();
                            
                            // Input: [t] (index), Target: [price]
                            // Simple trend following
                            let x_raw: Vec<Vec<f64>> = (0..data.len()).map(|i| vec![i as f64]).collect();
                            let y_raw: Vec<f64> = data.clone();
                            
                            let x = DenseMatrix::from_2d_vec(&x_raw);
                            let y = y_raw; 
                            
                            // Linear Regression
                            match LinearRegression::fit(&x, &y, LinearRegressionParameters::default()) {
                                Ok(lr) => {
                                    // Predict next step
                                    let next_idx = vec![vec![(data.len()) as f64]];
                                    let next_x = DenseMatrix::from_2d_vec(&next_idx);
                                    
                                    if let Ok(pred_vals) = lr.predict(&next_x) {
                                         let pred = pred_vals[0];
                                         info!("🧠 Enhanced Model Update: Trained on {} samples", data.len());
                                         info!("   🔮 Next BTC Prediction: ${:.2}", pred);
                                         *last_prediction.lock().unwrap() = pred;
                                    }
                                    
                                    *model.lock().unwrap() = Some(lr);
                                },
                                Err(e) => warn!("Training failed: {}", e),
                            }
                        }
                    },
                    Err(e) => warn!("Trainer failed to fetch price: {}", e),
                }

                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }
}

async fn fetch_price() -> Result<f64, String> {
    let url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";
    let resp = reqwest::get(url).await.map_err(|e| e.to_string())?
        .json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    
    resp["price"].as_str()
        .ok_or("No price field".to_string())?
        .parse::<f64>()
        .map_err(|e| e.to_string())
}
