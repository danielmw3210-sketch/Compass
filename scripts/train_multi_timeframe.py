#!/usr/bin/env python3
"""
Multi-Timeframe Signal Model Training Script

Trains 24 models total:
- 4 assets (BTC, ETH, SOL, LTC)
- 6 timeframes (5m, 30m, 1h, 3h, 6h, 24h)

Each model predicts price movement for its specific timeframe.
"""

import requests
import numpy as np
from sklearn.ensemble import RandomForestClassifier
import joblib
import os

TIMEFRAMES = {
    '5m': 1,      # 1 candle ahead
    '30m': 6,     # 6 candles ahead
    '1h': 12,     # 12 candles ahead  
    '3h': 36,     # 36 candles ahead
    '6h': 72,     # 72 candles ahead
    '24h': 288,   # 288 candles ahead
}

ASSETS = ['BTC', 'ETH', 'SOL', 'LTC']

def fetch_binance_data(symbol, limit=1000):
    """Fetch OHLCV data from Binance"""
    url = f"https://api.binance.com/api/v3/klines"
    params = {
        'symbol': f'{symbol}USDT',
        'interval': '5m',
        'limit': limit
    }
    
    response = requests.get(url, params=params)
    data = response.json()
    
    ohlcv = []
    for candle in data:
        ohlcv.append([
            float(candle[1]),  # Open
            float(candle[2]),  # High
            float(candle[3]),  # Low
            float(candle[4]),  # Close
            float(candle[5]),  # Volume
        ])
    
    return np.array(ohlcv)

def calculate_features(ohlcv):
    """Calculate 11 technical indicators"""
    closes = ohlcv[:, 3]
    highs = ohlcv[:, 1]
    lows = ohlcv[:, 2]
    volumes = ohlcv[:, 4]
    
    features = []
    
    for i in range(200, len(closes)):
        # RSI
        gains = np.maximum(np.diff(closes[i-14:i+1]), 0)
        losses = np.maximum(-np.diff(closes[i-14:i+1]), 0)
        avg_gain = np.mean(gains) if len(gains) > 0 else 0
        avg_loss = np.mean(losses) if len(losses) > 0 else 0
        rs = avg_gain / avg_loss if avg_loss != 0 else 0
        rsi = 100 - (100 / (1 + rs))
        
        # MACD  
        ema12 = np.mean(closes[i-11:i+1])
        ema26 = np.mean(closes[i-25:i+1])
        macd = (ema12 - ema26) / closes[i] if closes[i] != 0 else 0
        
        # Bollinger Bands
        sma_20 = np.mean(closes[i-19:i+1])
        std_20 = np.std(closes[i-19:i+1])
        bb_upper = sma_20 + (2 * std_20)
        bb_lower = sma_20 - (2 * std_20)
        bb_width = (bb_upper - bb_lower) / closes[i] if closes[i] != 0 else 0
        bb_position = (closes[i] - bb_lower) / (bb_upper - bb_lower) if (bb_upper - bb_lower) != 0 else 0.5
        
        # Moving average crossovers
        sma_20_50 = 1.0 if np.mean(closes[i-19:i+1]) > np.mean(closes[i-49:i+1]) else 0.0
        sma_50_200 = 1.0 if np.mean(closes[i-49:i+1]) > np.mean(closes[i-199:i+1]) else 0.0
        
        # Momentum
        momentum = (closes[i] - closes[i-10]) / closes[i-10] if closes[i-10] != 0 else 0
        
        # Volume ratio
        vol_ratio = volumes[i] / np.mean(volumes[i-19:i+1]) if np.mean(volumes[i-19:i+1]) != 0 else 1.0
        
        # ATR (Average True Range)
        tr = max(highs[i] - lows[i], abs(highs[i] - closes[i-1]), abs(lows[i] - closes[i-1]))
        atr = tr / closes[i] if closes[i] != 0 else 0
        
        # Stochastic Oscillator
        high_14 = np.max(highs[i-13:i+1])
        low_14 = np.min(lows[i-13:i+1])
        stochastic = ((closes[i] - low_14) / (high_14 - low_14)) * 100 if (high_14 - low_14) != 0 else 50
        
        # OBV Momentum  
        obv_change = (volumes[i] - volumes[i-5]) / volumes[i-5] if volumes[i-5] != 0 else 0
        
        features.append([
            rsi, macd, bb_width, bb_position,
            sma_20_50, sma_50_200, momentum, vol_ratio,
            atr, stochastic / 100, obv_change
        ])
    
    return np.array(features)

def create_labels(closes, lookahead_candles):
    """Create labels for a specific timeframe"""
    labels = []
    
    for i in range(200, len(closes) - lookahead_candles):
        future_price = closes[i + lookahead_candles]
        current_price = closes[i]
        
        price_change = (future_price - current_price) / current_price * 100
        
        if price_change > 2.0:
            labels.append(2)  # BUY
        elif price_change < -2.0:
            labels.append(0)  # SELL
        else:
            labels.append(1)  # HOLD
    
    return np.array(labels)

def train_model(asset, timeframe):
    """Train model for specific asset and timeframe"""
    print(f"\n{'='*60}")
    print(f"Training {asset} - {timeframe} model")
    print(f"{'='*60}")
    
    # Fetch data
    print(f"📊 Fetching {asset}USDT data...")
    ohlcv = fetch_binance_data(asset)
    
    # Calculate features
    print(f"🔢 Calculating features...")
    features = calculate_features(ohlcv)
    
    # Create labels for this timeframe
    lookahead = TIMEFRAMES[timeframe]
    print(f"🎯 Creating labels (lookahead: {lookahead} candles = {timeframe})...")
    labels = create_labels(ohlcv[:, 3], lookahead)
    
    # Align features and labels
    min_len = min(len(features), len(labels))
    X = features[:min_len]
    y = labels[:min_len]
    
    print(f"✅ Dataset: {len(X)} samples with 11 features")
    print(f"   Class distribution: SELL={np.sum(y==0)}, HOLD={np.sum(y==1)}, BUY={np.sum(y==2)}")
    
    # Train Random Forest
    print(f"🌲 Training Random Forest...")
    model = RandomForestClassifier(
        n_estimators=100,
        max_depth=12,
        min_samples_split=10,
        random_state=42,
        n_jobs=-1
    )
    
    model.fit(X, y)
    
    # Calculate training accuracy
    train_accuracy = model.score(X, y)
    print(f"   Training Accuracy: {train_accuracy*100:.1f}%")
    
    # Save model
    model_path = f"models/{asset.lower()}_signal_{timeframe}.bin"
    os.makedirs('models', exist_ok=True)
    joblib.dump(model, model_path)
    
    print(f"💾 Saved: {model_path}")
    print(f"✅ {asset} {timeframe} model complete!")
    
    return model_path, train_accuracy

def main():
    """Train all 24 models"""
    print("Multi-Timeframe Model Training")
    print(f"   Assets: {len(ASSETS)}")
    print(f"   Timeframes: {len(TIMEFRAMES)}")
    print(f"   Total Models: {len(ASSETS) * len(TIMEFRAMES)}")
    
    results = []
    
    for asset in ASSETS:
        for timeframe in TIMEFRAMES.keys():
            try:
                model_path, accuracy = train_model(asset, timeframe)
                results.append({
                    'asset': asset,
                    'timeframe': timeframe,
                    'path': model_path,
                    'accuracy': accuracy
                })
            except Exception as e:
                print(f"❌ Failed to train {asset} {timeframe}: {e}")
    
    print(f"\n{'='*60}")
    print("📊 Training Summary")
    print(f"{'='*60}")
    print(f"Models trained: {len(results)}/{len(ASSETS) * len(TIMEFRAMES)}")
    print(f"\nResults:")
    for r in results:
        print(f"   {r['asset']:4s} {r['timeframe']:4s} → {r['accuracy']*100:5.1f}% | {r['path']}")
    
    avg_accuracy = np.mean([r['accuracy'] for r in results])
    print(f"\n📈 Average Accuracy: {avg_accuracy*100:.1f}%")
    print(f"✅ Multi-timeframe training complete!")

if __name__ == '__main__':
    main()
