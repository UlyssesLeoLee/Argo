#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm run typecheck
npm test
if command -v cargo >/dev/null 2>&1; then cargo test -p argo-runtime; else echo 'SKIP Rust tests: cargo unavailable'; fi
