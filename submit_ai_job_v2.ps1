param (
    [string]$model = "llama-3-8b",
    [string]$prompt = "Explain quantum gravity",
    [int]$computeUnits = 100
)

$url = "http://127.0.0.1:8000"

# Convert prompt to mock bytes input
$inputs = [System.Text.Encoding]::UTF8.GetBytes($prompt)

$jobId = [Guid]::NewGuid().ToString()
$ownerId = "client_wallet_123"

# Mock signature (In real app, sign the job details)
$signature = "mock_sig_for_" + $jobId

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

Write-Host "🧠 Submitting AI Job..."
Write-Host "   ID: $jobId"
Write-Host "   Model: $model"

try {
    $response = Invoke-RestMethod -Uri $url -Method Post -Body $body -ContentType "application/json"
    if ($response.error) {
        Write-Error "RPC Error: $($response.error.message)"
    } else {
        Write-Host "✅ Job Submitted Successfully!" -ForegroundColor Green
        Write-Host "   Worker should pick this up shortly."
    }
} catch {
    Write-Error "Failed to connect to node: $_"
}
