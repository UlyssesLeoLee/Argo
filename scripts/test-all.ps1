$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
npm test
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust toolchain is required for integration testing' }
cargo test -p argo-runtime
cargo check -p argo-desktop
python tests/integration_cluster.py
python tests/smoke_snapshot_patch.py
