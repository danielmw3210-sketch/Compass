import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
import pandas as pd
import requests
import json
import os

# === Configuration ===
SYMBOL = "BTCUSDT"
INTERVAL = "1h"  # 1 Hour candles for more granularity
LIMIT = 1000     # Last 1000 hours (~41 days)
SEQ_LENGTH = 30  # Look back 30 hours
HIDDEN_SIZE = 64
EPOCHS = 50
MODEL_PATH = "models/price_decision_v2.onnx"

# === 1. Data Fetching ===
def fetch_binance_data():
    url = "https://api.binance.com/api/v3/klines"
    params = {"symbol": SYMBOL, "interval": INTERVAL, "limit": LIMIT}
    print(f"Fetching {LIMIT} candles ({INTERVAL}) for {SYMBOL}...")
    
    try:
        resp = requests.get(url, params=params)
        resp.raise_for_status()
        data = resp.json()
        # [Open Time, Open, High, Low, Close, Volume, ...]
        # We want Close (index 4) and Volume (index 5)
        df = pd.DataFrame(data, columns=["Open Time", "Open", "High", "Low", "Close", "Volume", "Close Time", "QAV", "Num Trades", "Taker Buy Base", "Taker Buy Quote", "Ignore"])
        df["Close"] = df["Close"].astype(float)
        df["Volume"] = df["Volume"].astype(float)
        return df[["Close", "Volume"]].values # Shape: (N, 2)
    except Exception as e:
        print(f"❌ Error fetching data: {e}")
        # Fallback dummy data
        print("⚠️ Using dummy data for testing...")
        c = np.linspace(50000, 60000, LIMIT)
        v = np.random.rand(LIMIT) * 100
        return np.column_stack((c, v))

# === 2. Preprocessing ===
class StandardScaler:
    def fit_transform(self, data):
        self.mean = data.mean(axis=0)
        self.std = data.std(axis=0) + 1e-8
        return (data - self.mean) / self.std

    def inverse_transform(self, data):
        return (data * self.std) + self.mean

def create_sequences(data, seq_length):
    xs, ys = [], []
    for i in range(len(data) - seq_length):
        x = data[i:i+seq_length]
        y = data[i+seq_length][0] # Predict next Close Price (index 0)
        xs.append(x)
        ys.append(y)
    return np.array(xs), np.array(ys)

# === 3. Model Definition ===
class CryptoLSTM(nn.Module):
    def __init__(self, input_size=2, hidden_size=64, num_layers=2):
        super(CryptoLSTM, self).__init__()
        self.lstm = nn.LSTM(input_size, hidden_size, num_layers, batch_first=True)
        self.fc = nn.Linear(hidden_size, 1) # Output: Predicted Price

    def forward(self, x):
        # x shape: (Batch, Seq_Len, Features)
        out, _ = self.lstm(x)
        # Take the output from the last time step
        out = out[:, -1, :] 
        out = self.fc(out)
        return out

# === 4. Training & Export ===
def main():
    # A. Prepare Data
    raw_data = fetch_binance_data()
    scaler = StandardScaler()
    scaled_data = scaler.fit_transform(raw_data)
    
    X, y = create_sequences(scaled_data, SEQ_LENGTH)
    
    # Convert to Tensors
    X_tensor = torch.FloatTensor(X)
    y_tensor = torch.FloatTensor(y).view(-1, 1)
    
    # B. Train
    print("Training LSTM Brain...")
    model = CryptoLSTM(hidden_size=HIDDEN_SIZE)
    criterion = nn.MSELoss()
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    
    for epoch in range(EPOCHS):
        model.train()
        outputs = model(X_tensor)
        loss = criterion(outputs, y_tensor)
        
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        
        if (epoch+1) % 10 == 0:
            print(f"Epoch [{epoch+1}/{EPOCHS}], Loss: {loss.item():.6f}", flush=True)

    print("Training Complete.", flush=True)

    # C. Export to ONNX
    print("Exporting to ONNX...")
    dummy_input = torch.randn(1, SEQ_LENGTH, 2) # Batch=1, Seq=30, Feat=2
    
    os.makedirs("models", exist_ok=True)
    os.makedirs("dist/models", exist_ok=True) # Ensure dist exists too

    # Export
    try:
        dummy_input = torch.randn(1, SEQ_LENGTH, 2)
        # Simplify export: No dynamic axes for now, use opset 17
        torch.onnx.export(model, 
                          dummy_input, 
                          MODEL_PATH, 
                          input_names=['input'], 
                          output_names=['output'],
                          opset_version=17) 
        print(f"[INFO] Model SAVED: {MODEL_PATH}")
    except Exception as e:
        print(f"[ERROR] Export Failed: {e}")
    
    # Copy to dist
    import shutil
    shutil.copy(MODEL_PATH, "dist/models/price_decision_v2.onnx")
    
    # Save Scaler params for the Node (it needs to normalize inputs!)
    # Actually, Node will just feed raw data? No, Node feeds normalized data OR worker normalizes?
    # BETTER: Worker receives RAW data. Worker contains the scaler... OR model learns on raw? 
    # LSTM on raw prices (50k range) is unstable.
    # SOLUTION: Node Sends RAW. Worker Normalizes? 
    # PROBLEM: Worker needs the mean/std from Training Data.
    # FIX: Save scaler params to a JSON file. Worker loads JSON + ONNX.
    
    scaler_params = {
        "mean": scaler.mean.tolist(),
        "std": scaler.std.tolist()
    }
    with open("dist/models/scaler_params.json", "w") as f:
        json.dump(scaler_params, f)
    print("Scaler Params Saved to dist/models/scaler_params.json")

if __name__ == "__main__":
    main()
