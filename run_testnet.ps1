# run_testnet.ps1 - Launch Local Testnet with Persistent Identity, GUI, Node, and Worker

Write-Host "Initializing Compass Testnet Environment (Production Mode)..." -ForegroundColor Cyan

# 0. Pre-check: Ensure Wallets Exist
$wallets = @("admin", "Daniel", "worker1")

foreach ($w in $wallets) {
    if (-not (Test-Path "wallets.json")) {
        Write-Host "Creating $w wallet..."
        cargo run --quiet --release -- wallet create --name $w
    }
    else {
        # Simple check if wallet exists in json (grep equivalent)
        $content = Get-Content "wallets.json" -Raw
        if ($content -notmatch $w) {
            Write-Host "Creating $w wallet..."
            cargo run --quiet --release -- wallet create --name $w
        }
    }
}

# 1. Force New Identity if Password Protected (Fixes Prompt Issue)
# If admin.json exists, we assume it might be old/password protected.
# We backup and generate a new passwordless one for the testnet session.
if (Test-Path "admin.json") {
    Write-Host "Found existing 'admin.json'. Renaming to 'admin.json.bak' to ensure passwordless startup..." -ForegroundColor Yellow
    Move-Item -Force "admin.json" "admin.json.bak"
}

# 2. Generate Passwordless Identity
Write-Host "Generating Passwordless 'admin.json' for Testnet..." -ForegroundColor Yellow
cargo run --quiet --release --example gen_testnet_id
Move-Item -Force "testnet_identity.json" "admin.json"

# 3. Start Full Node
Write-Host "Starting Compass Full Node (RPC: 9000)..." -ForegroundColor Green
$nodeProcess = Start-Process -FilePath "cargo" -ArgumentList "run --release -- node start --rpc-port 9000 --db-path testnet_db" -PassThru -NoNewWindow
Start-Sleep -Seconds 5

# 4. Start AI Worker
Write-Host "Starting AI Worker (RPC: 9001)..." -ForegroundColor Magenta
# Ensure wallet is definitely created before this
# $workerProcess = Start-Process -FilePath "cargo" -ArgumentList "run --release -- worker --node-url http://127.0.0.1:9000 --wallet worker1" -PassThru -NoNewWindow
Write-Host "Skipping external worker (use GUI worker)..." -ForegroundColor Magenta
Start-Sleep -Seconds 2

# 5. Start GUI
Write-Host "Launching Compass Desktop Client..." -ForegroundColor Cyan
Start-Process -FilePath "cargo" -ArgumentList "run --release --bin gui"

# 6. Keep Alive
Write-Host "Testnet is running. Press ENTER to stop..." -ForegroundColor Yellow
Read-Host

# Cleanup when GUI closes
Write-Host "Shutting down Testnet..." -ForegroundColor Red
Stop-Process -Id $nodeProcess.Id -ErrorAction SilentlyContinue
# Stop-Process -Id $workerProcess.Id -ErrorAction SilentlyContinue
Write-Host "Testnet stopped."
