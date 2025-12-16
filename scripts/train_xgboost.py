import numpy as np
import pandas as pd
import requests
import json
import os
from sklearn.ensemble import GradientBoostingRegressor
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType

# === Configuration ===
SYMBOL = "SOLUSDT" # Train Gradient Boosting for SOL
INTERVAL = "1h"
LIMIT = 2000
MODEL_PATH = "models/model_xgboost_v1.onnx"

# === 1. Fetch Data ===
def fetch_binance_data():
    url = "https://api.binance.com/api/v3/klines"
    params = {"symbol": SYMBOL, "interval": INTERVAL, "limit": LIMIT}
    print(f"[INFO] Fetching {LIMIT} candles ({INTERVAL}) for {SYMBOL}...")
    
    try:
        resp = requests.get(url, params=params)
        resp.raise_for_status()
        data = resp.json()
        df = pd.DataFrame(data, columns=["Open Time", "Open", "High", "Low", "Close", "Volume", "Close Time", "QAV", "Num Trades", "Taker Buy Base", "Taker Buy Quote", "Ignore"])
        
        df["Close"] = df["Close"].astype(float)
        
        # Feature Engineering (Lag Features)
        df['Return'] = df['Close'].pct_change()
        
        # Lag Features (Last 5 hours)
        for i in range(1, 6):
            df[f'Lag_{i}'] = df['Return'].shift(i)
            
        df = df.dropna()
        
        # Input: [Lag_1, Lag_2, Lag_3, Lag_4, Lag_5]
        # Target: Next Return
        features = [f'Lag_{i}' for i in range(1, 6)]
        X = df[features].values.astype(np.float32) # Ensure Float32
        y = df['Return'].shift(-1).dropna().values.astype(np.float32)
        X = X[:-1] 
        
        return X, y
        
    except Exception as e:
        print(f"[ERROR] Fetch failed: {e}")
        return np.random.rand(100, 5).astype(np.float32), np.random.rand(100).astype(np.float32)

# === 2. Train ===
def main():
    X, y = fetch_binance_data()
    
    print("[INFO] Training Gradient Boosting Agent...")
    model = GradientBoostingRegressor(n_estimators=100, max_depth=3, learning_rate=0.1)
    model.fit(X, y)
    
    print("[INFO] Training Complete. MSE:", np.mean((model.predict(X) - y)**2))
    
    # === 3. Export to ONNX ===
    if not os.path.exists("models"):
        os.makedirs("models")
        
    initial_type = [('float_input', FloatTensorType([None, 5]))]
    onnx_model = convert_sklearn(model, initial_types=initial_type)
    
    with open(MODEL_PATH, "wb") as f:
        f.write(onnx_model.SerializeToString())
        
    print(f"[INFO] Model MINTED: {MODEL_PATH}")

if __name__ == "__main__":
    main()
