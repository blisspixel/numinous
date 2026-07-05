# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- The Studio's expression engine (`crates/core` `studio`): a small, safe
  recursive-descent parser and evaluator for single-variable expressions in `x`
  (`+ - * / ^`, unary minus, `sin cos tan exp ln abs sqrt`, and `pi`/`e`), the
  Tier 1 safe-DSL seed of the creative graphing calculator. `numinous plot
  "sin(3*x) + x/2"` parses it and draws the curve; the engine is unit-tested for
  precedence, associativity, functions, and errors.
- Agents play too (MCP): three new tools so a digital mind can use the same
  content as a human. `list_sims` and `run_sim` steer the simulations by lever
  (fiddle to optimize or break them, and read the outcome), and `quiz` plays
  Guess the Shape (call for the puzzle, call again with a guess letter to be
  graded). Seven MCP tools now.
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

- New wing, Fractals and the Infinite, with three rooms:
  - `mandelbrot`: escape-time render of the Mandelbrot set; `t` zooms toward the
    seahorse valley.
  - `julia`: the Julia family with the same iteration but a fixed, morphing `c`;
    `t` walks `c` around a circle.
  - `barnsley-fern`: an iterated function system that grows a fern from four
    random affine maps; `t` grows it by adding points.
- `harmonograph` (Waves & Sound): the curve a decaying two-pendulum machine
  draws; `t` detunes the frequencies.
- New wing, Chaos & Order, with `logistic-map`: the bifurcation diagram of
  `x -> r*x*(1-x)`, order splitting into chaos; `t` zooms into the cascade.
- `langtons-ant` (Emergence): an ant that makes chaos for ten thousand steps then
  builds a highway; `t` runs the clock.
- Guess the Shape quiz (`crates/core` `quiz`, `numinous quiz`): a deterministic
  "name the math behind this mystery render" game, shared by every face so the
  CLI, the app, and agents over MCP can all play the same seeded round.
- `docs/PLAYFUL.md`: the design of the games and the Studio (Guess the Shape,
  Shape to Function via Fourier epicycles, the high-Wolfram ethos) across faces,
  plus the four-personas design (PhD nerd, stoner, aesthete, gamer).
- `lorenz` (Chaos & Order): the Lorenz attractor and the butterfly effect; `t`
  sweeps the parameter through the onset of chaos.
- `arecibo` (new Signals & Codes wing): a bitstream that looks like noise until
  you line it up at the one width its semiprime length allows (143 = 11 x 13);
  `t` hunts for the width and the hidden picture snaps into focus. 19 rooms.
- Base-N aliens: Talk to the Aliens transmissions can arrive in base 2, 8, or 16
  (a different number of fingers), so you translate before you answer.
- SETI detection game (`crates/core` `seti`, `numinous seti`): the step before
  talking. Scan channels of static near the hydrogen line and pick the one
  artificial signal (counting in primes) out of the regular pulsars and noise;
  nature makes rhythms, but only minds count in primes.
- A hidden Cult of Pythagoras easter egg (`crates/core` `secret`): a few unlisted
  names (`hippasus`, `tetractys`, `pythagoras`, `harmonia`, `odd`, ...) answer
  `numinous describe` with an akousma in the Order's voice instead of a not-found
  error. Never announced; found by knowing. See `docs/LORE.md`.
- Design capture in `docs/PLAYFUL.md`: the music visualizer plan (system-audio
  loopback plus FFT driving room parameters), the physical-made-digital rooms
  (Mobius, hexaflexagon, hyperbolic plane), the puzzle set (Nonograms, the Hat
  monotile, fractal zoomer), the alien-contact kit (Arecibo, Rosetta, base-N), and
  the digital-mind playground (manifold folding, chaos surfing, proof graphs).
- Two more mini-games, each seeded and shared across faces via the core:
  - Crack the Code (`crates/core` `codebreaker`, `numinous crack`): defuse a
    math-clued bomb, Bulls and Cows with a digit-sum-and-parity opening clue.
  - Talk to the Aliens (`crates/core` `aliens`, `numinous aliens`): continue the
    first-contact number sequences (primes, Fibonacci, powers of two, and more).

- Sims (`crates/core` `sim`): a multi-lever interactive-simulation abstraction
  (each lever has a range, default, and unit), separate from the single-knob
  Room. A sim renders a picture and returns a plain-language readout of the
  outcome (the optimization or the joke). Registry, `numinous sims` to list, and
  `numinous sim <id> --set lever=value` to run. First three sims:
  - `tribbles`: a logistic population that goes from a purring carpet to
    boom-and-bust chaos when you crank the breeding rate.
  - `wing`: lift versus angle of attack with a real stall past fifteen degrees
    ("you are now a lawn dart").
  - `black-hole`: Schwarzschild radius, time dilation, and spaghettification, with
    an event horizon and photon ring drawn to scale.
  - `supernova`: the star's mass decides its corpse, white dwarf, neutron star,
    or black hole (Chandrasekhar and TOV limits).
  - `big-bang`: the density omega decides the fate, expand forever, flat, or a Big
    Crunch (a numerically integrated Friedmann scale factor).
  - `carburetor`: tune the air-fuel mix from flooded (too rich) to backfiring (too
    lean); best power at 12.6:1, cleanest at 14.7:1.

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
