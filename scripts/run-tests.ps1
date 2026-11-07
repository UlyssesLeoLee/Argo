$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
npm run typecheck
npm test
if (Get-Command cargo -ErrorAction SilentlyContinue) {cargo test -p argo-runtime} else { Write-Host 'SKIP Rust tests: Cargo not installed' }
