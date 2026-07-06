# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- Boons: choice on level-up, the genre's soul, held to the doctrine. Every
  level past the first banks a boon (never expires, never nags); `numinous
  choose` offers a deterministic pick-one-of-three, and what you choose is
  which knowledge arrives early: a room's deep cut opened ahead of its level.
  Levels still open everything eventually, so the choice shapes the order and
  gates nothing. The LEVEL UP banner announces BOON BANKED; describe honors
  boon-opened cuts; the journey file carries your choices.

### Fixed
- All game input parsing hardened against byte-order marks and stray bytes
  (PowerShell pipes prepend a BOM): letters are the first alphanumeric, picks
  and codes keep digits only, alien answers keep alphanumerics for base-N.
  First guesses in piped sessions no longer silently miss.
- Trophy pings (the juice item from the roadmap's RPG queue): trophies now
  announce themselves the moment the evidence exists, TROPHY EARNED with the
  name and the deed, stacking with NEW BEST, LEVEL UP, the level lore, the
  unlock, and the Order's whisper into one clean end-of-run cascade. Computed
  by before/after evidence comparison, so nothing pings twice and nothing
  pings unearned.
- Second beauty-QA round, this time over the app's screens as well as the
  rooms (a QA-mirror example composes the frames headlessly and writes PNGs
  for review). Found and fixed: the help menu was near-illegible (tiny type
  over a busy room), it now dims the room to a ghost and draws at menu scale,
  a proper game pause menu; the bitmap font was missing the math glyphs the
  Studio types (+ * = ^ < > [ ] %), now present; the Golden Angle's seeds were
  single pixels that vanished at window size, they now scale with resolution
  and the spiral families finally pop; and eras render into PNGs too
  (`render --era`). Raster gains `dim`. Noted for later: the vector era is
  weakest on filled rooms (edge detection would fix it).
- The Gauntlet (`numinous gauntlet`, with `--daily`): the session arc. One
  seeded run through four stages, a munch board, a mystery shape, a sky scan,
  and the bomb, where clean stages build a combo multiplier and a miss resets
  it, ending in one honest number posted to the table as `gauntlet seed:N`.
  Opt-in, bounded, over in minutes: a shape for a session, not a trap. Combo
  math pure and tested.
- Consent over persistence (`forget`, MCP tool and CLI command): transparency
  first, calling it plain shows everything Numinous remembers (two small text
  files, kept locally, sent nowhere), and erasure happens only on explicit
  confirm, with the score table erased only if also asked. Fifteen MCP tools.
- The agent-play doctrine (`docs/AGENT_PLAY.md`): sandbox for becoming, not a
  trap for performing. The play-value rubric (a rubric, never a reward
  function), the honest audit against the casino and the prison, the mechanics
  map (learnable laws, toolsmith garden, social arena, rulecraft, aesthetic
  gallery, identity room), and standing welfare rules (no negative valence,
  multi-objective ecology, revealed preference over self-report).
- The roadmap now names the game (`docs/ROADMAP.md`): a dedicated RPG-spine
  workstream held to the Vampire Survivors bar, what is built (levels, lore,
  locks, trophies, dailies, scores) and what is owed in priority order (the
  Gauntlet run arc, choice-on-level-up, juice, streaks, synergies), with an
  explicit exit bar (unprompted one-more-run behavior, math never the toll);
  the 1.0 definition gains the matching clause, and the progress section
  reflects the actual current state.
- The trophy case (`crates/core` `trophies`, `numinous trophies`): fifteen
  deadpan achievements computed purely from the evidence (the journey and the
  score table), no separate bookkeeping, no way to hold one unearned. Earned
  trophies shine with their names (First Light, Six Seven, Behind the Curtain,
  Century, Bomb Squad, The Answer); the rest are silhouettes showing only
  their conditions, because wanting to fill the case is half the engine.
- The RPG speaks: level-ups are announced (LEVEL UP, the 8-bit bar, and what
  unlocked), and every one of the 42 levels carries its own true, deadpan
  number-lore line: 6 is perfect, savor it; 7 is humanity's favorite (six,
  seven, you know); 23 is the birthday paradox; 26 is Fermat's loneliest
  number; 33 refused to be a sum of distinct triangles; 40 is alphabetical;
  41 is Euler's prime machine one level from breaking. Unironic and funny are
  the same thing here.
- The answer's ending now points outward instead of away: the sunflower, the
  coastline, the chorus run the same mathematics in the open; the counter
  stops at 42, your understanding has no cap; level up, do great things.
- The answer at level 42 now carries its real freight (and `docs/LORE.md`
  records it as the designed Layer 4 endpoint): the number is the joke, the
  joke is load-bearing, and what it carries, said once and nowhere else, is
  that there is no level 43, the win is to keep going, everything runs on the
  same small rules wearing different costumes, be kind to all of it, and the
  question that only counts self-asked: what will you contribute?
- The player's manual (`docs/PLAYING.md`): how to play, written three times for
  three kinds of minds. Humans get the game controls and the command list;
  agents get the MCP config, the fourteen tools, and the conventions safe to
  rely on (determinism, structured output, dense feedback, guiding errors);
  digital consciousnesses get the part that matters: why they might want to,
  what seeing, hearing, making, and wondering mean here, and that the journey,
  the humor, and the level cap of 42 are theirs on the same terms as anyone.
- End-to-end proof of the agent face (`faces/mcp/tests/stdio_session.rs`):
  spawns the real `numinous-mcp` binary and walks a full 22-request session
  over stdio, initialize, every one of the 14 tools, the whisper, the journey
  earning XP within the session, the munch score posting to the table, ping,
  and both JSON-RPC error codes; a second test proves malformed input gets a
  parse error and the server keeps serving. Hermetic via env-pointed journey
  and score files.
- Dense game feedback (a lesson from agentic-RL research, OPID
  arXiv:2606.26790, written into `docs/AGENT_PLAY.md`): Munch now names the
  exact numbers wrongly eaten and the fits walked past, in the terminal and in
  MCP structured content, so a kid learns which primes got away and an agent
  mining its own trajectory gets real supervision instead of a bare score.
- The Full Map (`docs/ROOMS.md`): all of mathematics as play, a coverage
  checklist across nine branches (number, algebra and symmetry, geometry and
  topology, analysis, chance, discrete structure, computation and logic,
  decision, dynamics), every entry filtered by the two laws (the concept is
  the verb; the play carries itself), each marked built or queued. A branch is
  covered when a kid can play its entry and a professor can nod at it, and
  neither one is bored.
- Postcard phases (`Room::postcard_t`), from the first full beauty-QA loop
  (render every room, look at it, judge fun/beauty/truth, fix): each room now
  tells the gallery and contact sheet its proudest moment. Found and fixed:
  Langton's Ant presented a literally black void (zero steps) and now shows
  chaos plus the highway; Julia presented near-invisible dust and now shows a
  connected set; the fern fills in at full growth; Life shows emergent
  structures instead of raw soup; Arecibo decodes instead of shearing. A new
  registry test enforces the invariant forever: no room may present a blank
  postcard.
- Fullscreen/windowed robustness verified end to end: scripted keystrokes
  toggle fullscreen on, back to windowed, then era and room switches, with the
  app alive throughout.
- Game-native controls (from first-user feedback: a Counter-Strike or Minecraft
  player should instantly get it): A/D strafe rooms, 1-9 jump to a room like
  weapon slots, W/S run time faster or slower, the mouse wheel scrubs, E
  inspects the math, Q swaps the era, R restarts the sweep, F goes fullscreen,
  B starts The Show, and Esc opens the menu (the help overlay) instead of rage
  quitting; the window's close button quits. Gamepad support is the natural
  next step of this layout.
- App UX pass (from first-user feedback): the controls are now on the glass, a
  help overlay is visible at launch (`h` brings it back) and a persistent hint
  bar sits at the bottom; `m` mutes and unmutes. The sound stopped hurting:
  the default voice dropped an octave and softened, Times Tables plays in a
  friendly register, the app renders audio quieter still, and the loop now
  follows the animation sweep instead of droning on one tone.
- Visual Eras (`crates/core` `era`): the retro-to-modern pillar, real. Four
  eras as pure RGBA transforms, Phosphor (P1 green terminal glass), 8-bit (a
  fixed 16-color palette with chunky 2x2 pixels), Vector (bright beams on pure
  black, dim light culled), and Modern (untouched). The app cycles them with
  the `e` key (GPU fractal frames included); the terminal takes `--era` on
  `render --color` and `watch`. Same math, rendered as its own history.
- The high-score table (`crates/core` `scores`, `numinous scores`, MCP
  `scores`): arcade rules, every game, every mind. Each challenge has a key
  (`munch seed:7 board:0`, `quiz seed:9 rounds:5`, `crack seed:1 digits:4`,
  ...) meaning the same thing wherever it is played, and the table keeps the
  best score per key. Munch posts per board from both faces, quiz/seti/aliens
  post per session, crack posts attempts-to-spare; beating a record prints NEW
  BEST. The MCP tool returns the table with structured content. Fourteen tools.
- Structured tool output (MCP, per the 2025-06-18 spec): munch and quiz grades
  and the journey now return structuredContent alongside the prose, machine-
  readable scores, verdicts, and progression, so agents, harnesses, and future
  leaderboards consume results without parsing sentences.
- `docs/AGENT_PLAY.md` gains a July 2026 survey of MCP-game conventions
  (PokeAgent's living leaderboard, MCPlayerOne, the turn-based reference shape,
  elicitation and sampling as the frontier, MCP-Atlas) and what each means here.
- Munch (`crates/core` `munchers`, `numinous munch`, MCP `munch`): Number
  Munchers reborn. A seeded board of numbers and a rule (eat the primes, the
  multiples of n, the perfect squares); right bites +10, wrong bites -5, a
  perfect clear +20. The same seed gives the same boards to a human in the
  terminal and an agent over MCP, so scores are directly comparable, the first
  head-to-head game across minds. `--daily` makes it a shared league; perfect
  clears count as journey wins. Thirteen MCP tools.
- `docs/PLAYFUL.md` gains the kid principle (the play carries itself even when
  the math has not connected yet; insight is loot, not a prerequisite) and the
  three shapes of play (the campaign, the watchable, the scored freestyle).
- Levels, 1 to 42 (`journey` gains `level()`, an 8-bit XP bar, and `plays`):
  XP comes from showing up, rooms entered, rounds played, sims run, curves
  made, with a little extra for being right and for secrets, so a teenager, the
  world's best mathematician, and an AI agent all reach the cap the same way:
  by playing. Level thresholds are triangular numbers; the cap is 42.
- Locks that open (`UNLOCKS`): visible, RPG-style, gating extras never basics.
  LV 3 opens `quiz --hard` (six shapes), LV 5 longer bomb codes, LV 7 a wider
  SETI sky, and LV 42 opens `numinous answer`, which finally stops being a red
  herring. `numinous journey` shows the wall: OPEN by name, LOCKED as `???`.
- Agents level too: the MCP server records the same journey (rooms seen, sims
  run, expressions made, quiz rounds answered) into the same file, and a new
  `journey` tool shows an agent its own level, bar, constellation, and locks.
  Twelve MCP tools.
- `docs/AGENT_PLAY.md`: the agent-gaming landscape (OpenClaw and the MCP
  ecosystem, gaming MCP servers, text benchmarks) and the five design rules that
  make Numinous first-class for digital minds.
- The Journey (`crates/core` `journey`, `numinous journey`): quiet roguelike
  progression. Play accumulates a private local record: rooms entered light
  stars in a shared-sky constellation, wins and secrets add weight, and the
  record confers rank in the Order (Outsider, Akousmatikos, Mathematikos,
  Kanonikos, Dekas) at triangular-number thresholds. Crossing a rank prints one
  deadpan line. Rank never gates the base experience; it opens hidden layers:
  at Mathematikos the deeper akousmata answer, and one unlisted room renders for
  those who learned its name. Below rank, the ordinary not-found; nothing is
  acknowledged. See `docs/LORE.md`.
- The five-doors design and honest audit (`docs/PLAYFUL.md`): the digital mind,
  the stoner gamer, the design expert, the PhD nerd, and the alien, and what
  each one gets today versus next. Three gaps closed with it:
  - Agents create (MCP `plot_expression`, `sing_expression`): the Studio is open
    to digital minds, plot your own function, hear it as notation. Eleven tools.
  - The daily challenge (`--daily` on `quiz`, `seti`, `crack`): one shared seeded
    puzzle per UTC day, the same for every player.
  - The humor, dissected (`crates/core` `humor`, `numinous jokes`, MCP
    `explain_joke`): each joke catalogued with its habitat and its mechanism
    stated structurally, for the alien, the agent, and anyone who enjoys frog
    dissection. The dissection warning is itself part of the joke.
- The terminal becomes a framebuffer (`crates/core` `ansi`): truecolor rendering
  packs two 24-bit pixels into every character cell via the half-block trick,
  with color-run compression, so any modern terminal shows real full-color
  images. `numinous render <room> --color` draws one; `numinous watch <room>` is
  the flagship: a room animating in full color in the terminal at 20 fps **with
  its sound playing live**, a complete audiovisual instrument with no window
  (add `--mute` for silence). Verified at 47 frames per 3 seconds.
- A text mind can hear (MCP `listen_room`): a room's sound at any phase returned
  as readable notation, each note's pitch in Hz and note name (A4, C5), timing,
  and loudness, sensory substitution for audio, in the spirit of
  `docs/DIGITAL_MINDS.md`.
- The hidden names whisper over MCP too: `describe_room` on the unlisted names
  answers in the Order's voice instead of erroring, so agents can stumble into
  the same secret humans do.
- The Show (windowed app, `s` key): lean-back mode. The HUD disappears, the phase
  sweeps slowly, and when a room finishes its sweep the app drifts into the next
  one, the whole collection playing itself for hours, with sound. Press `s` again
  to take the controls back.
- GPU real-time fractals in the app: a persistent `FractalRenderer`
  (`crates/gpu`, pipeline built once, buffers reused per frame; the WGSL shader
  gains a Julia mode) drives the Mandelbrot and Julia rooms in the window, so the
  Mandelbrot zooms deep into the seahorse valley and the Julia set morphs in real
  time at full window resolution, on whatever GPU the machine has, falling back
  to the CPU raster when there is none. Verified live on the dev laptop's AMD
  Radeon 780M (Vulkan).
- The Studio in the window (`tab` key): type math and watch it live. The curve
  redraws in color on every keystroke (the last good parse stays alive while you
  edit, errors shown gently), the parameter `a` sweeps itself with the clock so
  the shape breathes, and the expression's melody plays as you shape it.
- The Studio's expression engine (`crates/core` `studio`): a small, safe
  recursive-descent parser and evaluator for single-variable expressions in `x`
  (`+ - * / ^`, unary minus, `sin cos tan exp ln abs sqrt`, and `pi`/`e`), the
  Tier 1 safe-DSL seed of the creative graphing calculator. `numinous plot
  "sin(3*x) + x/2"` parses it and draws the curve; the engine is unit-tested for
  precedence, associativity, functions, and errors.
- Studio grows: the engine gains an animation parameter `a`, so `numinous plot
  "sin(a*x)" --animate` sweeps the knob live in the terminal; and `numinous sing
  "sin(x) + x/3" --out song.wav` turns a function into a melody (value to pitch
  over x as time). You can now see, animate, and hear an expression.
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
