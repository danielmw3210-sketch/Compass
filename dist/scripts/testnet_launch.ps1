# Testnet Launch Script
# Wipes corrupted data (admin.json, DB) and starts fresh for Testnet

$ErrorActionPreference = "Stop"

Write-Host "🚀 INITIALIZING TESTNET LAUNCH SEQUENCE..." -ForegroundColor Cyan

# 1. Kill old processes
Write-Host "1️⃣  Stopping running nodes..."
Stop-Process -Name "rust_compass", "gui" -Force -ErrorAction SilentlyContinue

# 2. Wipe Corrupted Data
Write-Host "2️⃣  Wiping State (Fresh Start)..."
if (Test-Path "compass_leader.db") { Remove-Item "compass_leader.db" -Recurse -Force; Write-Host "   - Deleted compass_leader.db" }
if (Test-Path "dist/compass_leader.db") { Remove-Item "dist/compass_leader.db" -Recurse -Force; Write-Host "   - Deleted dist/compass_leader.db" }
if (Test-Path "admin.json") { Remove-Item "admin.json" -Force; Write-Host "   - Deleted root admin.json" }
if (Test-Path "dist/admin.json") { Remove-Item "dist/admin.json" -Force; Write-Host "   - Deleted dist/admin.json" }
if (Test-Path "wallets.json") { Remove-Item "wallets.json" -Force; Write-Host "   - Deleted root wallets.json" }
if (Test-Path "dist/wallets.json") { Remove-Item "dist/wallets.json" -Force; Write-Host "   - Deleted dist/wallets.json" }

# 3. Generate New Admin Identity
Write-Host "3️⃣  Generating NEW Admin Identity..."
# We pipe the password "password123" to the command
"password123" | .\target\release\rust_compass.exe keys generate --role admin --name admin

if (Test-Path "admin.json") {
    Move-Item "admin.json" "dist/admin.json" -Force
    Write-Host "✅ Generated and moved admin.json to dist/" -ForegroundColor Green
}
else {
    Write-Error "❌ Failed to generate admin.json"
}

# 4. Initialize Wallets (Optional, creates empty file so GUI doesn't complain)
Write-Host "4️⃣  Resetting GUI Wallets..."
Set-Content -Path "dist/wallets.json" -Value "{}"
Write-Host "✅ Reset wallets.json"

Write-Host "`n🎉 TESTNET PREP COMPLETE!" -ForegroundColor Green
Write-Host "   Admin Password: 'password123' (You will need this to start the node)"
Write-Host "   Next Step: Run '.\rust_compass.exe node start ...' inside dist/"
