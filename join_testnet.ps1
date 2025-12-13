# join_testnet.ps1

Write-Host "Joining Live Network at 34.45.156.0..." -ForegroundColor Cyan

# Start a local node (P2P 8081) and peer with the Live Node (8080)
# We run as a FOLLOWER initially to just sync the chain.

cargo run --bin rust_compass -- node start --p2p-port 8081 --rpc-port 9001 --peer 34.45.156.0:8080 --db-path compass_follower.db
