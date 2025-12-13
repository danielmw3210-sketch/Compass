# run_full_node.ps1
Write-Host "Starting Compass Full Node..." -ForegroundColor Green

# 1. Kill existing
Stop-Process -Name "rust_compass" -ErrorAction SilentlyContinue

# 2. Build
Write-Host "Building..."
cargo build

# 3. Start Leader Node
Write-Host "Starting Validator (P2P 8080, RPC 9000)..."
$leader = Start-Process -FilePath "cmd" -ArgumentList "/k cargo run --bin rust_compass -- node start --p2p-port 8080 --rpc-port 9000 --db-path compass_leader.db" -PassThru

Start-Sleep -Seconds 5

# 4. Start Worker
Write-Host "Starting AI Worker (Connected to 9000)..."
$worker = Start-Process -FilePath "cargo" -ArgumentList "run --bin rust_compass -- worker --node-url http://127.0.0.1:9000 --model-id llama-2-7b" -PassThru

Write-Host "Full Node Running!"
Write-Host "Leader PID: $($leader.Id)"
Write-Host "Worker PID: $($worker.Id)"
Write-Host "Press Ctrl+C to stop..."

# Keep script running to maintain processes (if run directly)
try {
    while ($true) { Start-Sleep -Seconds 1 }
}
finally {
    Stop-Process -Id $leader.Id -ErrorAction SilentlyContinue
    Stop-Process -Id $worker.Id -ErrorAction SilentlyContinue
}
