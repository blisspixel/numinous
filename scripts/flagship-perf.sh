#!/usr/bin/env bash
# Reference-machine performance gate for the five 0.3 flagships.
set -euo pipefail

cargo run --release --locked -p numinous-app --example flagship_perf -- --check
