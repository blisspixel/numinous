# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- Headless core (`crates/core`): the `Room` trait, a deterministic ASCII `Canvas`
  with Bresenham line drawing, the room registry, and the flagship Times Tables
  room (modular multiplication on a circle).
- CLI face (`faces/cli`, binary `numinous`): `rooms`, `describe`, and `render`
  commands, with `--json` output.
- MCP face (`faces/mcp`, binary `numinous-mcp`): a JSON-RPC 2.0 stdio server with
  `initialize`, `tools/list`, and `tools/call` (`list_rooms`, `describe_room`,
  `play_room`), returning renders as text so a text-only mind can perceive them.
- Engineering foundation: Cargo workspace (edition 2024), workspace lints
  (forbid unsafe, deny-warnings-ready), pinned toolchain (1.96.0), rustfmt and
  cargo-deny config, a house-style guard, and GitHub Actions CI (fmt, clippy with
  `-D warnings`, tests, cargo-deny, and a three-OS build).
- Deterministic quality gates: local check runners (`scripts/check.sh`,
  `scripts/check.ps1`) mirroring CI, and a `cargo-llvm-cov` coverage job gated at
  80% lines. Refactored the CLI into pure, unit-tested report functions and
  broadened MCP tests; workspace line coverage is 92%. `crates/core` now denies
  missing documentation.
- Room revelations: the `Room` trait now carries `reveal()` (the short, true
  insight that reframes a room). Surfaced in the CLI `describe` output and JSON,
  in the MCP `describe_room` result, and via a new MCP `reveal_room` tool so an
  agent can ask for the deeper meaning.
- Second room, `cellular-automata` (Emergence): elementary Wolfram rules on a
  line, rendered as a space-time diagram; Rule 90 draws a Sierpinski triangle.
  It appears automatically in the CLI and MCP faces through the registry.
- Deterministic RNG (`crate::rng::SplitMix64`): seeded, reproducible randomness
  for rooms, so renders and tests are deterministic.
- Third room, `chaos-game` (Emergence): repeatedly jumping halfway to a random
  triangle corner resolves into a Sierpinski fractal, drawn from a fixed seed.
- Fourth room, `golden-angle` (Number & Pattern): Vogel's phyllotaxis model;
  at the golden angle the seeds pack into a sunflower spiral, and `t` detunes it.
- Fifth room, `galton-board` (Chance & Order): thousands of coin-flip balls tally
  into a bell curve (the Central Limit Theorem); `t` biases the coin.
