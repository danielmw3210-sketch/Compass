Write-Host "Initializing Training..."
$python_cmd = "python"
if (Get-Command py -ErrorAction SilentlyContinue) {
    $python_cmd = "py"
}

Write-Host "Using Python: $python_cmd"
& $python_cmd -m pip install scikit-learn skl2onnx numpy requests joblib
Write-Host "Starting Python Script..."
& $python_cmd scripts/train_enhanced_24h.py
