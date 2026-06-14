# ANS Browser — Windows One-Click Installer
# Run: powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  ANS Browser — One-Click Installer" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ProjectRoot

Write-Host "[1/5] Checking prerequisites..." -ForegroundColor Yellow
$pythonOk=$false; try{$v=&python --version 2>&1;if($v -match "3\.\d+"){$pythonOk=$true;Write-Host "  Python: $v" -ForegroundColor Green}}catch{}
if(-not $pythonOk){Write-Host "  ERROR: Python 3.10+ required." -ForegroundColor Red;exit 1}
$rustOk=$false; try{$v=&rustc --version 2>&1;if($v){$rustOk=$true;Write-Host "  Rust: $v" -ForegroundColor Green}}catch{}
if(-not $rustOk){Write-Host "  WARNING: Rust not found. Prebuilt binary will be used if available." -ForegroundColor Yellow}

Write-Host "[2/5] Building daemon..." -ForegroundColor Yellow
Push-Location $ProjectRoot
if($rustOk){cargo build --release 2>&1;Write-Host "  Daemon built." -ForegroundColor Green}else{Write-Host "  Skipping build." -ForegroundColor Yellow}
Pop-Location

Write-Host "[3/5] Installing Python deps..." -ForegroundColor Yellow
Push-Location "$ProjectRoot\nerves"
if(-not (Test-Path ".venv")){python -m venv .venv;Write-Host "  Created venv." -ForegroundColor Green}
&.venv\Scripts\pip install -e . 2>&1
Write-Host "  Python deps installed." -ForegroundColor Green
Pop-Location

Write-Host "[4/5] Installing Chromium (Playwright)..." -ForegroundColor Yellow
try{npx playwright install chromium 2>&1;Write-Host "  Chromium installed." -ForegroundColor Green}catch{Write-Host "  Skipped. Daemon auto-downloads Chromium." -ForegroundColor Yellow}

Write-Host "[5/5] Starting ANS Browser..." -ForegroundColor Yellow
$envPath="$ProjectRoot\.env"
if(-not (Test-Path $envPath)){@"
ANS_MODE=fast
ANS_GRPC_PORT=50051
DEEPSEEK_API_KEY=sk-your-key-here
OPENAI_API_KEY=sk-your-key-here
"@|Out-File -FilePath $envPath -Encoding utf8;Write-Host "  Created .env. Add your API keys!" -ForegroundColor Green}

$daemon="$ProjectRoot\target\release\ans-daemon.exe"
if(Test-Path $daemon){Write-Host "  Launching on ports 50051 (gRPC) + 50052 (Web)..." -ForegroundColor Green;Write-Host "  Dashboard: http://localhost:50052" -ForegroundColor Cyan;Start-Process $daemon -ArgumentList "--grpc-port 50051 --gateway-port 50052";Start-Sleep 2;Start-Process "http://localhost:50052/"}else{Write-Host "  ERROR: ans-daemon.exe not found. Build may have failed." -ForegroundColor Red}
