# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- Windowed app (`faces/app`, binary `numinous-app`): a real, resizable window
  that shows a room animating in full color, rendered on the CPU via the shared
  `Raster`, using `winit` for the window and `softbuffer` for a
  toolkit-free pixel blit. Left/right switch rooms, space pauses, escape quits.
  Cross-platform (macOS/Linux/Windows); verified launching on the dev laptop.
- Live sound in the windowed app: a `LoopPlayer` (`crates/audio`) loops the
  visible room's `SoundSpec` through the system default device, updated when you
  switch rooms, so the app is audiovisual (you see and hear the same room).
- Mouse-drag phase scrubbing in the app: drag horizontally to sweep the room's
  phase directly (pausing the auto-animation), with the sound following the drag.
- On-screen HUD: a tiny 5x7 bitmap font (`crates/core` `font`, no external font
  dependency) draws the room title in the window, and the `i` key toggles the
  room's reveal (word-wrapped) over the visualization in the room's accent color.
  A `font_preview` example renders the glyphs to the terminal.
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

- A `Surface` drawing abstraction (`crates/core`): rooms render through
  `&mut dyn Surface`, so the same room logic draws to the ASCII `Canvas` and to an
  RGBA `Raster` (CPU, deterministic, no GPU). The Bresenham line drawing lives
  once and is shared by every surface.
- PNG output: `numinous render <room> --out image.png` renders any room to a real
  image (additive glow on a near-black stage), verified on the dev laptop.
- Per-surface aspect (`Surface::char_aspect`): circular rooms render round on
  square pixels while staying correct in the terminal (characters are tall).
- Per-room accent colors (`RoomMeta.accent`): each room has a signature color the
  `Raster` draws in, so image renders are distinct and on-brand.
- Room sonification (`crates/core` `SoundSpec`): every room can describe its own
  sound as timed sine notes, rendered to samples device-free (deterministic,
  testable). `Room::sound` defaults to a rising tone; Lissajous plays its two
  frequencies as a chord, Times Tables pitches with the multiplier, and Collatz
  plays its orbit as a melody. `numinous sonify <room> --out file.wav` writes it.
- `numinous gallery --dir <dir>` renders every room to a PNG at once, a showcase
  and a beauty-QA sweep of the whole collection.
- Tenth room, `game-of-life` (Emergence): Conway's Game of Life on a toroidal
  grid; `t` sweeps the generation, so the life evolves; verified with still-life
  and blinker (oscillator) tests.
- `numinous contact-sheet` tiles every room into one image (via `Raster::blit`),
  the fastest way to eyeball the whole collection; each tile is labeled with the
  room name using the bitmap font.
- Verification kit: `VERIFY.md` plus `scripts/verify.ps1` and `scripts/verify.sh`
  run every gate and regenerate all images and sounds in one command.
- `numinous play <room>` animates a room live in the terminal (the Watch mode of
  the Teletype face), sweeping its phase until Ctrl+C. The per-frame builder is a
  pure, tested function.

### Changed
- Rooms render through `Surface` instead of a concrete `Canvas` (the `render`
  method replaces `render_ascii`), which is what lets one room target both the
  terminal and an image (and, later, the GPU).
- Robustness hardening (from an independent code review): `Canvas` clamps its
  dimensions so an absurd size request cannot abort the process; the Galton Board
  caps its simulated bins and stretches them across wide canvases, so a huge-width
  render stays fast instead of hanging; `Canvas::line` steps in `i64` to avoid
  coordinate overflow; the CLI no longer uses `expect()` in a production path; and
  an `rng` doc comment was corrected. No behavior change for normal sizes.
