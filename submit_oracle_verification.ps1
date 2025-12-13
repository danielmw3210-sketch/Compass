param(
    [string]$ticker = "BTC",
    [string]$rpcUrl = "http://localhost:8545"
)

Write-Host "=== Oracle Price Verification Job Submission ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Submitting verification job for: $ticker" -ForegroundColor Yellow
Write-Host "RPC Endpoint: $rpcUrl" -ForegroundColor Gray
Write-Host ""

$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$jobId = "oracle_${ticker}_$timestamp"

$requestBody = @{
    jsonrpc = "2.0"
    method = "submitOracleVerificationJob"
    params = @{
        ticker = $ticker
        oracle_price = "45000.00"
        timestamp = $timestamp
        job_id = $jobId
        submitter = "admin"
        max_compute_units = 1000
    }
    id = 1
} | ConvertTo-Json -Depth 10

try {
    Write-Host "Sending RPC request..." -ForegroundColor Gray
    
    $response = Invoke-RestMethod -Uri $rpcUrl -Method Post -Body $requestBody -ContentType "application/json"
    
    Write-Host ""
    Write-Host "SUCCESS! Oracle verification job submitted" -ForegroundColor Green
    Write-Host "Job ID: $jobId" -ForegroundColor White
    Write-Host ""
    Write-Host "What happens next:" -ForegroundColor Cyan
    Write-Host "  1. Workers poll getPendingOracleJobs endpoint"
    Write-Host "  2. Query external APIs (CoinGecko, Binance, Kraken)"
    Write-Host "  3. Calculate price deviation"
    Write-Host "  4. Sign results with worker keypair"
    Write-Host "  5. Submit via submitOracleVerificationResult"
    Write-Host ""
    Write-Host "Response:" -ForegroundColor Yellow
    $response | ConvertTo-Json -Depth 10
    
} catch {
    Write-Host "RPC Error: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Make sure the Compass OS node is running with RPC enabled!" -ForegroundColor Yellow
    Write-Host "Start it with: cargo run --bin rust_compass" -ForegroundColor Gray
}
