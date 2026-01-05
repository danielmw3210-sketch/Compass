# Verify Fresh Testnet Script
# Checks if v2.0 genesis initialized correctly

Write-Host "🔍 Verifying Fresh Testnet State..." -ForegroundColor Cyan
Write-Host ""

# Check if database exists
if (!(Test-Path "compass_db")) {
    Write-Host "❌ Database not found! Node may not have started." -ForegroundColor Red
    exit 1
}

Write-Host "✅ Database exists" -ForegroundColor Green
Write-Host ""

Write-Host "📊 Expected State:" -ForegroundColor Yellow
Write-Host "  Account: vikingcoder"
Write-Host "  Balance: 10,000,000 COMPASS"
Write-Host "  Oracle Stake: 100,000 COMPASS"
Write-Host "  Oracle Status: Active"
Write-Host ""

Write-Host "💡 To verify manually, check the node logs for:" -ForegroundColor Cyan
Write-Host "  1. '🔐 Creating admin account: vikingcoder'"
Write-Host "  2. '💰 Minted 10,000,000 COMPASS to admin'"
Write-Host "  3. '🔮 Registered admin as genesis oracle'"
Write-Host ""

Write-Host "🧪 Next Steps:" -ForegroundColor Green
Write-Host "  1. Check node is running and genesis completed"
Write-Host "  2. Run oracle node: .\test_oracle_node.ps1"
Write-Host "  3. Submit test price via RPC"
Write-Host ""
