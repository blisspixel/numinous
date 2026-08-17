#!/usr/bin/env bash
# House style for the commit messages this branch adds.
#
# The three hard rules apply to everything produced, "including commit messages
# and PR descriptions, not only files". The tree-wide guard in
# `scripts/check-style.sh` has always covered the files. This covers the half a
# later edit cannot reach: a commit message is fixed only by rewriting published
# history, which changes every downstream hash and breaks the provenance a
# released archive pins.
#
# The commit-msg hook catches this one second before a message is written. This
# script is the same check for anyone who has not run
# `git config core.hooksPath scripts/hooks`, so the rule holds without depending
# on a contributor's local setup.
#
# Usage:
#     scripts/check-commit-messages.sh [BASE_REF]
#
# With a base, checks the commits this branch adds on top of it, which is what
# a pull request is. Without one, falls back to the default branch, and then to
# the most recent commits, so it is useful locally too.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# How far back to look when there is no base to compare against. Enough to
# cover an ordinary local branch without walking the whole history.
FALLBACK_COMMITS=30

base="${1:-}"
if [ -n "$base" ] && git rev-parse --quiet --verify "$base^{commit}" > /dev/null; then
  range="$base..HEAD"
elif git rev-parse --quiet --verify origin/main > /dev/null; then
  range="origin/main..HEAD"
else
  range="HEAD~$(git rev-list --count HEAD | awk -v n="$FALLBACK_COMMITS" \
    '{ print ($1 < n ? $1 - 1 : n) }')..HEAD"
fi

mapfile -t commits < <(git rev-list "$range")
if [ ${#commits[@]} -eq 0 ]; then
  echo "No commit messages to check in $range."
  exit 0
fi

fail=0
scratch=$(mktemp)
trap 'rm -f "$scratch"' EXIT
for commit in "${commits[@]}"; do
  # A dependency bot signs its own commits with its own name, which is honest
  # authorship rather than a tool being credited for work it did not do, and
  # rewriting it would misattribute the change to a person. Everything else is
  # held to the rule.
  author=$(git show -s --format='%ae' "$commit")
  case "$author" in
    *'[bot]'*) continue ;;
  esac
  git show -s --format='%B' "$commit" > "$scratch"
  if ! bash scripts/check-style.sh --text "$scratch" > /dev/null 2>&1; then
    echo "House-style violation in commit message $commit:"
    git show -s --format='  %h %s' "$commit"
    bash scripts/check-style.sh --text "$scratch" 2>&1 | sed 's/^/  /'
    echo ""
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "A commit message cannot be fixed by an ordinary edit. Reword the commit"
  echo "before merging: git rebase -i, or git commit --amend for the tip."
  exit 1
fi

echo "Checked ${#commits[@]} commit message(s) in $range: clean."
