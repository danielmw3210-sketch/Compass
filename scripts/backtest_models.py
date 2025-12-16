import torch
import numpy as np
import pandas as pd
import requests
import json
import os
import sys
from pathlib import Path

# Fix Unicode encoding issue for Windows
if sys.platform == 'win32':
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# === Configuration ===
ASSETS = ["BTC", "ETH", "SOL", "LTC"]
SYMBOLS = {"BTC": "BTCUSDT", "ETH": "ETHUSDT", "SOL": "SOLUSDT", "LTC": "LTCUSDT"}
INTERVAL = "1h"
TEST_CANDLES = 500  # Out-of-sample data for testing
SEQ_LENGTH = 30  # Must match training

# === LSTM Model Definition (must match training) ===
class CryptoLSTM(torch.nn.Module):
    def __init__(self, input_size=2, hidden_size=64, num_layers=2):
        super(CryptoLSTM, self).__init__()
        self.lstm = torch.nn.LSTM(input_size, hidden_size, num_layers, batch_first=True)
        self.fc = torch.nn.Linear(hidden_size, 1)

    def forward(self, x):
        out, _ = self.lstm(x)
        out = out[:, -1, :]
        out = self.fc(out)
        return out

# === Data Fetching ===
def fetch_test_data(symbol, limit=TEST_CANDLES):
    """Fetch recent candles for backtesting"""
    url = "https://api.binance.com/api/v3/klines"
    params = {"symbol": symbol, "interval": INTERVAL, "limit": limit}
    
    try:
        resp = requests.get(url, params=params, timeout=10)
        resp.raise_for_status()
        data = resp.json()
        df = pd.DataFrame(data, columns=["Open Time", "Open", "High", "Low", "Close", "Volume", 
                                         "Close Time", "QAV", "Num Trades", "Taker Buy Base", 
                                         "Taker Buy Quote", "Ignore"])
        df["Close"] = df["Close"].astype(float)
        df["Volume"] = df["Volume"].astype(float)
        return df[["Close", "Volume"]].values
    except Exception as e:
        print(f"Error fetching data for {symbol}: {e}")
        return None

# === Strategy 1: AI Model ===
def ai_strategy(asset, data):
    """Load ONNX model and run inference"""
    import onnxruntime as ort
    
    model_path = f"dist/models/{asset.lower()}_v1.onnx"
    scaler_path = f"dist/models/{asset.lower()}_scaler.json"
    
    if not os.path.exists(model_path):
        print(f"  [ERROR] Model not found: {model_path}")
        return None, None
    
    if not os.path.exists(scaler_path):
        print(f"  [ERROR] Scaler not found: {scaler_path}")
        return None, None
    
    # Load scaler
    with open(scaler_path, 'r') as f:
        scaler_params = json.load(f)
    
    mean = np.array(scaler_params['mean'])
    std = np.array(scaler_params['std'])
    
    # Scale data
    scaled_data = (data - mean) / std
    
    # Load ONNX model
    try:
        session = ort.InferenceSession(model_path)
        input_name = session.get_inputs()[0].name
    except Exception as e:
        print(f"  [ERROR] Failed to load ONNX model: {e}")
        return None, None
    
    predictions = []
    actuals = []
    
    for i in range(SEQ_LENGTH, len(scaled_data) - 1):
        sequence = scaled_data[i-SEQ_LENGTH:i]
        
        # Prepare input (batch_size=1, seq_length=30, features=2)
        input_data = sequence.reshape(1, SEQ_LENGTH, 2).astype(np.float32)
        
        # Run inference
        try:
            output = session.run(None, {input_name: input_data})
            predicted_value = output[0][0][0]
            
            # Compare prediction to current price to determine direction
            current_price_scaled = scaled_data[i, 0]
            predicted_direction = 1 if predicted_value > current_price_scaled else -1
        except Exception as e:
            # Fallback if inference fails
            predicted_direction = 0
        
        # Actual direction
        actual_direction = 1 if data[i+1, 0] > data[i, 0] else -1
        
        predictions.append(predicted_direction)
        actuals.append(actual_direction)
    
    return predictions, actuals

# === Strategy 2: Heuristic (SMA Momentum) ===
def heuristic_strategy(data):
    """Simple Moving Average momentum strategy"""
    predictions = []
    actuals = []
    
    for i in range(30, len(data) - 1):
        # SMA crossover
        sma_short = np.mean(data[i-5:i, 0])
        sma_long = np.mean(data[i-20:i, 0])
        
        predicted_direction = 1 if sma_short > sma_long else -1
        actual_direction = 1 if data[i+1, 0] > data[i, 0] else -1
        
        predictions.append(predicted_direction)
        actuals.append(actual_direction)
    
    return predictions, actuals

# === Strategy 3: Buy and Hold ===
def buyhold_strategy(data):
    """Always predict UP (buy and hold)"""
    predictions = []
    actuals = []
    
    for i in range(30, len(data) - 1):
        predicted_direction = 1  # Always buy
        actual_direction = 1 if data[i+1, 0] > data[i, 0] else -1
        
        predictions.append(predicted_direction)
        actuals.append(actual_direction)
    
    return predictions, actuals

# === Metrics Calculation ===
def calculate_metrics(predictions, actuals, strategy_name, asset):
    """Calculate accuracy and other metrics"""
    if predictions is None or len(predictions) == 0:
        return None
    
    correct = sum([1 for p, a in zip(predictions, actuals) if p == a])
    accuracy = (correct / len(predictions)) * 100
    
    # Simple return simulation
    returns = []
    for p, a in zip(predictions, actuals):
        if p == a:
            returns.append(0.01)  # 1% gain when correct
        else:
            returns.append(-0.01)  # 1% loss when wrong
    
    total_return = sum(returns) * 100
    
    return {
        "asset": asset,
        "strategy": strategy_name,
        "accuracy": round(accuracy, 2),
        "total_return": round(total_return, 2),
        "num_predictions": len(predictions)
    }

# === Main Backtesting ===
def main():
    print("=" * 60)
    print("AI Model Backtesting Framework")
    print("=" * 60)
    print()
    
    results = []
    
    for asset in ASSETS:
        symbol = SYMBOLS[asset]
        print(f"[{asset}] Fetching test data...")
        
        data = fetch_test_data(symbol)
        if data is None:
            print(f"  [SKIP] Failed to fetch data")
            continue
        
        print(f"  Fetched {len(data)} candles")
        
        # Run AI Strategy
        print(f"  Running AI Strategy...")
        ai_pred, ai_actual = ai_strategy(asset, data)
        ai_metrics = calculate_metrics(ai_pred, ai_actual, "AI Model", asset)
        if ai_metrics:
            results.append(ai_metrics)
        
        # Run Heuristic Strategy
        print(f"  Running Heuristic Strategy...")
        heur_pred, heur_actual = heuristic_strategy(data)
        heur_metrics = calculate_metrics(heur_pred, heur_actual, "Heuristic", asset)
        if heur_metrics:
            results.append(heur_metrics)
        
        # Run Buy & Hold Strategy
        print(f"  Running Buy & Hold Strategy...")
        bh_pred, bh_actual = buyhold_strategy(data)
        bh_metrics = calculate_metrics(bh_pred, bh_actual, "Buy & Hold", asset)
        if bh_metrics:
            results.append(bh_metrics)
        
        print()
    
    # Display Results
    print("=" * 60)
    print("BACKTEST RESULTS")
    print("=" * 60)
    print()
    
    df = pd.DataFrame(results)
    
    # Group by strategy and calculate averages
    print("Strategy Performance Summary:")
    print("-" * 60)
    for strategy in ["AI Model", "Heuristic", "Buy & Hold"]:
        strategy_data = df[df['strategy'] == strategy]
        if len(strategy_data) > 0:
            avg_acc = strategy_data['accuracy'].mean()
            avg_ret = strategy_data['total_return'].mean()
            print(f"{strategy:15} | Avg Accuracy: {avg_acc:.2f}% | Avg Return: {avg_ret:+.2f}%")
    
    print()
    print("Detailed Results by Asset:")
    print("-" * 60)
    print(f"{'Asset':<8} {'Strategy':<15} {'Accuracy':<12} {'Return':<10} {'Predictions'}")
    print("-" * 60)
    for _, row in df.iterrows():
        print(f"{row['asset']:<8} {row['strategy']:<15} {row['accuracy']:>6.2f}%     {row['total_return']:>+6.2f}%    {row['num_predictions']:>6}")
    
    # Save to JSON
    output_path = "models/backtest_results.json"
    os.makedirs("models", exist_ok=True)
    with open(output_path, 'w') as f:
        json.dump(results, f, indent=2)
    
    print()
    print(f"Results saved to {output_path}")
    
    # Check if AI meets 55% threshold
    ai_results = df[df['strategy'] == 'AI Model']
    if len(ai_results) > 0:
        avg_ai_accuracy = ai_results['accuracy'].mean()
        print()
        print("=" * 60)
        if avg_ai_accuracy >= 55:
            print(f"✓ SUCCESS: AI Model achieves {avg_ai_accuracy:.2f}% accuracy (>55% threshold)")
        else:
            print(f"✗ FAILURE: AI Model achieves {avg_ai_accuracy:.2f}% accuracy (<55% threshold)")
        print("=" * 60)

if __name__ == "__main__":
    main()
