# Wallet Access Verification Script
# Checks if you have private keys for all genesis accounts

Write-Host "🔍 COMPASS Genesis Wallet Verification" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Check 1: Daniel wallet
Write-Host "1️⃣ Checking 'Daniel' wallet (500,000,000,000 COMPASS)..." -ForegroundColor Yellow

$danielExists = Test-Path ".\identity.json" -or (Get-Content ".\wallets.json" -ErrorAction SilentlyContinue | Select-String "Daniel")

if ($danielExists) {
    Write-Host "   ✅ FOUND: Daniel wallet exists" -ForegroundColor Green
    Write-Host "   📁 Location: wallets.json or identity.json" -ForegroundColor Gray
}
else {
    Write-Host "   ❌ NOT FOUND: Daniel wallet missing!" -ForegroundColor Red
}

Write-Host ""

# Check 2: Admin wallet  
Write-Host "2️⃣ Checking 'admin' wallet (1,000,000,000,000 COMPASS)..." -ForegroundColor Yellow

$identityContent = Get-Content ".\identity.json" -ErrorAction SilentlyContinue
$isAdmin = $identityContent | Select-String '"role".*"Admin"'

if ($isAdmin) {
    Write-Host "   ✅ FOUND: identity.json has Admin role" -ForegroundColor Green
    Write-Host "   🔐 Unlocked with password: 'duke'" -ForegroundColor Gray
    Write-Host "   📝 Public Key: 5ee36581af31396b5c4750f49adf9e91711c79f88c1743f14dac03b3c3ff8830" -ForegroundColor Gray
}
else {
    Write-Host "   ❌ NOT FOUND: Admin wallet missing!" -ForegroundColor Red
}

Write-Host ""

# Check 3: Foundation wallet
Write-Host "3️⃣ Checking 'foundation' wallet (1,000,000,000,000 COMPASS)..." -ForegroundColor Yellow

$foundationWallet = Get-ChildItem . -Filter "*foundation*" -ErrorAction SilentlyContinue
$walletsHasFoundation = Get-Content ".\wallets.json" -ErrorAction SilentlyContinue | Select-String "foundation"

if ($foundationWallet -or $walletsHasFoundation) {
    Write-Host "   ✅ FOUND: Foundation wallet exists" -ForegroundColor Green
}
else {
    Write-Host "   ⚠️ WARNING: No 'foundation' wallet file found!" -ForegroundColor Red
    Write-Host "   This means 1 TRILLION COMPASS may be LOCKED FOREVER!" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Possible scenarios:" -ForegroundColor Yellow
    Write-Host "   • foundation = admin (same wallet, reused address)" -ForegroundColor Gray
    Write-Host "   • foundation tokens are BURNED (no private key)" -ForegroundColor Gray
    Write-Host "   • foundation is controlled by vault_master.seed" -ForegroundColor Gray
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan

# Summary
Write-Host "📊 SUMMARY:" -ForegroundColor Cyan
Write-Host ""

$totalAccessible = 0
$totalLocked = 0

if ($danielExists) { 
    $totalAccessible += 500000000000 
    Write-Host "   ✅ Daniel: 500,000,000,000 COMPASS (Accessible)" -ForegroundColor Green
}

if ($isAdmin) { 
    $totalAccessible += 1000000000000 
    Write-Host "   ✅ Admin: 1,000,000,000,000 COMPASS (Accessible)" -ForegroundColor Green
}

if (-not ($foundationWallet -or $walletsHasFoundation)) {
    $totalLocked = 1000000000000
    Write-Host "   ❌ Foundation: 1,000,000,000,000 COMPASS (POTENTIALLY LOCKED!)" -ForegroundColor Red
}
else {
    $totalAccessible += 1000000000000
    Write-Host "   ✅ Foundation: 1,000,000,000,000 COMPASS (Accessible)" -ForegroundColor Green
}

Write-Host ""
Write-Host "   Total Accessible: $($totalAccessible.ToString('N0')) COMPASS" -ForegroundColor Green
if ($totalLocked -gt 0) {
    Write-Host "   Total Locked: $($totalLocked.ToString('N0')) COMPASS" -ForegroundColor Red
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Backup reminder
Write-Host "💾 BACKUP REMINDER:" -ForegroundColor Yellow
Write-Host "   Your vault_master.seed.mnemonic contains:" -ForegroundColor Gray
Write-Host "   'insane pride shoot mutual slim build...'" -ForegroundColor Gray
Write-Host ""
Write-Host "   🚨 WRITE THIS DOWN ON PAPER AND STORE IN A SAFE!" -ForegroundColor Red
Write-Host ""
