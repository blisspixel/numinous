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
- Sixth room, `lissajous` (Waves & Sound, a fourth Wing): two perpendicular
  oscillations trace a figure that is stable at simple frequency ratios; `t`
  sweeps the second frequency.
- Seventh room, `prime-spirals` (Number & Pattern): the Ulam spiral; primes light
  up and fall into diagonal streaks; `t` shifts the starting number.
- Eighth room, `collatz` (Emergence): plots the log-scaled orbit of a starting
  number as it falls to 1 (the unproven 3n+1 conjecture); `t` picks the number.
- Ninth room, `buffon-needle` (Chance & Order): drops needles on a lined floor
  (crossing needles highlighted) and estimates pi from the crossing fraction, no
  circle in sight; `t` changes the needle length.
- GPU rendering (`crates/gpu`): an adaptive `wgpu` context that picks the
  machine's GPU (AMD, NVIDIA, Intel, or Apple, across Vulkan, Metal, and DX12,
  with a CPU fallback) and renders offscreen with no window. A first WGSL
  compute-shader workload renders the Mandelbrot set to a PNG, verified on the
  dev laptop's AMD Radeon 780M via Vulkan. The GPU crate is excluded from the
  coverage gate because it is integration-tested on real hardware.
- Audio (`crates/audio`): adaptive `cpal` output on the system default device,
  following the machine's sound settings across WASAPI, CoreAudio, and ALSA, with
  pure, tested sine synthesis kept separate from device I/O. A tone hello-world
  plays a 440 Hz sine and writes a WAV, verified on the dev laptop (Realtek at
  48 kHz, stereo). CI installs ALSA headers on Linux; the crate is excluded from
  the coverage gate (integration-tested on hardware).

### Changed
- Robustness hardening (from an independent code review): `Canvas` clamps its
  dimensions so an absurd size request cannot abort the process; the Galton Board
  caps its simulated bins and stretches them across wide canvases, so a huge-width
  render stays fast instead of hanging; `Canvas::line` steps in `i64` to avoid
  coordinate overflow; the CLI no longer uses `expect()` in a production path; and
  an `rng` doc comment was corrected. No behavior change for normal sizes.
