# Fresh Account-Based Testnet Startup Script
# Compass v2.0 - Account System Test

Write-Host "🧹 Cleaning old database..." -ForegroundColor Yellow

# Remove old Sled database
if (Test-Path "compass_db") {
    Remove-Item -Recurse -Force "compass_db"
    Write-Host "✅ Removed old compass_db" -ForegroundColor Green
}

# Remove old vault data if exists
if (Test-Path "vaults.json") {
    Remove-Item "vaults.json"
    Write-Host "✅ Removed vaults.json" -ForegroundColor Green
}

Write-Host ""
Write-Host "🚀 Starting Fresh Testnet with v2.0 Genesis..." -ForegroundColor Cyan
Write-Host ""
Write-Host "Expected Genesis Output:" -ForegroundColor Yellow
Write-Host "  🔐 Creating admin account: vikingcoder"
Write-Host "  ✅ Admin account created: vikingcoder"
Write-Host "  💰 Minted 10,000,000 COMPASS to admin"
Write-Host "  🔮 Registered admin as genesis oracle (100K COMPASS staked)"
Write-Host ""

# Build and run
cargo run --bin rust_compass
