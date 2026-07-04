#!/usr/bin/env bash
# Local quality gate, mirroring CI. Run before pushing.
# Requires cargo on PATH. See docs/ENGINEERING.md.
set -euo pipefail

echo "== fmt =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== test =="
cargo test --workspace
echo "== house style =="
bash scripts/check-style.sh
echo "All checks passed."
