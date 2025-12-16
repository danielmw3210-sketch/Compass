import requests
import numpy as np
import pandas as pd
from sklearn.linear_model import LinearRegression
from skl2onnx import to_onnx
import os
import time

def fetch_historical_data(symbol="BTCUSDT", interval="1d", limit=180):
    """
    Fetches historical kline data from Binance API.
    """
    url = "https://api.binance.com/api/v3/klines"
    params = {
        "symbol": symbol,
        "interval": interval,
        "limit": limit
    }
    print(f"Fetching {limit} days of historical data for {symbol} from Binance...")
    try:
        response = requests.get(url, params=params)
        response.raise_for_status()
        data = response.json()
        # Binance kline structure: [Open Time, Open, High, Low, Close, Volume, ...]
        # We only care about Close price (index 4)
        closes = [float(x[4]) for x in data]
        return np.array(closes).reshape(-1, 1)
    except Exception as e:
        print(f"❌ Failed to fetch data: {e}")
        # Fallback to realistic dummy data if offline (BTC ~95k)
        print("Using fallback data for model generation.")
        return np.linspace(90000, 100000, limit).reshape(-1, 1)

def train_and_export_model():
    # 1. Get Data
    prices = fetch_historical_data()
    
    # 2. Prepare Training Data
    # Predict Price(t) based on Price(t-1)
    X = prices[:-1] # Inputs: Yesterday's price
    y = prices[1:]  # Targets: Today's price
    
    # 3. Train Model
    print("Training Linear Regression Model on Real Data...")
    model = LinearRegression()
    model.fit(X, y)
    
    score = model.score(X, y)
    print(f"Model Trained. R^2 Score: {score:.4f}")
    print(f"   Coefficient: {model.coef_[0][0]:.4f}")
    print(f"   Intercept: {model.intercept_[0]:.4f}")
    
    # 4. Convert to ONNX
    # Input is a float tensor of shape [None, 1]
    onx = to_onnx(model, X[:1].astype(np.float32), target_opset=12)
    
    # 5. Save
    output_path = "models/price_decision_v1.onnx"
    os.makedirs("models", exist_ok=True)
    
    with open(output_path, "wb") as f:
        f.write(onx.SerializeToString())
    
    print(f"Saved ONNX model to: {output_path}")
    
    # Also save to dist if exists
    if os.path.exists("dist/models"):
        with open("dist/models/price_decision_v1.onnx", "wb") as f:
            f.write(onx.SerializeToString())
        print(f"Copy saved to: dist/models/price_decision_v1.onnx")

if __name__ == "__main__":
    train_and_export_model()
