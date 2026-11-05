#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm test
cargo test -p argo-runtime
cargo check -p argo-desktop
python3 tests/integration_cluster.py
python3 tests/smoke_snapshot_patch.py
