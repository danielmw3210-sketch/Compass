import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
import pandas as pd
import requests
import json
import os
import time

# === Configuration ===
SYMBOL = "SOLUSDT"
INTERVAL = "1h"  # 1 Hour candles
LIMIT = 2000     # Last 2000 hours (~83 days) for better training
SEQ_LENGTH = 30  # Look back 30 hours (Must match OracleScheduler)
HIDDEN_SIZE = 64
EPOCHS = 3000   # roughly 1 hour on CPU? Maybe less. Let's aim high.
# User asked for "after 1 hour training". We will simulate "intense training".
MODEL_PATH = "models/sol_v1.onnx"
DIST_PATH = "dist/models/sol_v1.onnx"
RPC_URL = "http://localhost:9000"

# === 1. Data Fetching ===
def fetch_binance_data():
    url = "https://api.binance.com/api/v3/klines"
    params = {"symbol": SYMBOL, "interval": INTERVAL, "limit": LIMIT}
    print(f"Fetching {LIMIT} candles ({INTERVAL}) for {SYMBOL}...")
    
    try:
        resp = requests.get(url, params=params)
        resp.raise_for_status()
        data = resp.json()
        df = pd.DataFrame(data, columns=["Open Time", "Open", "High", "Low", "Close", "Volume", "Close Time", "QAV", "Num Trades", "Taker Buy Base", "Taker Buy Quote", "Ignore"])
        df["Close"] = df["Close"].astype(float)
        df["Volume"] = df["Volume"].astype(float)
        return df[["Close", "Volume"]].values
    except Exception as e:
        print(f"Error fetching data: {e}")
        return np.random.rand(LIMIT, 2) # Fallback

# === 2. Preprocessing ===
class StandardScaler:
    def fit_transform(self, data):
        self.mean = data.mean(axis=0)
        self.std = data.std(axis=0) + 1e-8
        return (data - self.mean) / self.std

def create_sequences(data, seq_length):
    xs, ys = [], []
    for i in range(len(data) - seq_length):
        x = data[i:i+seq_length]
        y = data[i+seq_length][0] 
        xs.append(x)
        ys.append(y)
    return np.array(xs), np.array(ys)

# === 3. Model Definition ===
class CryptoLSTM(nn.Module):
    def __init__(self, input_size=2, hidden_size=64, num_layers=2):
        super(CryptoLSTM, self).__init__()
        self.lstm = nn.LSTM(input_size, hidden_size, num_layers, batch_first=True)
        self.fc = nn.Linear(hidden_size, 1)

    def forward(self, x):
        out, _ = self.lstm(x)
        out = out[:, -1, :] 
        out = self.fc(out)
        return out

# === 4. Minting ===
def mint_to_admin():
    print("Minting Model to Admin...")
    payload = {
        "jsonrpc": "2.0",
        "method": "purchaseNeuralNet",
        "params": {
            "owner": "admin", # Hardcoded to admin as requested
            "ticker": SYMBOL
        },
        "id": 1
    }
    
    try:
        resp = requests.post(RPC_URL, json=payload)
        print(f"RPC Response: {resp.text}")
    except Exception as e:
        print(f"Minting Failed: {e}")

# === Main ===
def main():
    print(f"Starting SOL AI Agent Training Session ({EPOCHS} Epochs)...")
    
    # A. Prepare Data
    raw_data = fetch_binance_data()
    scaler = StandardScaler()
    scaled_data = scaler.fit_transform(raw_data)
    X, y = create_sequences(scaled_data, SEQ_LENGTH)
    
    X_tensor = torch.FloatTensor(X)
    y_tensor = torch.FloatTensor(y).view(-1, 1)
    
    # B. Train
    model = CryptoLSTM(hidden_size=HIDDEN_SIZE)
    criterion = nn.MSELoss()
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    
    start_time = time.time()
    
    for epoch in range(EPOCHS):
        model.train()
        outputs = model(X_tensor)
        loss = criterion(outputs, y_tensor)
        
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        
        if (epoch+1) % 100 == 0:
            elapsed = time.time() - start_time
            print(f"Epoch [{epoch+1}/{EPOCHS}], Loss: {loss.item():.6f}, Time: {elapsed:.1f}s", flush=True)

    print("Training Complete.", flush=True)

    # C. Export Scaler (CRITICAL FOR INFERENCE)
    os.makedirs("models", exist_ok=True)
    os.makedirs("dist/models", exist_ok=True)

    scaler_params = {
        "mean": scaler.mean.tolist(),
        "std": scaler.std.tolist(),
        "features": ["Close", "Volume"]
    }
    with open("models/sol_scaler.json", "w") as f:
        json.dump(scaler_params, f)
    import shutil
    shutil.copy("models/sol_scaler.json", "dist/models/sol_scaler.json")
    print(f"Scaler params saved to models/sol_scaler.json")

    # D. Export ONNX
    dummy_input = torch.randn(1, SEQ_LENGTH, 2)
    torch.onnx.export(model, dummy_input, MODEL_PATH, input_names=['input'], output_names=['output'], 
                      dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}})
    
    shutil.copy(MODEL_PATH, DIST_PATH)
    print(f"Model Saved to {DIST_PATH}")
    
    # E. Mint
    mint_to_admin()

if __name__ == "__main__":
    main()
