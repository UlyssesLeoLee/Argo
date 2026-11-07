$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
Write-Host '[Argo v0.5] TypeScript + red/green contracts'
npm run check:all
if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed' }
python tests/smoke_snapshot_patch.py
if ($LASTEXITCODE -ne 0) { throw 'Git snapshot tests failed' }
python tests/adversarial_git_protocol.py
if ($LASTEXITCODE -ne 0) { throw 'Git adversarial tests failed' }
if (Get-Command cargo -ErrorAction SilentlyContinue) {
  cargo test -p argo-runtime
  if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
  cargo check -p argo-desktop
  if ($LASTEXITCODE -ne 0) { throw 'Tauri compilation failed' }
  python tests/integration_cluster.py
  if ($LASTEXITCODE -ne 0) { throw 'Cross-node E2E failed' }
} else {
  Write-Warning 'SKIP (NOT PASS): cargo/rustc absent. Run Rust/Tauri/E2E on a build host.'
}
