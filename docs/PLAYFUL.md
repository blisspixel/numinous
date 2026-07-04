# Playful math: games, the Studio, and the high-Wolfram ethos

The north star for this document: **what would Wolfram Alpha's team build if they
took a couple of weeks off, got happily stoned, and set out to have a blast?**
Not a teaching tool. Not a reference. A toybox for people who think math is
genuinely cool, where every serious idea is also a joke you get to be in on.
Everything here is math you can play, not read.

This doc collects the fun surfaces. The engine that powers them (rooms, the
`Surface` abstraction, `SoundSpec`, deterministic RNG) is in `docs/ARCHITECTURE.md`;
the version gating is in `docs/ROADMAP.md`.

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

## Roadmap fit

- **0.x now:** Guess the Shape in the CLI (done), then in the app and over MCP.
- **0.x Studio slice:** Function to Shape with a minimal safe DSL and live sound.
- **1.0:** Shape to Function (Fourier epicycles), Name That Constant, daily seed,
  The Show.
- **2.0+:** shareable authored rooms, human-vs-agent leaderboards, deeper lore.
