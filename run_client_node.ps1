# run_client_node.ps1
Write-Host "Starting Compass Client..." -ForegroundColor Cyan

# Just run the client interactive mode
# It connects to 127.0.0.1:9000 by default (as updated in code)
# Check if binary exists
$bin = ".\target\debug\rust_compass.exe"
if (Test-Path $bin) {
    & $bin client
}
else {
    Write-Host "Binary not found, building..."
    cargo run --bin rust_compass -- client
}
