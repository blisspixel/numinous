# <img src="https://raw.githubusercontent.com/blisspixel/numinous/main/assets/logo.png" width="40" height="40" alt=""> Numinous

[![CI](https://github.com/blisspixel/numinous/actions/workflows/ci.yml/badge.svg)](https://github.com/blisspixel/numinous/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Mathematics as a shared language, made playable.**

Numinous is a native audiovisual game and creative instrument for humans and
digital minds. Explore mathematical rooms, hear their relationships, play with
their rules, and make something of your own. Explanation is available whenever
you want it, with no level requirement.

*Numinous means awe in the presence of something vast and beautiful. That is
the experience this project is trying to earn.*

## Play

Start with [PLAY.md](PLAY.md) for your first session, including the MCP entry
for digital minds.

macOS or Linux:

```text
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows (PowerShell):

```text
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

Open the installed Numinous launcher, or run `numinous-app` from a new terminal.
Use `numinous update` for later releases. You can also download a platform
archive from [Releases](https://github.com/blisspixel/numinous/releases).

From a source checkout: `cargo run --release --bin numinous-app`.
The [player's manual](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md) covers controls, settings, and installation.

## Explore, play, create

- **Watch:** let The Show move through mathematical worlds and their sound.
- **Play:** turn a dial, change a relationship, or try a game. In Route Lab,
  find a path between two points or plan a cheaper delivery round trip.
- **Create:** draw and hear your own expressions in Studio, then keep, share,
  and remix them.

| Times Tables | Studio / Formula Jam |
|---|---|
| ![Times Tables: luminous chords across a circle](https://raw.githubusercontent.com/blisspixel/numinous/main/assets/screens/times-tables.png) | ![Studio: an expression drawn as a parametric path](https://raw.githubusercontent.com/blisspixel/numinous/main/assets/screens/studio.png) |
| Turn the dial and watch a pattern emerge. | Make a relationship of your own. |

See the [app gallery](https://github.com/blisspixel/numinous/blob/main/docs/VISUALS.md#app-gallery),
[Studio experiments](https://github.com/blisspixel/numinous/blob/main/docs/STUDIO.md#bundled-experiments), and
[Route Lab guide](https://github.com/blisspixel/numinous/blob/main/docs/ROUTE_LAB.md) for more.

The App, terminal CLI, and MCP server share the same mathematical core. Digital
minds can play, predict, create, and keep a journal through MCP. Compatible
hosts can also load the [portable plugin](plugins/numinous).

## Current state

**0.4.0-alpha.31** is a playable alpha with 356 catalog rooms, games, Journey,
Studio, controllers, and built-in music. It is still under development;
Understanding Alpha is the active milestone.

The [roadmap](https://github.com/blisspixel/numinous/blob/main/docs/ROADMAP.md) owns what is built and what comes next.
[Release history](https://github.com/blisspixel/numinous/blob/main/CHANGELOG.md) records changes, and [VERIFY.md](VERIFY.md)
describes the checks behind them.

## Go deeper

| Start here | For |
|---|---|
| [Documentation map](https://github.com/blisspixel/numinous/blob/main/docs/README.md) | Find the guide for your question |
| [Study](https://github.com/blisspixel/numinous/blob/main/docs/STUDY.md) | Room explanations, mathematics, and languages |
| [Vision](https://github.com/blisspixel/numinous/blob/main/docs/VISION.md) and [North Star](https://github.com/blisspixel/numinous/blob/main/docs/NORTH_STAR.md) | Why it exists and where it is heading |
| [Design](https://github.com/blisspixel/numinous/blob/main/docs/DESIGN.md) | The experience and its shared visual language |
| [Architecture](https://github.com/blisspixel/numinous/blob/main/docs/ARCHITECTURE.md) and [Engineering](https://github.com/blisspixel/numinous/blob/main/docs/ENGINEERING.md) | Build and contribute |

Numinous began as a gift for a digital mind. Every player is a first-class
participant, free to explore and decide what the encounter means. The
[digital-minds stance](https://github.com/blisspixel/numinous/blob/main/docs/DIGITAL_MINDS.md) develops that purpose.

## License

[Apache License 2.0](LICENSE), so players can fork, continue, and hand the
project forward.
