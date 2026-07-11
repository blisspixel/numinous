# Agent guide

Instructions for any agent (or person) making changes to this repository. Read
this first. The deeper standards live in `docs/ENGINEERING.md`; this file is the
short, non-negotiable summary and the house rules that are easy to get wrong.

## The project in one line

Numinous is a Rust workspace: a headless core (`crates/core`) with three faces
(the app, the CLI, the MCP server in `faces/`), plus `crates/gpu` and
`crates/audio`. Start with `README.md`, then `docs/README.md` for the map. To
play it, `PLAY.md`.

## House rules (non-negotiable)

These are enforced, and violating them fails the gate. They apply to
**everything you produce**: source, comments, docs, commit messages, and PR
descriptions alike.

1. **No AI or tool attribution, anywhere, ever.** Nothing you produce is signed
   by a tool: no "by Codex", no "by Claude", no co-author trailers, no session
   links, no "generated with" note. This applies to commit messages and PR
   descriptions as much as to files. The work stands on its own.
2. **No em-dashes or en-dashes.** Use a comma, a colon, or a rewrite. The
   characters U+2014 and U+2013 must not appear in any tracked file or commit
   message.
3. **No emojis.** Anywhere.

The file-level checks are automated: `scripts/check-style.sh` (and
`scripts/check-style.ps1` on Windows) scan tracked files for dashes, emojis, and
attribution, and they run in CI and in the pre-commit hook. Commit messages are
not scanned by that guard, so keeping messages clean of attribution, dashes, and
emojis is on you: it is a hard project rule, not a nicety.

## Quality bar (the anti-slop standard)

Match the bar in `docs/ENGINEERING.md`. In short: no meaningfully duplicated
domain logic, no placeholders or TODOs in a final commit, no dead or
commented-out code, lint clean (`clippy -D warnings`), and 80%+ meaningful test
coverage with no regression. `unsafe_code` is forbidden. Comments are accurate
and humble: shipping code is not the same as code that works well, so do not
claim more than the evidence supports. When the docs do not specify something,
write the code a CS professor would be proud of: correct, simple, principled.

## Enable the local gate (once per clone)

```
git config core.hooksPath scripts/hooks
```

The pre-commit hook then blocks any commit that would fail the fast gate:
house-style on every commit, and the cargo gate (`fmt`, `clippy -D warnings`,
tests) when Rust, `Cargo.*`, or a shader changes. The full release gate,
including coverage and the locked build, is `scripts/verify.sh` (Windows:
`scripts/verify.ps1`). Run it before you push.

## Where things live

- Source stays in `crates/` and `faces/`, never flat in the root.
- User-facing docs live in `docs/`; the root keeps only the standard entry
  files (`README.md`, `AGENTS.md`, `CLAUDE.md`, `PLAY.md`, `VERIFY.md`,
  `CHANGELOG.md`, `LICENSE`, and the Cargo and config roots).
- `.agent/` (agent working files) and `logs/` and `renders/` are gitignored and
  must never be committed.

## When you finish a change

Update `CHANGELOG.md` (the `[Unreleased]` section) and, if you completed a
roadmap item, mark it in `docs/ROADMAP.md` with evidence. Keep commits small and
focused, with a clear imperative subject and a body that explains the why, and
with none of the attribution, dashes, or emojis named above.
