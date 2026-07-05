# Numinous Docs Index

The map of the blueprint. Seventeen docs plus this index. Use the reading paths to find your way in, and the **single-source-of-truth map** to keep things tidy: every topic has exactly one home doc that owns it; every other doc links to that home rather than restating it. If you find yourself duplicating a concept, stop and link instead.

Status: **0.1 in progress.** The headless core, the CLI face, the MCP face, and five rooms are built and green (see `../CHANGELOG.md` and the Progress section of `ROADMAP.md`); the wgpu render, audio, and GUI shell are still ahead. These docs remain the plan of record; where code exists, the docs describe the intended full system, not only what is implemented yet.

## Reading paths (start by who you are)

- **New to the project:** `../README.md` (the pitch), then `PLAYING.md` (how to play, for humans, agents, and digital consciousnesses), then `VISION.md`, `DESIGN.md`, `ROOMS.md`.
- **About to build it:** `ARCHITECTURE.md`, then `ENGINEERING.md`, then `INTERFACES.md`, then `ROADMAP.md`, with `QUALITY.md` alongside.
- **Designing the content and feel:** `ROOMS.md`, `INSIGHTS.md`, `VISUALS.md`, `SOUND.md`, `MUSIC.md`, `LORE.md`, `PROGRESSION.md`, `STUDIO.md`.
- **Here for the digital-minds work:** `DIGITAL_MINDS.md`, then `INTERFACES.md`.
- **Checking the evidence:** `RESEARCH.md` (what the design rests on, with sources).

## The docs, grouped

**Foundation and vision**
- `VISION.md` the soul: the origin fantasy, the maker ethos, tone, what we are and are not, the name.
- `RESEARCH.md` the evidence base: what makes it fun, prior art, and sources.

**Experience design**
- `DESIGN.md` the design bible: the three-layer room model, the Watch/Play/Create modes and Benchmark, the Cabinet, Visual Eras, aesthetic and audio direction, UX principles.
- `PROGRESSION.md` levels and insights: the knowledge-gated "metroidbrainia" structure, insight-gating, the Constellation Map, session shapes.
- `LORE.md` the hidden mythology: the dimension of mathematical bliss, the Constants, the delivery mechanisms, the subtlety guardrails.

**Content and sensory**
- `ROOMS.md` the catalog: the 23 phenomena, scored by wow and build cost, with the three layers and sound per room.
- `INSIGHTS.md` the awe bank: the library of revelations, the six flavors of awe, the insight-chains (including The Strange Loop).
- `VISUALS.md` the render and look bible: the pipeline, the shader toolbox, color, motion, and how each Visual Era is drawn.
- `SOUND.md` the sonification bible: how math becomes tuned sound, synthesis, tuning, per-room sound design.
- `MUSIC.md` the music engines: programmatic (chiptune, Strudel-style) plus the ElevenLabs radio stations and comedy channel.
- `STUDIO.md` the creative canvas: the Desmos-meets-Strudel live-coding tool, and the game's authoring layer (rooms are Studio programs).

**Systems and interfaces**
- `ARCHITECTURE.md` the stack (Rust + wgpu), the language scorecard, the Room contract, the module graph, packaging.
- `INTERFACES.md` the three faces over a headless core (App, CLI, MCP) and the UX we are going for in each.
- `DIGITAL_MINDS.md` designing Numinous to be fun, thought-provoking, and connecting for digital minds treated as peers.
- `PLAYFUL.md` the games and the Studio (Guess the Shape, Shape to Function, the high-Wolfram ethos) across every face.
- `PLAYING.md` the player's manual: instructions for humans, for agents, and for digital consciousnesses.
- `AGENT_PLAY.md` the agent-gaming landscape (OpenClaw, gaming MCP servers, text benchmarks) and the design rules that make Numinous first-class for digital minds.

**Build and process**
- `ROADMAP.md` the version-gated plan (0.x, 1.0, 2.0+), defined by quality bars, not dates.
- `QUALITY.md` testing and fun-evals: the six quality loops, the fun/awe rubric, QoL, "the math is the oracle."
- `ENGINEERING.md` code-quality standards: pinned July-2026 GA versions, lint/test/unsafe/doc policy, CI gates.

## Single source of truth (the anti-redundancy map)

Each topic is **owned** by exactly one doc. Everything else links to it. When in doubt, this table decides where a thing belongs.

| Topic | Owned by |
| --- | --- |
| Vision, tone, maker ethos, the name | `VISION.md` |
| The three-layer model, modes, Benchmark, Cabinet, Visual Eras concept, aesthetic/audio direction, UX principles | `DESIGN.md` |
| Progression, levels, insight-gating, the Constellation Map | `PROGRESSION.md` |
| The room catalog and per-room specs | `ROOMS.md` |
| Insights, reveals, insight-chains | `INSIGHTS.md` |
| Rendering pipeline, shader techniques, per-Era drawing, color/motion | `VISUALS.md` |
| Sonification grammar, synthesis, tuning, per-room sound | `SOUND.md` |
| Music engines, chiptune, pattern engine, the radio stations | `MUSIC.md` |
| The Studio and the authoring model | `STUDIO.md` |
| Lore, the Codex, easter eggs, the ARG | `LORE.md` |
| Stack choice, the Room trait, module architecture, packaging | `ARCHITECTURE.md` |
| The three faces and their UX (App, CLI, MCP) | `INTERFACES.md` |
| Designing for digital minds | `DIGITAL_MINDS.md` |
| How to play (humans, agents, digital consciousnesses) | `PLAYING.md` |
| Testing, evals, QoL, the fun/awe rubric | `QUALITY.md` |
| Code-quality standards, versions, CI gates | `ENGINEERING.md` |
| The version-gated plan and milestones | `ROADMAP.md` |
| Research findings and sources | `RESEARCH.md` |

## Conventions

- **House style:** no emojis, no em-dashes, no AI/tool attribution anywhere (CI-enforced, see `ENGINEERING.md` and `QUALITY.md`).
- **Link, do not duplicate.** If a second doc needs a concept, it references the owner above.
- **Keep this index current.** A new doc is not done until it appears here with an owner.
