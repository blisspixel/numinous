#!/usr/bin/env bash
# One-command verification (macOS/Linux): runs every gate and regenerates all
# artifacts into renders/. See VERIFY.md.
set -euo pipefail

verify_state=".agent/verify"
mkdir -p "$verify_state"
export NUMINOUS_JOURNEY="$verify_state/journey.txt"
export NUMINOUS_SCORES="$verify_state/scores.txt"
export NUMINOUS_CAIRN="$verify_state/cairn.txt"

echo "== format =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== documentation =="
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc --locked
echo "== tests =="
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
echo "== creator roundtrip =="
python3 scripts/creator-roundtrip.py
python3 scripts/reduced-motion.py
cargo test -p numinous-core --release --lib -- --ignored --exact registry::tests::no_catalog_room_flashes_past_the_photosensitivity_budget
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
echo "== build =="
cargo build --workspace --locked

if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "== coverage =="
    cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex '(crates[\\/](gpu|audio)[\\/]|faces[\\/]app[\\/]src[\\/]main\.rs)'
else
    echo "== coverage == (skipped: run 'cargo install cargo-llvm-cov' to enable)"
fi

if command -v cargo-deny >/dev/null 2>&1; then
    echo "== supply-chain (cargo-deny) =="
    cargo deny check
else
    echo "== supply-chain (cargo-deny) == (skipped: run 'cargo install cargo-deny' to enable; CI enforces it)"
fi

if command -v cargo-audit >/dev/null 2>&1; then
    echo "== supply-chain (cargo-audit) =="
    cargo audit
else
    echo "== supply-chain (cargo-audit) == (skipped: run 'cargo install cargo-audit' to enable; CI enforces it)"
fi

echo "== house-style =="
bash scripts/check-style.sh
echo "== POSIX installer safety =="
bash scripts/install.sh --self-test

echo "== regenerate 2,913-screen app QA matrix =="
cargo run -q -p numinous-app --example screens
echo "== regenerate remaining artifacts into renders/ =="
cargo run -q --bin numinous -- gallery --dir renders --width 600 --height 600
cargo run -q --bin numinous -- contact-sheet --out renders/contact.png --cols 3 --tile 360
cargo run -q --bin numinous -- sonify lissajous --out renders/lissajous.wav
cargo run -q --bin numinous -- sonify collatz --out renders/collatz.wav
cargo run -q --bin numinous -- sonify lissajous --layer room-bed --out renders/lissajous-bed.wav

echo ""
echo "All checks passed. Open renders/contact.png; lissajous-bed.wav is the room-bed PCM16 projection."
