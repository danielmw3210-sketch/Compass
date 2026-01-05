# Test Oracle Node Submission
# Connects to fresh testnet and submits test prices

Write-Host "🔮 Testing Oracle Node Connection..." -ForegroundColor Cyan
Write-Host ""

Write-Host "Starting oracle node with vikingcoder credentials..." -ForegroundColor Yellow
Write-Host ""

# Run oracle node
cargo run --bin oracle_node -- `
    --account vikingcoder `
    --password "D4rkness10@@" `
    --rpc-url http://localhost:3030 `
    --interval 30 `
    --tickers BTCUSD, ETHUSD, SOLUSD, LTCUSD

Write-Host ""
Write-Host "✅ Oracle node should now be submitting prices every 30 seconds" -ForegroundColor Green
