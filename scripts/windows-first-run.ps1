# Argo v0.4 Tauri 2 / Rust + TypeScript. Designed for Windows PowerShell 7/5.1.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
foreach ($tool in @('node','npm','cargo','rustc','git')) {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
    throw "Missing prerequisite: $tool. Install it before starting Argo."
  }
}
npm install
if ($LASTEXITCODE -ne 0) {throw 'npm install failed'}
npm run typecheck
if ($LASTEXITCODE -ne 0) {throw 'TypeScript typecheck failed'}
npm test
if ($LASTEXITCODE -ne 0) {throw 'Contract tests failed'}
Write-Host "Validation passed. Starting Tauri + Rust Sidecar..." -ForegroundColor Green
npm run dev
if ($LASTEXITCODE -ne 0) {throw 'Tauri launch failed. See installation guide.'}
