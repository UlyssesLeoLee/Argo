#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
node --test tests/ui.contract.test.mjs
if command -v cargo >/dev/null 2>&1; then
  cargo fmt --all -- --check
  cargo test --workspace
  python3 tests/integration_cluster.py
else
  echo 'SKIP Rust build/integration: cargo is not installed on this machine'
fi
