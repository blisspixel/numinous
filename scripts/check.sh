#!/usr/bin/env bash
# Fast local quality gate. Use verify.sh for coverage, build, and artifacts.
# Requires cargo on PATH. See docs/ENGINEERING.md.
set -euo pipefail

echo "== fmt =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== test =="
cargo test --workspace --all-targets --locked
echo "== house style =="
bash scripts/check-style.sh
echo "All checks passed."
