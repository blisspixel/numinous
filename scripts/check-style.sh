#!/usr/bin/env bash
# House-style guard: no emojis, no em/en dashes, no AI/tool attribution.
# Enforced in CI (see docs/QUALITY.md and docs/ENGINEERING.md).
set -euo pipefail

mapfile -t files < <(git ls-files '*.rs' '*.md' '*.toml' '*.wgsl' '*.sh')
if [ ${#files[@]} -eq 0 ]; then
  exit 0
fi

fail=0
check() {
  local pattern="$1" label="$2"
  if grep -rInP "$pattern" "${files[@]}" >/dev/null 2>&1; then
    echo "House-style violation (${label}):"
    grep -rInP "$pattern" "${files[@]}" || true
    fail=1
  fi
}

check '\x{2014}|\x{2013}' "em/en dash"
check '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2728}]' "emoji"
check '(?i)co-authored-by:|generated with (claude|codex)' "AI/tool attribution"

if [ "$fail" -ne 0 ]; then
  echo ""
  echo "Fix the above before merging. House style: no emojis, no em-dashes, no AI/tool attribution."
fi
exit "$fail"
