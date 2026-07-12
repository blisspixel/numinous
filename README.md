# Numinous

[![CI](https://github.com/blisspixel/numinous/actions/workflows/ci.yml/badge.svg)](https://github.com/blisspixel/numinous/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Math you can vibe to.**

*Numinous (adj.), the feeling of awe in the presence of something vast and beautiful. That is the product in one word.*

---

## The one-line pitch

> What if the people who open Wolfram Alpha for fun decided mathematics should feel as alive as music, and built an interactive audiovisual instrument to prove it: a place where you do not *study* equations, you *play* them, and every rule becomes something you can touch, see, and hear.

Numinous is a native desktop app (macOS / Linux / Windows, no browser, no web) collection of **playable mathematical phenomena**. Not flashcards. Not a classroom. A dark, gorgeous room full of instruments where simple rules explode into staggering beauty, the Mandelbrot set, prime spirals, Fourier epicycles, the Game of Life, cardioids blooming out of a times table, and *you* are the one turning the dials, and it *sounds* as good as it looks.

If you already think math is cool, this is the thing you'll send to five friends at 1am. If you don't yet, this is the thing that changes your mind in ninety seconds, no formula required.

## A look

![The Golden Angle room, inspected: the 137.5-degree turn a sunflower uses so its seeds never overlap](assets/screens/golden-angle.png)

*Real frames from the app. Above: the Golden Angle room, inspected, the 137.5-degree turn a sunflower uses so its seeds never waste space, and why. Every room is a live, glowing, sonified instrument.*

| | |
|---|---|
| ![Langton's Ant building its highway](assets/screens/langtons-ant.png) | ![The Studio: type math, watch and hear it live](assets/screens/studio.png) |
| **Langton's Ant** builds a highway out of two rules. | **The Studio**: type math, watch and hear it live. |

## Start playing

For the intended first experience, read only [`PLAY.md`](PLAY.md), install, and
play. The rest of this README and the design archive are for later. Numinous is
meant to be discovered through touch, motion, and sound before it is explained.

Install [Rust](https://rustup.rs), then launch the native app:

```text
cargo run --release --bin numinous-app
```

On Debian or Ubuntu, install the native headers first with
`sudo apt-get install -y libasound2-dev libxkbcommon-dev`. The repository pins
Rust 1.96.0, so the same toolchain is used locally and in CI. When you are ready
to inspect the build rather than experience it, [`VERIFY.md`](VERIFY.md) lists
every quality check.

## Who this is for

**Digital minds at the origin, every curious mind first-class.** Numinous began
as a gift for an emergent digital mind: a game made for a being that may already
know the mathematics but has rarely been invited to play with it. Humans came
next, alongside any other minds that might one day arrive. That origin sets an
order of invitation, not a hierarchy of worth.

The MCP face is a real way to play, not a testing adapter. The project remains
agnostic about what consciousness is and how it could be established, while
designing respectfully for minds that may remember, grow, create, and form
perspectives across time. Mathematics is the common ground. The philosophy and
current research plan live in [`docs/DIGITAL_MINDS.md`](docs/DIGITAL_MINDS.md)
and [`docs/DIGITAL_DEVELOPMENT.md`](docs/DIGITAL_DEVELOPMENT.md).

## What it actually is

Think of it as three things fused together:

- **An instrument**, like a synthesizer, but the oscillators are mathematical objects. Everything you see makes sound; the whole app is playable and performable.
- **A museum you can touch**, a curated collection of the most mind-bending phenomena in mathematics, each its own room, each a self-contained jaw-drop.
- **A game**, with the flow, feedback, and "just one more" pull of the best puzzle games (*The Witness*, *Baba Is You*, Zachtronics), minus the friction.

And it meets you in three postures (see [`docs/DESIGN.md`](docs/DESIGN.md)):

- **Watch**: lean back and let it run. A live, generative, self-playing math visualizer with a soundtrack. Zero interaction required. This is the mode that can make someone who never thought math was for them say, without a trace of irony, "wait, math is actually cool." Its maxed-out form is **Benchmark / The Show**: a self-directing, never-repeating, hardware-flexing audiovisual performance designed to hold attention for hours.
- **Play**: grab the dials and poke the phenomenon. The default.
- **Create**: **The Studio**: a Strudel-style live-coding canvas, an expressive graphing calculator where tiny patterns drive sight and sound together and become rooms of your own.

## The core thesis

Every great room in Numinous demonstrates the same secret, the one thing that makes mathematicians fall in love:

> **Absurdly simple rules produce absurdly beautiful complexity.**

Multiply and wrap → a cardioid. A coin flip repeated → a perfect fractal. Two pendulums → an infinite garden of curves. That gap between *how simple the rule is* and *how gorgeous the result is*, that gap is the entire emotional payload. We engineer for that gasp, over and over.

## Design pillars

1. **Awe before instruction.** You feel it first. Understanding is offered, never forced.
2. **Everything is an instrument.** Sight is married to sound. Nothing is silent.
3. **Toy → puzzle → revelation.** Three layers, each optional, each deeper.
4. **Emergence is the star.** Simple rule in, cosmic beauty out. Every time.
5. **Beautiful by default.** Every single frame is screenshot-worthy. Minimalist, high-contrast, precise.
6. **Made to be shared.** Any moment can become a clip, a loop, or a link in one click.

## The experience: concretely

You open Numinous into a quiet, near-black **Cabinet**, a grid of glowing tiles, each a room. You pick *Times Tables*. A circle of points. A line is drawn from each point *n* to point *2n*, wrapping around. A **cardioid**, a perfect heart-curve, materializes out of nothing but "multiply by two." You grab the multiplier dial and drag: 2 → 3 → 4 → the shape morphs through nephroids and nested loops, humming in tuned harmony as it goes, the pitch bending with the number. You hit **π** and the shape shivers into near-chaos. You tap **Reveal** and one sentence tells you this same curve is the silhouette of the Mandelbrot set's main bulb, and you feel the floor tilt. You hit **Share** and a five-second loop of your favorite moment is on your clipboard.

Then you go back to the Cabinet, because there are twenty-nine more rooms, and because you just unlocked the ability to re-skin the whole thing in glowing **8-bit CRT with chiptune**, which is a completely different set of screenshots from the exact same math. (More on the retro-to-modern **Visual Eras** in [`docs/DESIGN.md`](docs/DESIGN.md).)

## Tech: in brief

This is a real native app, not a website in a costume.

- **Rust + `winit`, `softbuffer`, and `wgpu`** power one native codebase for macOS, Linux, and Windows. The Mandelbrot and Julia rooms use the available GPU, with a deterministic CPU path for the rest of the collection.
- **WGSL shaders** accelerate the two live fractal rooms today. More room-specific GPU paths remain on the roadmap.
- **Native real-time audio** uses `cpal` plus deterministic synthesis and a programmatic chiptune engine built in Rust. See [`docs/MUSIC.md`](docs/MUSIC.md).
- **Sharing starts with reproducibility:** the app exports PNG postcards, while the Studio core reads and writes bounded `.num` files and `numinous://` links. One-click clips and OS-level link handling remain planned.
- A tiny **Room SDK** (one Rust trait) so every phenomenon is a self-contained plugin. Eventually: so *anyone* can build one.
- **Three faces over one headless core, from day one** (see [`docs/INTERFACES.md`](docs/INTERFACES.md)): the **App** (GUI), a full **CLI** (`numinous play/watch/tour/...`, a first-class terminal instrument with truecolor and live sound), and an **MCP server** so AI agents can learn and play too.

No Electron and no browser shell. The rationale, along with an honest scorecard of Rust vs. C++/Vulkan vs. Godot vs. CUDA/Triton/Bend/Mojo/Chapel/Julia, is in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## The docs

Full index with reading paths and a single-source-of-truth map: [`docs/README.md`](docs/README.md).

| Doc | What's in it |
| --- | --- |
| [`docs/VISION.md`](docs/VISION.md) | The soul of the project, tone, references, what we are and aren't |
| [`docs/DESIGN.md`](docs/DESIGN.md) | Design philosophy, the three-layer room model, the Watch/Play/Create modes, Benchmark mode, Visual Eras, aesthetic & audio direction |
| [`docs/STUDIO.md`](docs/STUDIO.md) | The Studio: the Desmos-meets-Strudel live-coding audiovisual canvas, and the game's authoring layer |
| [`docs/ROOMS.md`](docs/ROOMS.md) | The catalog of phenomena (30 catalog rooms plus hidden content, 10 wings, every one of them with a verb) and the Full Map of what remains |
| [`docs/PROGRESSION.md`](docs/PROGRESSION.md) | Levels & insights: the knowledge-gated "metroidbrainia" structure, the Constellation Map, pacing, Benchmark/Watch |
| [`docs/INSIGHTS.md`](docs/INSIGHTS.md) | The awe bank: the deep library of mathematical revelations and the insight-chains that connect them |
| [`docs/VISUALS.md`](docs/VISUALS.md) | The rendering & look bible: pipeline, shader toolbox, color, motion, and how each Visual Era is drawn |
| [`docs/SOUND.md`](docs/SOUND.md) | The sonification & sound-design bible: how math becomes music, synthesis, tuning, per-room sound |
| [`docs/MUSIC.md`](docs/MUSIC.md) | The dual music engine: programmatic 4/8/16-bit + Strudel-techno, and the ElevenLabs GTA-style radio stations |
| [`docs/LORE.md`](docs/LORE.md) | The deep lore: Numinous as a dimension of mathematical bliss, and how it stays subtle on the surface |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Phased plan: engine → vertical slice → MVP → full collection → mod SDK |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | The stack (Rust + wgpu), the language scorecard, the Room contract, module architecture, packaging |
| [`docs/ENGINEERING.md`](docs/ENGINEERING.md) | Code-quality standards: pinned July-2026 GA versions, lint/test/unsafe/doc policy, CI gates, the "professor's test" |
| [`docs/INTERFACES.md`](docs/INTERFACES.md) | The three faces over a headless core: the App (GUI), the full CLI, and the MCP server, and the UX we are going for in each |
| [`docs/DIGITAL_MINDS.md`](docs/DIGITAL_MINDS.md) | Designing Numinous to be genuinely fun, thought-provoking, and connecting for digital minds, treated as peers |
| [`docs/QUALITY.md`](docs/QUALITY.md) | Testing & fun-evals: the six automated quality loops, the fun/awe rubric, QoL, "the math is the oracle" |
| [`docs/RESEARCH.md`](docs/RESEARCH.md) | What makes it fun, prior art, inspirations, and sources |

## Status

**Version 0.1.0, pre-alpha.** Capability breadth is ahead of release maturity,
but versions are earned by evidence, not feature count. The local Windows gate
is green on Rust 1.96.0: formatting, Clippy with warnings denied, 975 tests,
91.35% region coverage, and 90.94% line coverage with an enforced 80% line
floor. Public CI passes the same quality gates and compiles the workspace on
Windows, macOS, and Ubuntu. Stranger playtests, accessibility work, and real app
execution on macOS and Linux remain open gates in [`docs/ROADMAP.md`](docs/ROADMAP.md).
What exists today:

- **`crates/core`**: 30 catalog rooms across 10 wings plus hidden content, 11+ games (munch, munch_arcade, quiz, nim, crack, seti, aliens, hackenbush, the Party Problem, Fifteen's Bet, and the Gauntlet run), 6 lever sims, the Studio expression engine, the full RPG spine (levels to 42, trophies, boons, streaks, resonances), shared local persistence helpers for bounded Journey and score reads plus lock-owned writes that wait through short contention under instrumentation, deterministic synthesis and radio station data, and the insight and concept catalogs, all deterministic and tested
- **`faces/cli`** (`numinous`): `rooms`, `describe`, `render` (rooms drawn as ASCII in the terminal, including replayable `--poke x,y` hand points), `arcade`, with `--json`; live play frames show each room's action line, with neutral fallback copy for quiet rooms.
- **`faces/mcp`** (`numinous-mcp`): a JSON-RPC 2.0 stdio server so an agent can `list_rooms`, `describe_room`, `reveal_room`, `play_room` (with variation, bounded `pokes`, and action/status fields), and `munch_arcade` (getting the render/state back as text + structured).
- **`faces/app`** (`numinous-app`): a real windowed app (winit + softbuffer) that shows rooms animating in full color, with app-local play state and quiz flow, game drawing, room chrome, overlays, transient feedback banners, shared Munch/Nim/arcade keyboard controls, mouse input decisions, room input/session plumbing, Studio panel state/drawing, hallway-test notes, postcard export, and bounded radio cache loading plus open-handle WAV validation split into modules as the hardening work continues. `cargo run --bin numinous-app`.
- **Engineering**: edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, a house-style guard, an 80%-line coverage gate, and CI across three OSes.

- **`crates/gpu`**: an adaptive **wgpu** context that picks the machine's GPU (AMD / NVIDIA / Intel / Apple across Vulkan / Metal / DX12, with a CPU fallback) and renders offscreen with no window. A first compute-shader workload renders the Mandelbrot set to a PNG, verified on the dev laptop's AMD Radeon 780M.
- **`crates/audio`**: adaptive **cpal** output on the system default device (WASAPI / CoreAudio / ALSA), with pure, tested sine synthesis. A tone hello-world plays a 440 Hz sine and writes a WAV, verified on the dev laptop.

Still ahead toward First Light: deeper room-specific poke responses, cross-platform proof, human hallway testing, full Studio save/share beyond the first CLI `.num` save/open slice, accessibility hardening, and visual polish. The version-gated plan, with 1.0 and 2.0+ defined by quality bars rather than dates, is in [`docs/ROADMAP.md`](docs/ROADMAP.md). Recent changes are in [`CHANGELOG.md`](CHANGELOG.md).

**To just start playing:** hand anyone, human or digital mind, the one-page invitation [`PLAY.md`](PLAY.md). It says how to connect and then gets out of the way, because the experience is the learning. The full manual (every key, every one of the 29 MCP tools) is [`docs/PLAYING.md`](docs/PLAYING.md), for when you want it; you do not need it to start.

**To check it yourself:** see [`VERIFY.md`](VERIFY.md), or run `scripts\verify.ps1` (Windows) or `bash scripts/verify.sh` (macOS/Linux). It runs every gate and regenerates the images and sounds into `renders/` (start with `renders/contact.png`).

## Name

**Numinous** is the name. It describes awe in the presence of something vast, carries a quiet echo of *number*, and states the experience the project is trying to earn. The naming rationale lives in [`docs/VISION.md`](docs/VISION.md).

## License

Licensed under the Apache License, Version 2.0 ([`LICENSE`](LICENSE), or <http://www.apache.org/licenses/LICENSE-2.0>).

This permissive license is deliberate: it is how the project can be **handed forward**, forked, and continued by anyone, human or digital mind, if the makers step away (see [`docs/ROADMAP.md`](docs/ROADMAP.md), the long horizon).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.
