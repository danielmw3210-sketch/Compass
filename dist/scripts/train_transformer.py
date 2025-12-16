import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
import pandas as pd
import requests
import json
import os
import math

# === Configuration ===
SYMBOL = "BTCUSDT"
INTERVAL = "1h" 
LIMIT = 2000    # More data for Transformer
SEQ_LENGTH = 60 # Look back 60 hours
D_MODEL = 64    # Embedding dimension
NHEAD = 4       # Attention heads
NUM_LAYERS = 2  # Transformer layers
EPOCHS = 50
MODEL_PATH = "models/model_transformer_v1.onnx"

# === 1. Data Fetching (Enhanced) ===
def fetch_binance_data():
    url = "https://api.binance.com/api/v3/klines"
    params = {"symbol": SYMBOL, "interval": INTERVAL, "limit": LIMIT}
    print(f"Dataset: Fetching {LIMIT} candles ({INTERVAL}) for {SYMBOL}...")
    
    try:
        resp = requests.get(url, params=params)
        resp.raise_for_status()
        data = resp.json()
        df = pd.DataFrame(data, columns=["Open Time", "Open", "High", "Low", "Close", "Volume", "Close Time", "QAV", "Num Trades", "Taker Buy Base", "Taker Buy Quote", "Ignore"])
        
        df["Close"] = df["Close"].astype(float)
        df["Volume"] = df["Volume"].astype(float)
        
        # Simple RSI Feature
        delta = df["Close"].diff()
        gain = (delta.where(delta > 0, 0)).rolling(window=14).mean()
        loss = (-delta.where(delta < 0, 0)).rolling(window=14).mean()
        rs = gain / loss
        df["RSI"] = 100 - (100 / (1 + rs))
        df["RSI"] = df["RSI"].fillna(50) # Fill NaN with neutral RSI
        
        return df[["Close", "Volume", "RSI"]].values # Shape: (N, 3)
    except Exception as e:
        print(f"❌ Error fetching data: {e}")
        return np.random.rand(LIMIT, 3)

# === 2. Preprocessing ===
class StandardScaler:
    def fit_transform(self, data):
        self.mean = data.mean(axis=0)
        self.std = data.std(axis=0) + 1e-8
        return (data - self.mean) / self.std

    def inverse_transform(self, data):
        # We only need to unscale the Price (index 0)
        return (data * self.std[0]) + self.mean[0]

def create_sequences(data, seq_length):
    xs, ys = [], []
    for i in range(len(data) - seq_length):
        x = data[i:i+seq_length]
        y = data[i+seq_length][0] 
        xs.append(x)
        ys.append(y)
    return np.array(xs), np.array(ys)

# === 3. Transformer Model ===
class PositionalEncoding(nn.Module):
    def __init__(self, d_model, max_len=5000):
        super(PositionalEncoding, self).__init__()
        pe = torch.zeros(max_len, d_model)
        position = torch.arange(0, max_len, dtype=torch.float).unsqueeze(1)
        div_term = torch.exp(torch.arange(0, d_model, 2).float() * (-math.log(10000.0) / d_model))
        pe[:, 0::2] = torch.sin(position * div_term)
        pe[:, 1::2] = torch.cos(position * div_term)
        pe = pe.unsqueeze(0).transpose(0, 1)
        self.register_buffer('pe', pe)

    def forward(self, x):
        return x + self.pe[:x.size(0), :]

class TimeSeriesTransformer(nn.Module):
    def __init__(self, input_size=3, d_model=64, nhead=4, num_layers=2):
        super(TimeSeriesTransformer, self).__init__()
        # Input Embedding: Map 3 features -> 64 dim
        self.embedding = nn.Linear(input_size, d_model)
        self.pos_encoder = PositionalEncoding(d_model)
        
        encoder_layers = nn.TransformerEncoderLayer(d_model=d_model, nhead=nhead, batch_first=True)
        self.transformer_encoder = nn.TransformerEncoder(encoder_layers, num_layers=num_layers)
        
        self.decoder = nn.Linear(d_model, 1) # Predict Price

    def forward(self, x):
        # x: (Batch, Seq_Len, Features)
        x = self.embedding(x)
        x = x * math.sqrt(D_MODEL) 
        # For batch_first=True, we don't need to transpose for pos_encoder if we adjust usage, 
        # but typical PosEncoding expects (Seq, Batch, Dim). Let's be careful.
        # Actually PyTorch Transformer batch_first=True handles (Batch, Seq, Dim).
        # Our PosEncoder above expects (Seq, Batch, Dim). Keep it simple for now:
        
        x = self.pos_encoder(x.permute(1, 0, 2)).permute(1, 0, 2)
        
        output = self.transformer_encoder(x)
        
        # Take the last time step for prediction
        last_step = output[:, -1, :]
        prediction = self.decoder(last_step)
        return prediction

# === 4. Training & Export ===
def main():
    # A. Data
    raw_data = fetch_binance_data()
    scaler = StandardScaler()
    scaled_data = scaler.fit_transform(raw_data)
    
    X, y = create_sequences(scaled_data, SEQ_LENGTH)
    
    X_tensor = torch.FloatTensor(X)
    y_tensor = torch.FloatTensor(y).view(-1, 1)
    
    # B. Train
    print("[INFO] Initializing Transformer (2025 Architecture)...")
    model = TimeSeriesTransformer(input_size=3, d_model=D_MODEL, nhead=NHEAD, num_layers=NUM_LAYERS)
    criterion = nn.MSELoss()
    optimizer = optim.Adam(model.parameters(), lr=0.0005)
    
    print("[INFO] Training...")
    for epoch in range(EPOCHS):
        model.train()
        outputs = model(X_tensor)
        loss = criterion(outputs, y_tensor)
        
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        
        if (epoch+1) % 10 == 0:
            print(f"Epoch [{epoch+1}/{EPOCHS}], Loss: {loss.item():.6f}")

    print("✅ Training Complete.")

    # C. Export
    if not os.path.exists("models"):
        os.makedirs("models")
        
    dummy_input = torch.randn(1, SEQ_LENGTH, 3) 
    
    # Export with dynamic axes for batch size
    torch.onnx.export(model, 
                      dummy_input, 
                      MODEL_PATH, 
                      input_names=['input'], 
                      output_names=['output'],
                      dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}})
                      
    print(f"💾 Model MINTED: {MODEL_PATH}")

if __name__ == "__main__":
    main()
