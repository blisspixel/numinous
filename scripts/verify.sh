#!/usr/bin/env bash
# One-command verification (macOS/Linux): runs every gate and regenerates all
# artifacts into renders/. See VERIFY.md.
set -euo pipefail

echo "== format =="
cargo fmt --all --check
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== tests =="
cargo test --workspace

if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "== coverage =="
    cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex 'crates[\\/](gpu|audio)[\\/]'
else
    echo "== coverage == (skipped: run 'cargo install cargo-llvm-cov' to enable)"
fi

echo "== house-style =="
bash scripts/check-style.sh

echo "== regenerate artifacts into renders/ =="
cargo run -q --bin numinous -- gallery --dir renders --width 600 --height 600
cargo run -q --bin numinous -- contact-sheet --out renders/contact.png --cols 3 --tile 360
cargo run -q --bin numinous -- sonify lissajous --out renders/lissajous.wav
cargo run -q --bin numinous -- sonify collatz --out renders/collatz.wav

echo ""
echo "All checks passed. Open renders/contact.png; renders/*.wav are the room sounds."
