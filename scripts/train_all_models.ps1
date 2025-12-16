# Training script for all 4 AI models
# Run with: pwsh scripts/train_all_models.ps1

Write-Host "===============================================" -ForegroundColor Cyan
Write-Host "Training All AI Models (BTC, ETH, SOL, LTC)" -ForegroundColor Cyan
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host ""

$models = @("btc", "eth", "sol", "ltc")
$startTime = Get-Date

foreach ($model in $models) {
    Write-Host "[$model] Starting training..." -ForegroundColor Yellow
    $modelStart = Get-Date
    
    py -3.11 "scripts/train_${model}_agent.py"
    
    $modelEnd = Get-Date
    $elapsed = ($modelEnd - $modelStart).TotalSeconds
    Write-Host "[$model] Completed in $([math]::Round($elapsed, 1))s" -ForegroundColor Green
    Write-Host ""
}

$totalTime = ((Get-Date) - $startTime).TotalMinutes
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host "All models trained in $([math]::Round($totalTime, 1)) minutes" -ForegroundColor Cyan
Write-Host "===============================================" -ForegroundColor Cyan

# Verify ONNX files
Write-Host "`nVerifying ONNX files..." -ForegroundColor Yellow
foreach ($model in $models) {
    $onnxPath = "dist/models/${model}_v1.onnx"
    $scalerPath = "dist/models/${model}_scaler.json"
    
    if (Test-Path $onnxPath) {
        $size = (Get-Item $onnxPath).Length / 1KB
        Write-Host "[OK] $onnxPath ($([math]::Round($size, 1)) KB)" -ForegroundColor Green
    }
    else {
        Write-Host "[MISSING] $onnxPath" -ForegroundColor Red
    }
    
    if (Test-Path $scalerPath) {
        Write-Host "[OK] $scalerPath" -ForegroundColor Green
    }
    else {
        Write-Host "[MISSING] $scalerPath" -ForegroundColor Red
    }
}
