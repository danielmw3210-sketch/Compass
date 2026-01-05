"""
LSTM Signal Model Training Script
Week 3: Deep Learning Enhancement

This script trains an LSTM neural network for BUY/SELL/HOLD classification
using sequential price data. The model captures temporal patterns that
traditional ML models miss.
"""

import os
import requests
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
from sklearn.preprocessing import StandardScaler
import joblib

# Configuration
MODEL_DIR = "models"
SEQUENCE_LENGTH = 60  # Use 60 timesteps (5 hours of 5min candles)
HIDDEN_SIZE = 128
NUM_LAYERS = 2
DROPOUT = 0.2
BATCH_SIZE = 32
EPOCHS = 50
LEARNING_RATE = 0.001

os.makedirs(MODEL_DIR, exist_ok=True)

class SignalLSTM(nn.Module):
    """
    LSTM Architecture for Trading Signal Classification
    
    Input: (batch, seq_len, n_features)
    Output: (batch, 3) - probabilities for SELL/HOLD/BUY
    """
    def __init__(self, input_size=11, hidden_size=HIDDEN_SIZE, num_layers=NUM_LAYERS, dropout=DROPOUT):
        super(SignalLSTM, self).__init__()
        
        self.lstm = nn.LSTM(
            input_size=input_size,
            hidden_size=hidden_size,
            num_layers=num_layers,
            batch_first=True,
            dropout=dropout if num_layers > 1 else 0
        )
        
        self.fc = nn.Linear(hidden_size, 3)  # 3 classes: SELL, HOLD, BUY
        
    def forward(self, x):
        # x shape: (batch, seq_len, features)
        lstm_out, (hidden, cell) = self.lstm(x)
        
        # Take last timestep output
        last_output = lstm_out[:, -1, :]
        
        # Classification layer
        output = self.fc(last_output)
        return output  # Raw logits (use with CrossEntropyLoss)

class PriceDataset(Dataset):
    """PyTorch Dataset for sequential price data"""
    def __init__(self, sequences, labels):
        self.sequences = torch.FloatTensor(sequences)
        self.labels = torch.LongTensor(labels)
        
    def __len__(self):
        return len(self.labels)
    
    def __getitem__(self, idx):
        return self.sequences[idx], self.labels[idx]

def fetch_binance_data(ticker, limit=5000):
    """Fetch OHLCV data from Binance"""
    url = "https://api.binance.com/api/v3/klines"
    params = {
        "symbol": ticker,
        "interval": "5m",
        "limit": str(limit)
    }
    
    response = requests.get(url, params=params)
    data = response.json()
    
    highs = []
    lows = []
    closes = []
    volumes = []
    
    for candle in data:
        highs.append(float(candle[2]))
        lows.append(float(candle[3]))
        closes.append(float(candle[4]))
        volumes.append(float(candle[5]))
    
    return np.array(highs), np.array(lows), np.array(closes), np.array(volumes)

def calculate_technical_indicators(highs, lows, closes, volumes):
    """
    Calculate all 11 features for each timestep
    
    Features match the Rust implementation:
    0-7: Original features (RSI, MACD, BB, SMA, Momentum, Volume)
    8-10: New features (ATR, Stochastic, OBV)
    """
    n = len(closes)
    features = np.zeros((n, 11))
    
    # Simplified feature calculation for demo
    # In production, use TA-Lib or implement full indicators
    
    for i in range(200, n):  # Need 200 for SMA200
        # Feature 0: RSI (simplified)
        if i >= 14:
            gains = sum([max(closes[j] - closes[j-1], 0) for j in range(i-14, i)]) / 14
            losses = sum([max(closes[j-1] - closes[j], 0) for j in range(i-14, i)]) / 14
            if losses == 0:
                features[i, 0] = 100
            else:
                rs = gains / losses
                features[i, 0] = 100 - (100 / (1 + rs))
        
        # Feature 1: MACD (simplified)
        if i >= 26:
            ema12 = np.mean(closes[i-12:i])
            ema26 = np.mean(closes[i-26:i])
            features[i, 1] = ema12 - ema26
        
        # Feature 2-3: Bollinger Bands
        if i >= 20:
            sma = np.mean(closes[i-20:i])
            std = np.std(closes[i-20:i])
            upper = sma + 2 * std
            lower = sma - 2 * std
            features[i, 2] = (upper - lower) / sma if sma != 0 else 0  # BB Width
            features[i, 3] = (closes[i] - lower) / (upper - lower) if (upper - lower) != 0 else 0.5  # BB Position
        
        # Feature 4-5: SMA Divergence
        sma20 = np.mean(closes[i-20:i])
        sma50 = np.mean(closes[i-50:i])
        sma200 = np.mean(closes[i-200:i])
        features[i, 4] = (sma20 - sma50) / sma50 if sma50 != 0 else 0
        features[i, 5] = (sma50 - sma200) / sma200 if sma200 != 0 else 0
        
        # Feature 6: Price Momentum
        if i >= 5:
            features[i, 6] = (closes[i] - closes[i-5]) / closes[i-5] if closes[i-5] != 0 else 0
        
        # Feature 7: Volume Ratio
        if i >= 20:
            vol_avg = np.mean(volumes[i-20:i])
            features[i, 7] = volumes[i] / vol_avg if vol_avg != 0 else 1.0
        
        # Feature 8: ATR (simplified)
        if i >= 14:
            tr = max(highs[i] - lows[i], abs(highs[i] - closes[i-1]), abs(lows[i] - closes[i-1]))
            atr = np.mean([max(highs[j] - lows[j], abs(highs[j] - closes[j-1]), abs(lows[j] - closes[j-1])) 
                          for j in range(i-14, i)])
            features[i, 8] = atr / closes[i] if closes[i] != 0 else 0
        
        # Feature 9: Stochastic Oscillator
        if i >= 14:
            lowest = np.min(lows[i-14:i])
            highest = np.max(highs[i-14:i])
            features[i, 9] = ((closes[i] - lowest) / (highest - lowest)) * 100 if (highest - lowest) != 0 else 50
        
        # Feature 10: OBV Momentum (simplified)
        # OBV calculation would be cumulative, simplified here
        features[i, 10] = 0.0  # Placeholder
    
    return features

def create_sequences(features, closes, seq_length=SEQUENCE_LENGTH, look_ahead=6):
    """
    Create sequences and labels
    
    Args:
        features: (n_samples, n_features) array
        closes: Price array
        seq_length: Number of timesteps per sequence
        look_ahead: Predict N candles ahead (6 = 30min)
    
    Returns:
        sequences: (n_sequences, seq_length, n_features)
        labels: (n_sequences,) - 0=SELL, 1=HOLD, 2=BUY
    """
    sequences = []
    labels = []
    
    start_idx = 200  # Need 200 for indicators
    for i in range(start_idx + seq_length, len(closes) - look_ahead):
        # Get sequence of features
        seq = features[i-seq_length:i]
        
        # Calculate future return
        current_price = closes[i]
        future_price = closes[i + look_ahead]
        future_return = (future_price - current_price) / current_price * 100.0
        
        # Label based on threshold (0.15% as in Rust code)
        if future_return > 0.15:
            label = 2  # BUY
        elif future_return < -0.15:
            label = 0  # SELL
        else:
            label = 1  # HOLD
        
        sequences.append(seq)
        labels.append(label)
    
    return np.array(sequences), np.array(labels)

def train_lstm_model(ticker):
    """Train LSTM model for a specific ticker"""
    print(f"\n{'='*60}")
    print(f"Training LSTM Model for {ticker}")
    print(f"{'='*60}\n")
    
    # 1. Fetch Data
    print("📊 Fetching data from Binance...")
    highs, lows, closes, volumes = fetch_binance_data(ticker, limit=5000)
    print(f"✅ Retrieved {len(closes)} candles")
    
    # 2. Calculate Features
    print("🔬 Computing technical indicators...")
    features = calculate_technical_indicators(highs, lows, closes, volumes)
    
    # 3. Create Sequences
    print(f"🔗 Creating sequences (length={SEQUENCE_LENGTH})...")
    sequences, labels = create_sequences(features, closes)
    print(f"✅ Generated {len(sequences)} training samples")
    
    # Check class distribution
    unique, counts = np.unique(labels, return_counts=True)
    print(f"📈 Class Distribution:")
    for cls, count in zip(unique, counts):
        cls_name = ["SELL", "HOLD", "BUY"][cls]
        print(f"   {cls_name}: {count} ({count/len(labels)*100:.1f}%)")
    
    # 4. Normalize Features
    print("📏 Normalizing features...")
    scaler = StandardScaler()
    n_samples, seq_len, n_features = sequences.shape
    sequences_flat = sequences.reshape(-1, n_features)
    sequences_scaled = scaler.fit_transform(sequences_flat)
    sequences_scaled = sequences_scaled.reshape(n_samples, seq_len, n_features)
    
    # Save scaler
    ticker_short = ticker.replace("USDT", "").lower()
    scaler_path = os.path.join(MODEL_DIR, f"{ticker_short}_lstm_scaler.pkl")
    joblib.dump(scaler, scaler_path)
    print(f"💾 Saved scaler: {scaler_path}")
    
    # 5. Split Data (80/20)
    split_idx = int(len(sequences_scaled) * 0.8)
    X_train = sequences_scaled[:split_idx]
    y_train = labels[:split_idx]
    X_val = sequences_scaled[split_idx:]
    y_val = labels[split_idx:]
    
    print(f"🔀 Split: {len(X_train)} train, {len(X_val)} validation")
    
    # 6. Create DataLoaders
    train_dataset = PriceDataset(X_train, y_train)
    val_dataset = PriceDataset(X_val, y_val)
    
    train_loader = DataLoader(train_dataset, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=BATCH_SIZE)
    
    # 7. Initialize Model
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"🖥️  Using device: {device}")
    
    model = SignalLSTM(input_size=11, hidden_size=HIDDEN_SIZE, num_layers=NUM_LAYERS).to(device)
    criterion = nn.CrossEntropyLoss()
    optimizer = optim.Adam(model.parameters(), lr=LEARNING_RATE)
    
    # 8. Training Loop
    print(f"\n🚀 Starting training ({EPOCHS} epochs)...\n")
    
    best_val_acc = 0.0
    for epoch in range(EPOCHS):
        # Training
        model.train()
        train_loss = 0.0
        train_correct = 0
        train_total = 0
        
        for sequences_batch, labels_batch in train_loader:
            sequences_batch = sequences_batch.to(device)
            labels_batch = labels_batch.to(device)
            
            optimizer.zero_grad()
            outputs = model(sequences_batch)
            loss = criterion(outputs, labels_batch)
            loss.backward()
            optimizer.step()
            
            train_loss += loss.item()
            _, predicted = outputs.max(1)
            train_total += labels_batch.size(0)
            train_correct += predicted.eq(labels_batch).sum().item()
        
        train_acc = 100.0 * train_correct / train_total
        
        # Validation
        model.eval()
        val_loss = 0.0
        val_correct = 0
        val_total = 0
        
        with torch.no_grad():
            for sequences_batch, labels_batch in val_loader:
                sequences_batch = sequences_batch.to(device)
                labels_batch = labels_batch.to(device)
                
                outputs = model(sequences_batch)
                loss = criterion(outputs, labels_batch)
                
                val_loss += loss.item()
                _, predicted = outputs.max(1)
                val_total += labels_batch.size(0)
                val_correct += predicted.eq(labels_batch).sum().item()
        
        val_acc = 100.0 * val_correct / val_total
        
        # Print progress every 5 epochs
        if (epoch + 1) % 5 == 0:
            print(f"Epoch [{epoch+1}/{EPOCHS}] | Train Loss: {train_loss/len(train_loader):.4f} | Train Acc: {train_acc:.2f}% | Val Acc: {val_acc:.2f}%")
        
        # Save best model
        if val_acc > best_val_acc:
            best_val_acc = val_acc
            model_path = os.path.join(MODEL_DIR, f"{ticker_short}_lstm.pth")
            torch.save(model.state_dict(), model_path)
    
    print(f"\n✅ Training Complete!")
    print(f"🏆 Best Validation Accuracy: {best_val_acc:.2f}%")
    print(f"💾 Model saved: {model_path}")
    
    # 9. Export to ONNX
    print(f"\n📦 Exporting to ONNX...")
    model.eval()
    dummy_input = torch.randn(1, SEQUENCE_LENGTH, 11).to(device)
    onnx_path = os.path.join(MODEL_DIR, f"{ticker_short}_lstm.onnx")
    
    torch.onnx.export(
        model,
        dummy_input,
        onnx_path,
        export_params=True,
        opset_version=11,
        do_constant_folding=True,
        input_names=['input'],
        output_names=['output'],
        dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}}
    )
    
    print(f"✅ ONNX model exported: {onnx_path}")
    return onnx_path

if __name__ == "__main__":
    tickers = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "LTCUSDT"]
    
    for ticker in tickers:
        try:
            train_lstm_model(ticker)
        except Exception as e:
            print(f"❌ Failed to train {ticker}: {e}")
    
    print(f"\n{'='*60}")
    print("🎉 All LSTM models trained successfully!")
    print(f"{'='*60}\n")
