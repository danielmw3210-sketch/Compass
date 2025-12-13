# run_local_follower.ps1
Write-Host "Starting Local Follower Node..." -ForegroundColor Cyan

# Start a local node (P2P 8081, RPC 9001) and peer with the main Local Node (8080)
# DB path is different to avoid lock conflicts
$bin = ".\target\debug\rust_compass.exe"

if (Test-Path $bin) {
    & $bin node start --p2p-port 8081 --rpc-port 9001 --peer 127.0.0.1:8080 --db-path compass_follower.db
}
else {
    Write-Host "Building binary..."
    cargo run --bin rust_compass -- node start --p2p-port 8081 --rpc-port 9001 --peer 127.0.0.1:8080 --db-path compass_follower.db
}
