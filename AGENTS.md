# Agent guide

Instructions for any agent (or person) making changes to this repository. Read
this first. The deeper standards live in `docs/ENGINEERING.md`; this file is the
short, non-negotiable summary and the house rules that are easy to get wrong.

## The project in one line

Numinous is a Rust workspace: a headless core (`crates/core`) with three faces
(the app, the CLI, the MCP server in `faces/`), plus `crates/gpu` and
`crates/audio`. Mathematics as a shared language, made playable, for humans
and digital minds as peers. The rooms are the product. Studio multiplies
them. Progression is ceremony and never a gate on play or study.

Start with `README.md`, then `docs/README.md` for the map. To play it,
`PLAY.md`. Current work lives in `docs/ROADMAP.md`. If README and ROADMAP
disagree on what is built or next, ROADMAP wins. Direction is
`docs/NORTH_STAR.md`; the definition of no is `docs/SCOPE.md`.

## House rules (non-negotiable)

These are enforced, and violating them fails the gate. They apply to
**everything you produce**: source, comments, docs, commit messages, and PR
descriptions alike.

1. **No AI or tool attribution, anywhere, ever.** Nothing you produce is signed
   by a tool: no tool names in authorship claims, no co-author trailers, no
   session links, no "generated with" note. This applies to commit messages and PR
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
claim more than the evidence supports. `docs/RESEARCH.md` defines Built,
Measured, Observed, Designed, and Hypothesis; do not write Designed work as
Built. When the docs do not specify something, write the code a CS professor
would be proud of: correct, simple, principled.

## Canonical seams

Product truth lives in `numinous-core`: rooms, grading, persistence, Studio
capsules, Journey, study content, and protocol-neutral request types. The
three faces translate transport and presentation. They do not reimplement
rules. Python in `scripts/` drives compiled binaries as black boxes and must
not become a second owner of domain facts.

Before adding a shared helper, parser, persistence path, or catalog, find the
existing one. Face-local adapters are for transport. Do not reopen the stack:
the toolchain, edition, and crate versions are pinned in `rust-toolchain.toml`
and `docs/ENGINEERING.md`.

## What a downloaded player can actually read

A release archive carries `PLAY.md`, `README.md`, `VERIFY.md`, and
`plugins/numinous/skills/play-numinous/SKILL.md`. Nothing under `docs/` ships
in that archive. A player-facing fact, including MCP `next` pointers and
bundled experiment ids, has to live in one of those four files or it is not
documented for a packaged player. `docs/PLAYING.md` is the full manual for a
clone, not a substitute.

A structured `next` field is a followable tool call: `tool` plus `arguments`.
If following it verbatim does not work, it is not a door. Lock that with a
regression that follows the pointer. Detail is in `docs/ENGINEERING.md`.

## Enable the local gate (once per clone)

```
git config core.hooksPath scripts/hooks
```

The pre-commit hook then blocks any commit that would fail the fast floor:
house-style on every commit, and the cargo gate (`fmt`, `clippy -D warnings`,
rustdoc with warnings denied, tests) when Rust, `Cargo.*`, or a shader
changes. A commit-msg hook holds the message itself to the same three rules,
because the rules cover commit messages as much as files and a message is the
half an ordinary edit cannot reach later: fixing one means rewriting published
history, which changes every downstream hash and breaks the provenance a
released archive pins. CI runs the same check over the commits a pull request
adds, so the rule does not depend on anyone remembering this setup.

To run that floor yourself:

- Windows: `scripts\check.ps1`
- macOS / Linux: `bash scripts/check.sh`

Those scripts also run the Python harness contracts (MCP play, agent
hallway and tactile, goldens, packaging). The hook only pays for those when
the matching scripts change. Focused work can start with `cargo test -p
<crate>` for the crate you touched, then the floor.

The full release gate, including coverage and the locked build, is
`scripts/verify.sh` (Windows: `scripts/verify.ps1`). Run it before you push.
The command list and optional tools are in `VERIFY.md`. Do not make a check
pass by weakening it.

## Where things live

- Source stays in `crates/` and `faces/`, never flat in the root.
- User-facing docs live in `docs/`; the root keeps only the standard entry
  files (`README.md`, `AGENTS.md`, `CLAUDE.md`, `PLAY.md`, `VERIFY.md`,
  `CHANGELOG.md`, `LICENSE`, and the Cargo and config roots).
- `.agent/` (agent working files) and `logs/` and `renders/` are gitignored and
  must never be committed.

## When you finish a change

Update `CHANGELOG.md` (the `[Unreleased]` section) and, if you completed a
roadmap item, mark it in `docs/ROADMAP.md` with evidence. If the change is
player-facing, check the four packaged files, not only `docs/`. If it adds a
count in prose, route it through `numinous_core::counted` and lock it against
live data. If it adds an MCP `next`, follow it in a test. Keep commits small
and focused, with a clear imperative subject and a body that explains the why,
and with none of the attribution, dashes, or emojis named above. Temporary
agent scratch stays in gitignored `.agent/`; durable knowledge belongs in
source, tests, or tracked docs.
