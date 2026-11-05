# Build a signed-capable Windows install bundle after normal local verification.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
foreach ($tool in @('node','npm','cargo','rustc')) {
 if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {throw "Missing $tool"}
}
npm install
if ($LASTEXITCODE -ne 0) {throw 'npm install failed'}
npm run check:all
if ($LASTEXITCODE -ne 0) {throw 'TypeScript/test gate failed'}
npm run build
if ($LASTEXITCODE -ne 0) {throw 'Tauri build failed'}
Write-Host 'Bundle is expected under target/release/bundle/. Check output and code signing.' -ForegroundColor Green
