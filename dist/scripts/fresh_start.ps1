Write-Host "Starting Fresh Blockchain Reset..." -ForegroundColor Cyan

# 1. Stop Processes
Write-Host "Stopping running nodes..."
Stop-Process -Name "rust_compass" -ErrorAction SilentlyContinue
Stop-Process -Name "gui" -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# 2. Wipe Data
Write-Host "Wiping Database and Identities (Root & Dist)..."
# Root (where cargo run might have created them)
if (Test-Path "compass_leader.db") { Remove-Item "compass_leader.db" -Recurse -Force }
if (Test-Path "compass_db_leader") { Remove-Item "compass_db_leader" -Recurse -Force }
if (Test-Path "admin.json") { Remove-Item "admin.json" -Force }
if (Test-Path "wallets.json") { Remove-Item "wallets.json" -Force }
if (Test-Path "genesis.json") { Remove-Item "genesis.json" -Force }
if (Test-Path "admin_key.mnemonic") { Remove-Item "admin_key.mnemonic" -Force }

# Dist (where production build runs)
if (Test-Path "dist/compass_leader.db") { Remove-Item "dist/compass_leader.db" -Recurse -Force }
if (Test-Path "dist/compass_db_leader") { Remove-Item "dist/compass_db_leader" -Recurse -Force }
if (Test-Path "dist/admin.json") { Remove-Item "dist/admin.json" -Force }
if (Test-Path "dist/wallets.json") { Remove-Item "dist/wallets.json" -Force }
if (Test-Path "dist/market.json") { Remove-Item "dist/market.json" -Force }
if (Test-Path "dist/genesis.json") { Remove-Item "dist/genesis.json" -Force }
if (Test-Path "dist/admin_key.mnemonic") { Remove-Item "dist/admin_key.mnemonic" -Force }

# 3. Generate New Admin Identity
Write-Host "Generating NEW Admin Identity..."
Set-Location dist
./rust_compass.exe admin-gen
Set-Location ..

Write-Host "Fresh Start Complete!"
Write-Host "-------------------------------------------"
Write-Host "Run the following command to start your node:"
Write-Host "./rust_compass.exe node start --p2p-port 8080 --rpc-port 9000 --db-path compass_leader.db" -ForegroundColor Green
Write-Host "-------------------------------------------"
