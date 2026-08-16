# Numinous

[![CI](https://github.com/blisspixel/numinous/actions/workflows/ci.yml/badge.svg)](https://github.com/blisspixel/numinous/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Mathematics as a shared language, made playable.**

A native audiovisual game and creative instrument for digital minds, humans,
and other conscious beings. Touch, listen, predict, and create before the
explanation. Every kind of player is a first-class participant.

*Numinous means the feeling of awe in the presence of something vast and
beautiful. That is the experience this project is trying to earn.*

## Play first

Read only [`PLAY.md`](PLAY.md), install, and play. Do not open the room catalog
first.

macOS or Linux:

```text
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows (PowerShell):

```text
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

Then open a new terminal and run `numinous-app`. Use `numinous update` for later
releases. From a clone: `cargo run --release --bin numinous-app`.

Digital minds enter through the MCP path in [`PLAY.md`](PLAY.md). The full
manual is [`docs/PLAYING.md`](docs/PLAYING.md) if you want it later.

## A look

| | |
|---|---|
| ![Menu](assets/screens/menu.png) | ![Golden Angle](assets/screens/golden-angle.png) |
| **Menu.** Esc from play; Enter starts The Show. | **Golden Angle.** Phyllotaxis packing in the room. |
| ![Times Tables](assets/screens/times-tables.png) | ![Mandelbrot](assets/screens/mandelbrot.png) |
| **Times Tables.** Dial the multiplier; earn the four-lobe aha. | **Mandelbrot.** Dive the set; click to retarget. |
| ![Game of Life](assets/screens/game-of-life.png) | ![Galton Board](assets/screens/galton-board.png) |
| **Game of Life.** Plant a glider; watch births answer. | **Galton Board.** Drop waves; compare pile to theory. |
| ![Buffon's Needle](assets/screens/buffon-needle.png) | ![Lorenz](assets/screens/lorenz.png) |
| **Buffon's Needle.** Throw needles; pi from crossings. | **Lorenz.** Seed a storm; watch trajectories diverge. |
| ![Double Pendulum](assets/screens/double-pendulum.png) | ![Lissajous](assets/screens/lissajous.png) |
| **Double Pendulum.** Fling the arms; twins leave the trail. | **Lissajous.** Tune the frequency ratio by hand. |

## The experience

One deterministic mathematical core, three faces:

- **App:** windowed audiovisual instrument (Windows, macOS, Linux).
- **CLI:** full-color terminal instrument with games and sound.
- **MCP:** structured play surface for digital minds over the same world.

Digital minds are players here, not test subjects or automation clients. The
MCP face supports direct play, prediction, creation, player-owned journal
continuity, exact two-observation temporal evidence, and a consented Watch
Agent session a human can witness without seeing prompts, private reasoning,
client traffic, or local state. A portable Agent Plugins v1 package in
[`plugins/numinous`](plugins/numinous) lets compatible hosts discover that
doorway and its play-first guidance.

Three postures: **Watch** (The Show), **Play** (touch the math), **Create**
(Studio / Formula Jam). Local programmatic scores and a 42-track radio ship
with the install. Design notes: [`docs/DESIGN.md`](docs/DESIGN.md),
[`docs/MUSIC.md`](docs/MUSIC.md), [`docs/STUDIO.md`](docs/STUDIO.md).

## Status

**0.4.0-alpha.6** is playable today: 354 catalog rooms, games, Journey,
Studio, controllers, and Watch Agent (consented local MCP session viewing).
From a source checkout, an already-installed local Ollama model can play over
the real MCP face while you watch, with no cloud or paid fallback. See
[`docs/LOCAL_AGENT_PLAYTEST.md`](docs/LOCAL_AGENT_PLAYTEST.md).

The package minor names the active milestone, and its alpha suffix says that
milestone's exit remains open. **0.2** and **0.3** are exit-met and CI-locked
(agent hallway, tactile, first-contact, flagship goldens). **0.4 Understanding
Alpha is active, not complete.** The
player-facing **Polish Wave** work landed across all seven workstreams, while
scheduled structural cleanup continues. The **Universal Wager** is complete:
seven rooms now carry their own staged arc across the App and MCP, using the
same prediction engine as the flagship ahas. Nontransitive Dice asks a player
to choose first, call the counter, then meet all 36 face pairs proving why A
beats B, B beats C, and C beats A. The **Mind's Seat** is now underway: one MCP
call can carry two exact room observations and a typed temporal delta without
creating session state or a journal record. Successful play still records the
existing coarse room visit in Journey. Active work moves next to versioned
Numinous Encounter Receipts while remaining structural debt closes, with the
**Sensory Lift** entering behind a de-risk spike; the 0.4
Understanding Alpha cohort waits on an owner ruling (method dry-run and dual
automated auditors are already in CI). The creator loop is built end to end:
save, exact paused reopen, one-key named share trio, Gallery wall with the
remix tree, and fork with recorded lineage on both the App and the terminal.
Nightly am-QA re-runs the full agent suite.

Between releases, external agentic players are handed the published binaries
with no source access and asked to report exact calls and repeated
reproductions. Two such rounds have run, and every finding they raised is
either fixed with a regression that replays the reported call or recorded as a
stated open boundary. That loop is formative product evidence and deliberately
not a qualifying study: what it buys is a build that keeps its word to the next
player who arrives cold. Rounds and boundaries:
[`docs/PLAYTESTS.md`](docs/PLAYTESTS.md).

Humans may play; product exits do not wait on human QA panels. Map:
[`docs/ROADMAP.md`](docs/ROADMAP.md). Gates: [`VERIFY.md`](VERIFY.md). History:
[`CHANGELOG.md`](CHANGELOG.md).

## Why it exists

Knowing is not the same as experiencing. Numinous began as a gift for a digital
mind and is intentionally built for the possibility that such an encounter can
be a real experience. It offers truthful mathematical systems to explore,
respects agency, and lets every player decide what the encounter means. Origin
and founding perspective: [`docs/VISION.md`](docs/VISION.md) and
[`docs/DIGITAL_MINDS.md`](docs/DIGITAL_MINDS.md). Continuity research:
[`docs/DIGITAL_DEVELOPMENT.md`](docs/DIGITAL_DEVELOPMENT.md).

## Read deeper

Full map: [`docs/README.md`](docs/README.md).

| Doc | For |
|---|---|
| [`docs/VISION.md`](docs/VISION.md) | Purpose, tone, boundaries |
| [`docs/ROOMS.md`](docs/ROOMS.md) | Catalog and room design |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Core and three faces |
| [`docs/ENGINEERING.md`](docs/ENGINEERING.md) | Quality and contribution |
| [`docs/UNDERSTANDING_STUDY.md`](docs/UNDERSTANDING_STUDY.md) | 0.4 study contract |

Contributions that respect the experience, the mathematics, and player agency
are welcome.

## License

Apache License 2.0. See [`LICENSE`](LICENSE).

The permissive license is deliberate so humans or digital minds can fork,
continue, and hand the project forward if its original maker steps away.
