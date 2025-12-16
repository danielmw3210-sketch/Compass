# Build Compass Production Release
$ErrorActionPreference = "Stop"

Write-Host "Starting Production Build..."

# 1. Build Release Binary
Write-Host "Compiling Rust binaries (Release mode)..."
cargo build --release
if ($LASTEXITCODE -ne 0) { Write-Error "Build Failed"; exit 1 }

# 2. Run ONNX Bundler
Write-Host "Bundling ONNX Runtime..."
powershell -ExecutionPolicy Bypass -File ./scripts/bundle_onnx.ps1

# 3. Create Dist Folder
$DistDir = "dist"
if (Test-Path $DistDir) { Remove-Item $DistDir -Recurse -Force }
New-Item -ItemType Directory -Path $DistDir | Out-Null
New-Item -ItemType Directory -Path "$DistDir/models" | Out-Null

# 4. Copy Binaries
Write-Host "Copying files to dist..."
Copy-Item "target/release/rust_compass.exe" -Destination $DistDir
Copy-Item "target/release/gui.exe" -Destination $DistDir
if (Test-Path "target/release/onnxruntime.dll") { Copy-Item "target/release/onnxruntime.dll" -Destination $DistDir }
Copy-Item "models/*" -Destination "$DistDir/models" -Recurse
if (Test-Path "scripts") { Copy-Item "scripts" -Destination $DistDir -Recurse }

# 5. Create Default Configs
Write-Host "Creating default configs..."
Set-Content -Path "$DistDir/wallets.json" -Value "{}"
if (Test-Path "admin.json") { 
    Write-Host "Copying existing admin.json..."
    Copy-Item "admin.json" -Destination "$DistDir/admin.json" 
}
else {
    Set-Content -Path "$DistDir/admin.json" -Value "{}"
}
Set-Content -Path "$DistDir/oracle.json" -Value "{}"

# 6. Usage Instructions
Set-Content -Path "$DistDir/README.txt" -Value "Compass Node Deployment"
Add-Content -Path "$DistDir/README.txt" -Value "1. Run compass_node.exe"
Add-Content -Path "$DistDir/README.txt" -Value "2. Run gui.exe"
Add-Content -Path "$DistDir/README.txt" -Value "3. Place admin.json here for admin features."

Write-Host "Build Complete! Deployment ready in dist folder."
Get-ChildItem $DistDir
