#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
node --test tests/ui.contract.test.mjs tests/rag.contract.test.mjs tests/v03.contract.test.mjs
if command -v cargo >/dev/null 2>&1; then cargo test -p argo-runtime; else echo 'SKIP Rust tests: cargo unavailable'; fi
