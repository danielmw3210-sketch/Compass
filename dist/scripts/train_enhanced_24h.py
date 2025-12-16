import os
import time
import json
import numpy as np
import requests
from sklearn.neural_network import MLPRegressor
from sklearn.preprocessing import StandardScaler
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType
import joblib

# Configuration
MODEL_DIR = "dist/models"
MODEL_NAME = "price_decision_v1" # Overwriting the active model to "Upgrade" it
ONNX_PATH = os.path.join(MODEL_DIR, f"{MODEL_NAME}.onnx")
DURATION_HOURS = 24
INTERVAL_SECONDS = 60

# Ensure directories exist
os.makedirs(MODEL_DIR, exist_ok=True)

# State
history_prices = []
scaler = StandardScaler()
# Enhanced Model: Multi-Layer Perceptron (Neural Network)
# Hidden Layers: 100 neurons, 50 neurons. ReLU activation.
model = MLPRegressor(hidden_layer_sizes=(100, 50), activation='relu', solver='adam', max_iter=500, warm_start=True)
is_fitted = False

def fetch_price():
    try:
        url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT"
        resp = requests.get(url, timeout=5)
        data = resp.json()
        return float(data['price'])
    except Exception as e:
        print(f"⚠️ Error fetching price: {e}")
        return None

def train_step():
    global is_fitted
    
    # 1. Fetch Data
    price = fetch_price()
    if price is None:
        return

    print(f"📈 Fetched BTC Price: ${price:.2f}")
    history_prices.append(price)
    
    # Keep window manageable for proof-of-concept (e.g., last 1000 points)
    if len(history_prices) > 1000:
        history_prices.pop(0)

    # Need at least 2 points to train
    if len(history_prices) < 5:
        print(f"⏳ Collecting data... ({len(history_prices)}/5)")
        return

    # 2. Prepare Iterative Batch
    # Input: Price at T
    # Target: Price at T+1 (Self-Supervised / Next Token Prediction)
    X = np.array(history_prices[:-1]).reshape(-1, 1)
    y = np.array(history_prices[1:])

    # 3. Train (Incremental)
    print(f"🧠 Training Enhanced Model (Epoch {int(time.time())})...")
    X_scaled = scaler.fit_transform(X) # Note: Scaling changes every step, imperfect for online but okay for demo
    
    try:
        model.partial_fit(X_scaled, y)
        is_fitted = True
        score = model.score(X_scaled, y)
        print(f"✅ Training Complete. R² Score: {score:.4f}")
    except Exception as e:
        # First fit needs full fit? partial_fit handles it if classes usually provided, but regressor is easier
        model.fit(X_scaled, y)
        is_fitted = True
        print(f"✅ Initial Training Complete.")

    # 4. Export to ONNX
    # We export ONLY if fitted
    if is_fitted:
        try:
            initial_type = [('float_input', FloatTensorType([None, 1]))]
            onnx_model = convert_sklearn(model, initial_types=initial_type)
            with open(ONNX_PATH, "wb") as f:
                f.write(onnx_model.SerializeToString())
            print(f"💾 Saved Enhanced Model to {ONNX_PATH}")
        except Exception as e:
            print(f"❌ Failed to export ONNX: {e}")

def main():
    print(f"🚀 Starting 24-Hour Enhanced AI Training Cycle")
    print(f"🎯 Target Model: {MODEL_NAME}")
    print(f"⏱️ Duration: {DURATION_HOURS} Hours")
    print("--------------------------------------------------")

    start_time = time.time()
    end_time = start_time + (DURATION_HOURS * 3600)

    while time.time() < end_time:
        train_step()
        
        # Calculate progress
        elapsed = time.time() - start_time
        remaining = end_time - time.time()
        print(f"⏱️ Elapsed: {elapsed/3600:.2f}h | Remaining: {remaining/3600:.2f}h")
        print("--------------------------------------------------")
        
        time.sleep(INTERVAL_SECONDS)

if __name__ == "__main__":
    main()
