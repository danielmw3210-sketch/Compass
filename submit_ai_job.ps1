# Compass AI Compute Job Submission (Layer 3)
# This script submits AI/ML compute jobs to the Compass Network

$RPC_ENDPOINT = "http://127.0.0.1:9000"

function Submit-ComputeJob {
    param(
        [string]$ModelId = "llama-3-8b-q4",
        [string]$Input = "Explain quantum computing in simple terms",
        [int]$MaxComputeUnits = 1000
    )
    
    $jobId = [guid]::NewGuid().ToString()
    $inputBytes = [System.Text.Encoding]::UTF8.GetBytes($Input)
    
    $body = @{
        jsonrpc = "2.0"
        method  = "submitCompute"
        params  = @{
            job_id            = $jobId
            model_id          = $ModelId
            inputs            = $inputBytes
            max_compute_units = $MaxComputeUnits
        }
        id      = 1
    } | ConvertTo-Json -Depth 5
    
    Write-Host "🔬 Submitting Compute Job to Layer 3..." -ForegroundColor Cyan
    Write-Host "   Job ID: $jobId"
    Write-Host "   Model: $ModelId"
    Write-Host "   Input: $Input"
    
    try {
        $response = Invoke-RestMethod -Uri $RPC_ENDPOINT -Method Post -Body $body -ContentType "application/json"
        
        if ($response.result) {
            Write-Host "✅ Job Submitted Successfully!" -ForegroundColor Green
            Write-Host "   Transaction Hash: $($response.result.tx_hash)" -ForegroundColor Yellow
            return $jobId
        }
        else {
            Write-Host "❌ Job Submission Failed" -ForegroundColor Red
            Write-Host "   Error: $($response.error.message)"
        }
    }
    catch {
        Write-Host "❌ RPC Error: $_" -ForegroundColor Red
    }
}

function Get-PendingJobs {
    param([string]$ModelId = $null)
    
    $params = if ($ModelId) { @{ model_id = $ModelId } } else { @{} }
    
    $body = @{
        jsonrpc = "2.0"
        method  = "getPendingComputeJobs"
        params  = $params
        id      = 2
    } | ConvertTo-Json
    
    Write-Host "📋 Fetching Pending Jobs from Layer 3..." -ForegroundColor Cyan
    
    try {
        $response = Invoke-RestMethod -Uri $RPC_ENDPOINT -Method Post -Body $body -ContentType "application/json"
        
        if ($response.result) {
            $jobs = $response.result.jobs
            Write-Host "Found $($jobs.Count) pending jobs" -ForegroundColor Green
            
            foreach ($job in $jobs) {
                Write-Host "`n  🔬 Job: $($job.job_id)" -ForegroundColor Yellow
                Write-Host "     Model: $($job.model_id)"
                Write-Host "     Max Units: $($job.max_compute_units)"
            }
            
            return $jobs
        }
    }
    catch {
        Write-Host "❌ RPC Error: $_" -ForegroundColor Red
    }
}

# Example Usage:
Write-Host "=== Compass Layer 3 AI Compute ===" -ForegroundColor Magenta
Write-Host ""

# Submit a sample job
$jobId = Submit-ComputeJob -ModelId "gpt-4o-mini" -Input "What is the meaning of life?" -MaxComputeUnits 500

Write-Host ""
Start-Sleep -Seconds 2

# Check pending jobs
Get-PendingJobs
