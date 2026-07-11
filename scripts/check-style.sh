#!/usr/bin/env bash
# House-style guard: no emojis, no em/en dashes, no AI/tool attribution.
# Enforced in CI and by the pre-commit hook (see docs/QUALITY.md and
# docs/ENGINEERING.md).
set -euo pipefail

# `grep -P` needs a UTF-8 (or single-byte) locale to interpret the Unicode
# escapes below; in a bare C/POSIX locale it aborts with "supports only unibyte
# and UTF-8 locales". If that abort were mistaken for "no match", the guard would
# silently pass everything (it did, in shells with an unset locale). So pick a
# UTF-8 locale up front, and treat a grep error as a hard failure below.
for candidate in "${LC_ALL:-}" "${LANG:-}" C.UTF-8 C.utf8 en_US.UTF-8 en_US.utf8; do
  case "$candidate" in
    *.[Uu][Tt][Ff]-8 | *.[Uu][Tt][Ff]8)
      if locale -a 2>/dev/null | grep -qix "$candidate"; then
        export LC_ALL="$candidate"
        break
      fi
      ;;
  esac
done

mapfile -t files < <(git ls-files '*.rs' '*.md' '*.toml' '*.wgsl' '*.sh' '*.ps1')
if [ ${#files[@]} -eq 0 ]; then
  exit 0
fi

fail=0
check() {
  local pattern="$1" label="$2" out status
  # Distinguish "found a match" (rc 0, a violation) from "no match" (rc 1, clean)
  # from "grep could not run" (rc >= 2), which must fail loudly rather than be
  # read as clean.
  set +e
  out=$(grep -rInP "$pattern" "${files[@]}")
  status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    echo "House-style violation (${label}):"
    echo "$out"
    fail=1
  elif [ "$status" -ge 2 ]; then
    echo "House-style check could not run (${label}): grep exited ${status}."
    echo "Set a UTF-8 locale (for example LC_ALL=C.UTF-8) and retry."
    fail=1
  fi
}

check '\x{2014}|\x{2013}' "em/en dash"
check '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2728}]' "emoji"
attr_a='co-'
attr_b='authored-by:'
tool_a='cla'
tool_b='ude'
tool_c='co'
tool_d='dex'
check "(?i)${attr_a}${attr_b}|generated with (${tool_a}${tool_b}|${tool_c}${tool_d})" "AI/tool attribution"

if [ "$fail" -ne 0 ]; then
  echo ""
  echo "Fix the above before merging. House style: no emojis, no em-dashes, no AI/tool attribution."
fi
exit "$fail"
