param(
    [string]$ticker = "BTC",
    [int]$hours = 6,
    [int]$intervalMinutes = 1,
    [int]$rewardPerUpdate = 10,
    [string]$rpcUrl = "http://localhost:9000"
)

$requestBody = @{
    jsonrpc = "2.0"
    method = "submitRecurringOracleJob"
    params = @{
        ticker = $ticker
        duration_hours = $hours
        interval_minutes = $intervalMinutes
        reward_per_update = $rewardPerUpdate
        submitter = "admin"
    }
    id = 1
} | ConvertTo-Json

Write-Host "Creating $hours-hour recurring oracle job for $ticker" -ForegroundColor Cyan
Write-Host "Updates every $intervalMinutes minute(s)" -ForegroundColor Yellow
Write-Host "Reward: $rewardPerUpdate COMPASS per update" -ForegroundColor Green
Write-Host "Total updates: $($hours * 60 / $intervalMinutes)" -ForegroundColor White
Write-Host ""
Write-Host "Sending request to $rpcUrl..." -ForegroundColor Gray

try {
    $response = Invoke-RestMethod -Uri $rpcUrl -Method Post -Body $requestBody -ContentType "application/json"
    
    if ($response.error) {
        Write-Host "RPC Error: $($response.error.message)" -ForegroundColor Red
    } else {
        Write-Host "✅ Job created successfully!" -ForegroundColor Green
        Write-Host "Job ID: $($response.result.job_id)" -ForegroundColor White
        Write-Host "Status: $($response.result.status)" -ForegroundColor White
        Write-Host "Total Reward: $($response.result.total_reward) COMPASS" -ForegroundColor White
    }
} catch {
    Write-Host "Error connecting to node: $_" -ForegroundColor Red
    Write-Host "Ensure the node is running on $rpcUrl" -ForegroundColor Yellow
}
