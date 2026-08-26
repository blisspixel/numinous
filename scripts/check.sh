#!/usr/bin/env bash
# Fast local quality gate. Use verify.sh for coverage, build, and artifacts.
# Requires cargo on PATH. See docs/ENGINEERING.md.
set -euo pipefail

echo "== fmt =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== GPU post and App presentation =="
cargo clippy -p numinous-gpu --all-features --all-targets -- -D warnings
cargo test -p numinous-gpu --all-features --all-targets --locked
cargo clippy -p numinous-app --all-features --all-targets -- -D warnings
cargo test -p numinous-app --all-features --all-targets --locked
echo "== docs =="
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc --locked
echo "== test =="
cargo test --workspace --all-targets --locked
echo "== MCP play driver =="
python3 scripts/test-mcp-play.py
echo "== local agent playtest contract =="
python3 scripts/test-local-agent-playtest.py
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
echo "== creator roundtrip =="
python3 scripts/creator-roundtrip.py
echo "== creator parity =="
python3 scripts/creator-parity.py
echo "== reduced motion contract =="
python3 scripts/test-reduced-motion.py
echo "== reduced motion =="
python3 scripts/reduced-motion.py
echo "== no color contract =="
python3 scripts/test-no-color.py
echo "== no color =="
python3 scripts/no-color.py
echo "== one gate resolver =="
python3 scripts/test-gate-cli.py
echo "== am soak contract =="
python3 scripts/test-am-soak.py
echo "== am soak =="
python3 scripts/am-soak.py
echo "== catalog scorecard =="
python3 scripts/catalog-scorecard.py
echo "== understanding am dry-run =="
python3 scripts/test-understanding-am.py
echo "== understanding am registration audit =="
python3 scripts/understanding-am-pipeline.py --check-only docs/evidence/understanding-0.4/registration-dry-run.json
echo "== understanding study runner =="
python3 scripts/test-understanding-study.py
echo "== understanding study collector =="
python3 scripts/test-understanding-collect.py
echo "== release packaging =="
python3 scripts/test-package-release.py
echo "== portable agent plugin =="
python3 scripts/test-agent-plugin.py
echo "== Sensory Lift platform proof contract =="
python3 scripts/test-sensory-platform-proof.py
echo "== Sensory Lift physical set contract =="
python3 scripts/test-sensory-platform-set.py
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
