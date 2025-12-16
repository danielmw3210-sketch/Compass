# Python 3.11 Installation Script for Windows
# This script installs Python 3.11 alongside your existing Python 3.14

Write-Host "=== Python 3.11 Installation for Compass AI ===" -ForegroundColor Cyan
Write-Host ""

# Method 1: Try winget (Windows Package Manager)
Write-Host "Attempting installation via winget..." -ForegroundColor Yellow
try {
    winget install Python.Python.3.11 --silent --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Python 3.11 installed successfully via winget!" -ForegroundColor Green
        Write-Host "Please restart your terminal for PATH updates to take effect." -ForegroundColor Yellow
        exit 0
    }
}
catch {
    Write-Host "⚠️ winget installation failed. Trying alternative method..." -ForegroundColor Yellow
}

# Method 2: Download and install manually
Write-Host ""
Write-Host "=== Manual Installation Instructions ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Download Python 3.11.7 from:" -ForegroundColor White
Write-Host "   https://www.python.org/ftp/python/3.11.7/python-3.11.7-amd64.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "2. Run the installer with these settings:" -ForegroundColor White
Write-Host "   ✓ Check 'Add Python 3.11 to PATH'" -ForegroundColor Green
Write-Host "   ✓ Choose 'Customize installation'" -ForegroundColor Green
Write-Host "   ✓ Enable 'pip' and 'py launcher'" -ForegroundColor Green
Write-Host "   ✓ Install for all users (optional)" -ForegroundColor Green
Write-Host ""
Write-Host "3. After installation, verify with:" -ForegroundColor White
Write-Host "   py -3.11 --version" -ForegroundColor Cyan
Write-Host ""
Write-Host "4. Return here and run:" -ForegroundColor White
Write-Host "   .\setup_ai_environment.ps1" -ForegroundColor Cyan
Write-Host ""
