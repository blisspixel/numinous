#!/usr/bin/env bash
# Fast local quality gate. Use verify.sh for coverage, build, and artifacts.
# Requires cargo on PATH. See docs/ENGINEERING.md.
set -euo pipefail

echo "== fmt =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== docs =="
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc --locked
echo "== test =="
cargo test --workspace --all-targets --locked
echo "== MCP play driver =="
python3 scripts/test-mcp-play.py
echo "== understanding study runner =="
python3 scripts/test-understanding-study.py
echo "== understanding study collector =="
python3 scripts/test-understanding-collect.py
echo "== release packaging =="
python3 scripts/test-package-release.py
echo "== release engagement contract =="
python3 scripts/test-release-engagement-smoke.py
echo "== physical input session contract =="
python3 scripts/test-input-hardware-session.py
echo "== house style =="
bash scripts/check-style.sh
echo "All checks passed."
