# Script to bundle ONNX Runtime binaries for release
# Usage: ./bundle_onnx.ps1

$OrtVersion = "1.16.3"
$BaseUrl = "https://github.com/microsoft/onnxruntime/releases/download/v$OrtVersion"
$OutDir = "target/release"

Write-Host "Bundling ONNX Runtime v$OrtVersion..."

# Create Release Dir if not exists
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$File = "onnxruntime-win-x64-$OrtVersion.zip"
$Url = "$BaseUrl/$File"
$Dest = "$env:TEMP\$File"

Write-Host "Downloading $Url..."
if (-not (Test-Path $Dest)) {
    Invoke-WebRequest -Uri $Url -OutFile $Dest
}

Write-Host "Extracting..."
Expand-Archive -Path $Dest -DestinationPath "$env:TEMP\ort_extract" -Force

$DllPath = "$env:TEMP\ort_extract\onnxruntime-win-x64-$OrtVersion\lib\onnxruntime.dll"
if (Test-Path $DllPath) {
    Copy-Item $DllPath -Destination $OutDir
    Write-Host "Copied onnxruntime.dll to $OutDir"
}
if (-not (Test-Path $DllPath)) {
    Write-Error "Could not find onnxruntime.dll in extracted files"
}

Write-Host "Done. You can now run the release binary without network dependency."
