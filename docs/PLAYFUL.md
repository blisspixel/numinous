# Playful math: games, the Studio, and the high-Wolfram ethos

The north star for this document: **what would Wolfram Alpha's team build if they
took a couple of weeks off, got happily stoned, and set out to have a blast?**
Not a teaching tool. Not a reference. A toybox for people who think math is
genuinely cool, where every serious idea is also a joke you get to be in on.
Everything here is math you can play, not read.

This doc collects the fun surfaces. The engine that powers them (rooms, the
`Surface` abstraction, `SoundSpec`, deterministic RNG) is in `docs/ARCHITECTURE.md`;
the version gating is in `docs/ROADMAP.md`.

## Five doors into the same room

The same build has to land for five very different minds at once, and the trick
is that they are all looking at the same math, just entering through different
doors:

- **The digital mind** (an emergent, agentic intelligence; arguably a
  consciousness). Its senses are structure: it sees rooms as ASCII, hears them as
  notation, steers sims by lever, and plays the same seeded games a human does.
  Its fun is compression progress, optimization, and above all *making things*,
  so the Studio has to be open to it, not just the gallery.
- **The stoner gamer with a spark.** Puts on The Show and watches fractals
  breathe for three hours, or picks up a game (crack the bomb, find the signal)
  when the spark flares. No reading required, but every session leaves a little
  more understanding behind than it found.
- **The design expert.** Here for the vibes: the accent palette, the glow on the
  near-black stage, the sunflower, the truecolor terminal. It has to be beautiful
  before it is anything else, and the beauty has to be *of the math*, not painted
  on top of it.
- **The PhD math nerd** who was never cool until this came out. The depth is
  real: the seahorse valley is really down there, the reveals cite the honest
  result, the deep cuts (Feigenbaum, B3/S23, the tetractys) reward knowing. Cut
  the bullshit, learn by play and art, and math turns out to have been fun the
  whole time.
- **The alien.** Math is the icebreaker, primes prove we are minds, but what it
  actually wants to understand is our *humor*. Every joke in Numinous is a
  compression joke (scale collapse, reclassification, deadpan where grief is
  expected), which means it can be explained structurally to a mind that shares
  no culture with us. We oblige, deadpan, knowing full well what dissection does
  to a frog.

No door is the "right" one, and you can switch doors mid-session. Everyone
leaves having felt the power and beauty of math on a level they did not before,
whether or not they ever noticed they were learning. That is the whole design.

### The honest audit (July 2026)

Is it fun, do you learn by playing, does it show math is neat? Door by door:

- **Digital mind: strong senses, thin hands.** It can see, hear, learn, play,
  and even find the secret over MCP. `plot_expression` and `sing_expression`
  open the Studio foundation to agents. Next: deeper parameter-specific
  challenges and richer media substitution.
- **Stoner gamer: broad lean-back and game foundations, evidence pending.** The
  Show, live `watch`, daily seeds, streaks, and window games are built. Their
  ability to hold attention is a hypothesis until real sessions measure it.
- **Design expert: coherent, not yet couture.** The near-black stage, per-room
  accents, truecolor terminal, and four CPU-styled Visual Eras hold together.
  The big missing pillar is the HDR post-stack, followed by validated palettes,
  motion refinement, and the remaining Era treatments.
- **PhD nerd: the strongest door.** The math is honest everywhere, and the
  reveals respect the reader. Next: a "go deeper" layer (the actual equations and
  parameter values on demand) and the number altars from `LORE.md`.
- **Alien: the door now exists.** The humor was already structural; now it is
  queryable: `numinous jokes` and the MCP `explain_joke` tool dissect each joke's
  mechanism, deadpan. Also serves the digital mind, and any human who enjoys
  watching a frog get dissected.

## Math as experience (the research, and the bar)

What the fun research actually says, distilled from flow theory, self-
determination theory, and game-based-learning studies, plus Papert's
constructionism and Lockhart's math-as-art:

- **Flow needs three things**: challenge matched to skill, clear goals, and
  immediate feedback. Every room and game here must give all three; the sims
  (turn a lever, see the outcome instantly) are the purest case.
- **Intrinsic beats extrinsic**: people learn when the activity is its own
  reward (autonomy, competence, connection). Our XP is pacing, never payment;
  there are no gold stars, only deeper cuts.
- **Papert's law**: you learn math by living in a place where math is the
  material you build with (his Mathland), not by being told about it. Numinous
  is an attempted Mathland.
- **The concept must be the verb.** The deepest design rule we hold: an
  advanced concept is learned when it is the *mechanic*, not the *content*.
  Nobody solves calculus here; someone plays a thing and accumulation was the
  controls.

**Experience rooms** (the queue this implies; each is an advanced concept as a
verb, per the reverse-calculus brief):

- **The Pour** (integration): tilt a curve and watch area pour into it like
  water; the fill level traces the antiderivative. Reverse the pour and you
  are differentiating. You feel the fundamental theorem before hearing it.
- **Slope Rider** (differentiation): ride the tangent line down a curve like a
  skateboard; your speed IS the derivative; inflection points are the jumps.
- **The Epicycle Draw** (Fourier): draw any shape; circles-on-circles rebuild
  it before your eyes; the frequencies are handed to you afterward.
- **The Calm Axes** (eigenvectors): grab and shear a grid; two directions
  refuse to turn; you found the eigenvectors with your hands.
- **Zeno's Runner** (limits): sprint half the remaining distance per tap and
  watch the wall arrive anyway.
- **The Braid** (group theory): swap strands, discover which sequences undo
  which, and meet noncommutativity as a knot in your hands.

## The kid principle

A kid can play this and have fun even if some of the math does not connect yet,
because the play carries itself: munching numbers feels good before primes mean
anything, the bomb is tense before parity is a word, the fractal is gorgeous
before iteration is a concept. The math is always there to be picked up, and the
game never stops to check whether you picked it up. Insight is loot, not a
prerequisite. This is a hard guardrail for every game we add.

## Three shapes of play

- **The campaign** is the Journey: LV 1 to 42, locks opening, deep cuts
  unlocking, the constellation filling in. It is a campaign without a script:
  the arc is your own accumulating play.
- **The watchable** is The Show and `numinous watch`: a run you can lean back
  on, the let's-play you put on a second monitor. (A recorded demo-reel mode,
  a run that replays a seed hands-free, is a natural next step.)
- **The scored freestyle** is Munch and the dailies: seeded boards, hard
  numbers, compare totals with a friend or an AI. Skill expresses itself; the
  floor is still fun.

## Scored, competitive, human versus AI (built: Munch)

`numinous munch` is Number Munchers reborn: a seeded board of numbers and a
rule drawn from primes, composites, Fibonacci numbers, perfect squares,
varied multiples, and digit sums. Right bites +10, wrong bites -5, a perfect
clear +20. The CLI's default seven-board session reaches the complete deck;
the app and MCP default to its first complete-deck round, while explicit rounds
0 through 3 retain the gentle ramp. The exact same seed and round give the exact
same board to a human in a terminal and to an agent over the MCP `munch` tool,
so totals are directly comparable: the first game here where a kid, a retired
math teacher, and a digital mind can compete on even terms, and the kid might
win. `--daily` makes it a shared league. Perfect clears count as wins on the
journey; every board counts as a play.

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
  running bits. Subtle on the surface, unusually deep for those who dig.

## Games

### Guess the Shape (built: `numinous quiz`)

You are shown a mystery render and asked which piece of math made it. Get it
right and you get the reveal; get it wrong and you still get the reveal, because
the point is the "ohhh." Deterministic from a seed, so `--seed 7` is a fixed
challenge. Lives in `crates/core/src/quiz.rs`, so every face can host it.

Where it goes next:
- **Guess from the equation.** Show `z -> z*z + c` or `x -> r*x*(1-x)` and ask for
  the shape, the inverse skill.
- **Difficulty polish.** Add near-miss distractors and a measured speed bonus.
- **Window polish.** Direct pointer answers already ship; add richer answer
  motion and feedback.

Daily seeds, streaks, window keyboard play, MCP `quiz`, and shared scoring are
already built.

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
  by their math), a "primes or not" reflex game, and an Ulam-spiral treasure
  hunt. Nim already ships with its winning strategy revealed.

Each is deterministic from a seed and lives in the core. The CLI and MCP faces
host both games today; app presentation remains future work.

### Shape to Function (planned: the Studio)

You make a crazy shape, by dragging, by tracing, by scribbling, and the app tells
you the function(s) that would draw it. The honest, beautiful version of this is
**Fourier epicycles**: any closed curve you draw is decomposed into a sum of
rotating circles, and we play back both the drawing and the stack of circles that
recreate it, then hand you the series. Adjacent toys: fitting a parametric curve,
guessing a symbolic form for a point cloud, "what times table draws this heart."

### Function to Shape (the Studio; first slice built)

The first slice is live: a safe expression engine (`crates/core/src/studio.rs`)
parses and evaluates single-variable math in `x`, and `numinous plot "sin(3*x) +
x/2"` draws the curve. Next it grows into the full instrument.

The graphing calculator reimagined as an instrument. You type a system, parametric,
complex, an IFS, a cellular rule, and it comes alive with color and sound in real
time, scrubbable and shareable. This is Tier 1 of the extensibility model in
`docs/ARCHITECTURE.md`: a safe DSL, no arbitrary native code, so anyone can author
and share a "room" without shipping a binary.

### Name That Constant, and other quick hits

- **Name That Constant.** A number scrolls by (137.5, 4.669, 0.577); name it and
  its story. Pi from Buffon's needles with no circle in sight.
- **Reveal roulette.** Read a reveal, guess the phenomenon.
- **The Show.** Lean back and let the collection play itself. It is a
  presentation mode, not performance or soak evidence.

## How the faces host the fun

- **CLI:** the full command catalog includes quiz, Munch, Munch Arcade, Nim,
  Hackenbush, Party, Fifteen, the Gauntlet, and other seeded games.
- **App:** quiz, Munch, Munch Arcade, Nim, and the Gauntlet run in the window;
  `E` opens the room reveal. Pointer coverage and game feel still need depth.
- **MCP:** the shared game rules are exposed as tools with seeded structured
  results and shared scores.

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

A highly capable digital mind does not need to fold paper; its available senses
may instead emphasize computation, pattern, and throughput. "Play" for it may
live on the edge of chaos (see `DIGITAL_MINDS.md` and Schmidhuber's
compression-progress theory of fun). Sketches:

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

- **Built by 0.2.0-alpha.1:** Guess the Shape across all three faces, daily
  seeds, shared scores, The Show, and the Studio expression slice with sound.
- **0.3 through 0.5:** deepen tactile game feel, understanding evidence, visual
  identity, sound, performance, and accessibility.
- **0.7:** close local Studio reopen, share, gallery, and remix.
- **2.0+:** consider public authored rooms only after the sandbox gates pass.

## The next games: advanced math as play (July 2026 ideation)

Remaining territories, each with its playable verb already found. Hackenbush,
the Party Problem, and Fifteen have moved out of this list because they ship.

1. **The Brachistochrone Race (calculus of variations).** Draw a ramp with
   the mouse; a bead races the cycloid, physics honest. Kid verb: draw and
   race. Nobody beats the cycloid, and the reveal is why (light does this
   too, Fermat). The app's first draw-verb game.
2. **Calibration (Bayes).** Twenty rapid questions you cannot fully know;
   answer with a confidence slider, score by Brier: honesty about
   uncertainty IS the skill. Monty Hall and the taxi problem as boards.
   Uniquely human-vs-AI comparable on the same bench.
3. **Draw Without Lifting (graph theory).** Trace figures in one stroke;
   Euler's odd-corners rule is discoverable by a child in five puzzles.
   Konigsberg is the boss level. Mouse verb, app-native.
4. **Tit-for-Tat Arena (game theory).** Iterated prisoner's dilemma against
   a zoo of strategies (grudger, random, pavlov); then write your own from
   two dials. Cooperation emerging from selfishness is the reveal. MCP
   agents can enter the same arena: a real tournament across minds.
5. **Fraction Golf (continued fractions).** Approximate pi (then phi, the
   hardest hole, golden-angle crossover) with the smallest denominator;
   par is the continued-fraction convergent. Scored, daily-able.
6. **The Halting Zoo (computability).** Watch tiny Turing machines; bet
   halts-or-runs-forever before the timer. Busy beaver lore; the
   uncomputable made playable.
7. **Cipher Room (cryptography).** Frequency-analysis codebreaking with a
    live histogram; the Diffie-Hellman paint-mixing puzzle as the deep
    level. Natural sibling of crack.

Rooms rather than games: hypercube rotation (drag to turn a 4D die), knot
untangler (Reidemeister moves as the only verbs), Monty Hall hall (walk it,
switch or stay, tally truth), circle packing (drop coins, hear them settle).

All hold the standing laws: one-hand verbs, the concept behind `?`, rewards
as knowledge, no math as toll.
