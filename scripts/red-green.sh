#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
printf '\n[Argo v0.5] Frontend / contract gates\n'
npm run check:all
printf '\n[Argo v0.5] Real Git tests\n'
python3 tests/smoke_snapshot_patch.py
python3 tests/adversarial_git_protocol.py
if command -v cargo >/dev/null 2>&1; then
  printf '\n[Argo v0.5] Rust tests and desktop compile\n'
  cargo test -p argo-runtime
  cargo check -p argo-desktop
  python3 tests/integration_cluster.py
else
  printf '\nSKIP (NOT PASS): cargo/rustc missing; Rust/Tauri/E2E must run on Windows/Linux build host.\n'
fi
