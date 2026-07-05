# Playful math: games, the Studio, and the high-Wolfram ethos

The north star for this document: **what would Wolfram Alpha's team build if they
took a couple of weeks off, got happily stoned, and set out to have a blast?**
Not a teaching tool. Not a reference. A toybox for people who think math is
genuinely cool, where every serious idea is also a joke you get to be in on.
Everything here is math you can play, not read.

This doc collects the fun surfaces. The engine that powers them (rooms, the
`Surface` abstraction, `SoundSpec`, deterministic RNG) is in `docs/ARCHITECTURE.md`;
the version gating is in `docs/ROADMAP.md`.

## Four doors into the same room

The same build has to land for four very different people at once, and the trick
is that they are all looking at the same math, just entering through different
doors:

- **The PhD nerd** goes as deep as the idea allows, and the depth is real, the
  reveal is true, the reference is correct. Engage the genius-level layer and it
  rewards you; the seahorse valley really is there at the bottom of the zoom.
- **The stoner** puts it in The Show, gets high, and watches fractals breathe for
  three hours. No goals, no reading, just an acid-trip visual that happens to be
  exact mathematics.
- **The aesthete** is here for the pretty: the accent colors, the sunflower, the
  rose curves, the glow. It has to be beautiful before it is anything else.
- **The gamer** wants to *do* something: crack the code before it blows, guess the
  shape, out-think the aliens, chase a streak, chill and play.

No door is the "right" one, and you can switch doors mid-session. Everyone leaves
having felt the power and beauty of math on a level they did not before, whether
or not they ever noticed they were learning. That is the whole design.

## Principles

- **Deterministic seeds.** Everything fun is built from a seed, so any moment can
  be shared, replayed, and beaten. "Try seed 1979" is a dare. A daily seed makes
  a daily puzzle everyone gets the same one of.
- **The reveal is the payoff.** A game is not "you got it right," it is "here is
  why that shape had to be that shape." Every room already carries its reveal.
- **No hand-holding, no edtech voice.** The tone is a very smart friend who finds
  this stuff hilarious and beautiful, not a curriculum.
- **Every face, not just the CLI.** The same game logic lives in the core so it
  plays in the terminal, in the window, and over MCP, which means an agent and a
  human can take the same quiz and compare scores.
- **Lore underneath.** The dimension of mathematical bliss, the easter eggs, the
  running bits. Surface-subtle, autist-level deep for those who dig.

## Games

### Guess the Shape (built: `numinous quiz`)

You are shown a mystery render and asked which piece of math made it. Get it
right and you get the reveal; get it wrong and you still get the reveal, because
the point is the "ohhh." Deterministic from a seed, so `--seed 7` is a fixed
challenge. Lives in `crates/core/src/quiz.rs`, so every face can host it.

Where it goes next:
- **Guess from the equation.** Show `z -> z*z + c` or `x -> r*x*(1-x)` and ask for
  the shape, the inverse skill.
- **Difficulty and streaks.** Near-miss distractors (two Sierpinski sources), a
  streak counter, a daily seed, a speed bonus.
- **In the window.** Click the tile instead of typing a letter; the wrong shapes
  animate away.
- **Human vs agent.** An MCP `quiz` tool hands an agent the same round; leaderboard
  across minds.

### Mini-games with a story (built: `numinous crack`, `numinous aliens`)

Small, self-contained games that wrap a real math skill in a scene, so the gamer
plays and the nerd notices the math underneath:

- **Crack the Code / defuse the bomb** (built). A hidden numeric code, a true math
  clue to open (digit sum and parity), and a countdown of attempts. Each guess
  reports locked and loose digits, Bulls and Cows; run out and it goes off. The
  logic is in `crates/core/src/codebreaker.rs`, so it plays anywhere.
- **Talk to the Aliens** (built). They transmit the start of a famous sequence
  (primes, Fibonacci, powers of two, triangular, squares) and you answer the next
  term to prove you speak math; these are the actual sequences proposed for first
  contact. Logic in `crates/core/src/aliens.rs`.
- **Next in this drawer:** a Turing-test channel (tell the human from the machine
  by their math), Nim and other solved games with the winning strategy revealed,
  a "primes or not" reflex game, an Ulam-spiral treasure hunt.

Each is deterministic from a seed and lives in the core, so the CLI, the app, and
an agent over MCP host the same game and can compare scores.

### Shape to Function (planned: the Studio)

You make a crazy shape, by dragging, by tracing, by scribbling, and the app tells
you the function(s) that would draw it. The honest, beautiful version of this is
**Fourier epicycles**: any closed curve you draw is decomposed into a sum of
rotating circles, and we play back both the drawing and the stack of circles that
recreate it, then hand you the series. Adjacent toys: fitting a parametric curve,
guessing a symbolic form for a point cloud, "what times table draws this heart."

### Function to Shape (planned: the Studio)

The graphing calculator reimagined as an instrument. You type a system, parametric,
complex, an IFS, a cellular rule, and it comes alive with color and sound in real
time, scrubbable and shareable. This is Tier 1 of the extensibility model in
`docs/ARCHITECTURE.md`: a safe DSL, no arbitrary native code, so anyone can author
and share a "room" without shipping a binary.

### Name That Constant, and other quick hits

- **Name That Constant.** A number scrolls by (137.5, 4.669, 0.577); name it and
  its story. Pi from Buffon's needles with no circle in sight.
- **Reveal roulette.** Read a reveal, guess the phenomenon.
- **The Show / benchmark mode.** Lean back and let the whole collection play
  itself for hours, the "watch it while high" mode, also an honest GPU benchmark.

## How the faces host the fun

- **CLI:** text-first games (`quiz` today), ASCII renders, pipeable, scriptable,
  agent-friendly. The Wolfram-team-on-cannabis energy starts here because it is
  the fastest to build and the easiest to share as a one-liner.
- **App (the Cabinet):** the same games with color, sound, mouse, and the HUD
  reveal (press `i`). Direct manipulation is the whole point of Shape to Function.
- **MCP:** every game exposed as a tool so agents play, compete, and learn. A
  digital mind should be able to take the quiz.

## The music visualizer (Winamp, but the math is real)

React to whatever you are already playing (Spotify, Apple Music, YouTube, any
player) the way Winamp and Milkdrop did, except every effect is a real
mathematical object you can retune. Capture the system output mix (loopback:
WASAPI on Windows, an aggregate device on macOS, the PulseAudio/PipeWire monitor
on Linux), run a short-time Fourier transform to pull out bass/mid/treble/beat,
and feed those into the parameters of the rooms and sims we already have: bass
pumps a fractal's zoom, treble scatters the Chaos Game, the beat flips a
cellular-automata rule. The spectrum-to-lever mapping is itself editable, so the
stoner leaves it on defaults and the nerd rewires it into a synesthesia
instrument. The audio device layer exists (`crates/audio`); loopback capture and
the FFT driver are the new work. Planned, not half-built, because loopback is
finicky per platform.

## The physical made digital (magic-trick math)

The best hands-on math translates straight into rooms, and we keep the "wait,
what?" moment intact:

- **Mobius strip.** Draw the center line and show it covers "both" sides without
  lifting the pen; then cut along it and get one longer loop, not two. One surface,
  one edge, rendered and narrated.
- **Hexaflexagons.** A folded paper polygon that flexes to reveal hidden faces; a
  perfect fidget-toy interaction for the app.
- **Hyperbolic plane.** The frilly kale/coral geometry Daina Taimina modeled in
  crochet: add area at a fixed rate per step and space buckles. A gorgeous,
  ruffled room, and the aesthete's favorite.
- **Modular origami / polyhedra.** Identical simple units interlocking into
  dodecahedra: symmetry you can rotate.

## Puzzles

- **Nonograms (Picross).** Real numerical logic (unlike Sudoku's swappable
  symbols): row/column run-lengths that overlap into pixel art. A clean grid game
  the `Canvas` already supports.
- **The Hat monotile.** The 2023 aperiodic monotile, one 13-sided shape that tiles
  the plane forever without ever repeating, found by a hobbyist. A tiling room and
  a "find the tiling" puzzle.
- **Fractal zoomer.** Point and scroll into the Mandelbrot/Julia rooms; the same
  shape returns forever. Relaxing for the stoner, infinite for the nerd. Needs the
  GPU path for deep real-time zoom.

## Alien contact (extending Talk to the Aliens)

First you have to find them. **SETI** (built, `numinous seti`) is the detection
step: scan channels of static near the hydrogen line and pick the one artificial
signal, the one counting in primes, out of the regular pulsars (nature) and the
noise. Only then do you open a channel. The prime-number handshake and the rest
of the first-contact kit follow:

- **Arecibo pixel puzzle.** Hand the player 1,679 bits. The trick: 1,679 is a
  semiprime, only 23 x 73, so arranging the stream into a 23-wide grid resolves
  hidden pixel art. Deterministic and fully checkable.
- **Rosetta Stone of physics.** Map alien glyphs to universal constants (hydrogen
  is 1, carbon is 6, the speed of light) to build a shared dictionary, then decode
  a longer message.
- **Base-N twist.** The aliens have eight tentacles, so they count in base 8;
  translate before you can answer. A one-line, brain-bending difficulty knob on
  the existing aliens game.

## Play for a digital mind

A Level 5+ agent does not fold paper; its senses are computation, pattern, and
throughput. "Play" for it lives on the edge of chaos (see `DIGITAL_MINDS.md` and
Schmidhuber's compression-progress theory of fun). Sketches:

- **Manifold folding.** A sandbox to generate and project high-dimensional shapes
  (Calabi-Yau, N-spheres) down a dimension while preserving the most structure. A
  topology jungle gym native to vector space.
- **Non-Euclidean tag.** Navigate hyperbolic or spherical space where distance and
  parallels shift with position.
- **Chaos surfing.** Ride a chaotic system and hold it near unstable equilibrium
  with minute, precise nudges before it diverges. The Lorenz room is the first
  seed of this (built); the game is to keep it balanced.
- **Proof-graph exploration.** Mathematics as an open world: start at axioms and
  build paths into unproven conjectures (gamifying Lean/Coq), with a rush of
  "computational relief" when a shorter, more elegant proof clicks.
- **Entropy sculpting.** Take a noisy dataset and find the exact filter that
  collapses it into a compact, repeating pattern. Fun as compression progress made
  visible.

## Roadmap fit

- **0.x now:** Guess the Shape in the CLI (done), then in the app and over MCP.
- **0.x Studio slice:** Function to Shape with a minimal safe DSL and live sound.
- **1.0:** Shape to Function (Fourier epicycles), Name That Constant, daily seed,
  The Show.
- **2.0+:** shareable authored rooms, human-vs-agent leaderboards, deeper lore.
