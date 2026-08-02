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
echo "== agent cohort contracts =="
python3 scripts/test-agent-cohort.py
echo "== agent hallway cohort =="
python3 scripts/agent-hallway.py
echo "== agent tactile cohort =="
python3 scripts/agent-tactile.py
echo "== agent first-contact suite =="
python3 scripts/agent-first-contact.py
echo "== flagship visual and audio goldens =="
python3 scripts/flagship-goldens.py
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
echo "== release SBOM contract =="
python3 scripts/test-release-sbom.py
echo "== release workflow contract =="
python3 scripts/test-release-workflow.py
echo "== dependency migration performance contract =="
python3 scripts/test-dependency-migration-performance.py
echo "== dependency migration performance receipt =="
python3 scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json
echo "== house style =="
bash scripts/check-style.sh
echo "All checks passed."
