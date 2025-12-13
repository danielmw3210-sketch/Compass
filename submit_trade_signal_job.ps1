param (
    [string]$ticker = "BTC",
    [string]$prompt = "Analyze trend", 
    [int]$computeUnits = 500
)

$url = "http://127.0.0.1:9000"
$model = "crypto-signal-v1"

# Convert prompt to mock bytes input
$inputs = [System.Text.Encoding]::UTF8.GetBytes($ticker)

$jobId = [Guid]::NewGuid().ToString()
$ownerId = "trader_wallet_007"

# Mock signature 
$signature = "sig_for_trade_" + $jobId

$body = @{
    jsonrpc = "2.0"
    method = "submitCompute"
    params = @{
        job_id = $jobId
        model_id = $model
        inputs = $inputs
        max_compute_units = $computeUnits
        signature = $signature
        owner_id = $ownerId
    }
    id = 1
} | ConvertTo-Json -Depth 5

Write-Host "Ordering Trading Signal Job..."
Write-Host "   ID: $jobId"
Write-Host "   Model: $model (Technical Analysis)"
Write-Host "   Ticker: $ticker"

try {
    $response = Invoke-RestMethod -Uri $url -Method Post -Body $body -ContentType "application/json"
    if ($response.error) {
        Write-Error "RPC Error: $($response.error.message)"
    } else {
        Write-Host "Order Placed!" -ForegroundColor Green
        Write-Host "   Worker is analyzing market data..."
    }
} catch {
    Write-Error "Failed to connect to node."
    Write-Error $_
}
