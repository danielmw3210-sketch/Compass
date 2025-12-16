Write-Host "🛑 Stopping all Compass processes..." -ForegroundColor Yellow
Stop-Process -Name "rust_compass" -ErrorAction SilentlyContinue
Stop-Process -Name "gui" -ErrorAction SilentlyContinue
Stop-Process -Name "oracle_monitor" -ErrorAction SilentlyContinue

Start-Sleep -Seconds 2

Write-Host "🔍 Checking databases for NFTs..." -ForegroundColor Cyan

$env:RUST_BACKTRACE = "1"
cargo run --example check_nft_status

Write-Host "✅ Done. If you see your NFTs above, use the database path shown." -ForegroundColor Green
