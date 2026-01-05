# Manual RPC Test - Submit Oracle Price
# Tests the submitOraclePrice endpoint directly

$body = @{
    jsonrpc = "2.0"
    id      = 1
    method  = "submitOraclePrice"
    params  = @{
        oracle_account = "vikingcoder"
        ticker         = "BTCUSD"
        price          = 43250.50
        timestamp      = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        signature      = @()  # Empty for now (TODO: implement signing)
    }
} | ConvertTo-Json -Depth 10

Write-Host "📡 Submitting Oracle Price via RPC..." -ForegroundColor Cyan
Write-Host ""
Write-Host "Payload:" -ForegroundColor Yellow
Write-Host $body
Write-Host ""

try {
    $response = Invoke-RestMethod -Uri "http://localhost:3030" -Method Post -Body $body -ContentType "application/json"
    
    Write-Host "✅ Success!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Response:" -ForegroundColor Cyan
    $response | ConvertTo-Json -Depth 10
    
}
catch {
    Write-Host "❌ Error:" -ForegroundColor Red
    Write-Host $_.Exception.Message
    Write-Host ""
    Write-Host $_.ErrorDetails.Message
}
