# Clean and Start Fresh - Compass v2.0
# Removes old database and starts with v2.0 genesis

Write-Host "Cleaning old state..." -ForegroundColor Yellow

# Remove old database
if (Test-Path "compass_db_leader") {
    Remove-Item -Recurse -Force compass_db_leader
    Write-Host "  [OK] Removed compass_db_leader" -ForegroundColor Green
}

# Remove legacy files (optional - already migrated)
if (Test-Path "wallets.json") {
    Remove-Item wallets.json
    Write-Host "  [OK] Removed wallets.json" -ForegroundColor Green
}

if (Test-Path "vaults.json") {
    Remove-Item vaults.json
    Write-Host "  [OK] Removed vaults.json" -ForegroundColor Green
}

if (Test-Path "market.json") {
    Remove-Item market.json
    Write-Host "  [OK] Removed market.json" -ForegroundColor Green
}

Write-Host ""
Write-Host "Starting fresh v2.0 genesis..." -ForegroundColor Cyan
Write-Host ""
Write-Host "Expected output:" -ForegroundColor Yellow
Write-Host "  Creating admin account: vikingcoder"
Write-Host "  Minted 10,000,000 COMPASS to admin"
Write-Host "  Registered admin as genesis oracle"
Write-Host ""

# Start node
cargo run --bin rust_compass
