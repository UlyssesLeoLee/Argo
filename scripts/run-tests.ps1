$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
node --test tests/ui.contract.test.mjs tests/rag.contract.test.mjs tests/v03.contract.test.mjs
if (Get-Command cargo -ErrorAction SilentlyContinue) {cargo test -p argo-runtime} else { Write-Host 'SKIP Rust tests: Cargo not installed' }
