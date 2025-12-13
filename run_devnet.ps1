# run_devnet.ps1

Write-Host "Starting Compass Devnet..." -ForegroundColor Green

# 1. Kill any existing instances
Stop-Process -Name "rust_compass" -ErrorAction SilentlyContinue

# 2. Build Release (optional, or just use debug)
Write-Host "Building..."
cargo build

# 3. Start Leader Node (Background)
Write-Host "Starting Leader Node (P2P 8080, RPC 9000)..."
$leader = Start-Process -FilePath "cargo" -ArgumentList "run --bin rust_compass -- node start --p2p-port 8080 --rpc-port 9000 --db-path compass_leader.db" -PassThru -NoNewWindow

Start-Sleep -Seconds 5

# 4. Start Worker (Background)
Write-Host "Starting AI Worker (Connected to 9000)..."
$worker = Start-Process -FilePath "cargo" -ArgumentList "run --bin rust_compass -- worker --node http://127.0.0.1:9000 --model llama-2-7b" -PassThru -NoNewWindow

Start-Sleep -Seconds 2

# 5. Start Client (Interactive)
Write-Host "Devnet Running!"
Write-Host "Leader PID: $($leader.Id)"
Write-Host "Worker PID: $($worker.Id)"
Write-Host "Starting Client... (Ctrl+C to exit all)"

cargo run --bin rust_compass -- client

# Cleanup
Stop-Process -Id $leader.Id -ErrorAction SilentlyContinue
Stop-Process -Id $worker.Id -ErrorAction SilentlyContinue
Write-Host "Devnet Stopped."
