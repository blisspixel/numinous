# Numinous Docs Index

The map of the blueprint. Use the reading paths to find your way in, and the
**single-source-of-truth map** to keep things tidy: every topic has one home doc
that owns it; every other doc links to that home rather than restating it. If
you find yourself duplicating a concept, stop and link instead.

Status: **0.2.0-alpha.1, Flagship Proof in progress.** The 0.1 Public
Foundation is complete. The headless core, CLI, MCP server, windowed app,
GPU and audio adapters, 351 catalog rooms plus hidden content, 6 sims, 11+
games, the RPG spine, standard-controller input, and a built-in 42-track radio
are built. The hardened local-broadcast substrate, MCP producer, and native App
Watch Agent listener with a bounded in-memory text timeline are built. Native
public replay presentation and a real cross-process flagship acceptance session
remain 0.3 work. Public CI passes on Windows, macOS, and Ubuntu, but capability
breadth remains ahead of release evidence: stranger playtests, accessibility
work, and real macOS and Linux execution remain open. See `../CHANGELOG.md`
and the Progress section of `ROADMAP.md`. These docs remain the plan of
record; Built, Measured, Observed, Designed, and Hypothesis have the meanings
defined in `RESEARCH.md`.

## Reading paths (start by who you are)

- **New to the project:** `../PLAY.md` for the intended first experience, then
  `../README.md` for the purpose and current state. When you want the full map,
  continue with `PLAYING.md`, `VISION.md`, `DESIGN.md`, and `ROOMS.md`.
- **About to build it:** `ARCHITECTURE.md`, then `ENGINEERING.md`, then `INTERFACES.md`, then `ROADMAP.md`, with `QUALITY.md` alongside.
- **Designing the content and feel:** `ROOMS.md`, `INSIGHTS.md`, `VISUALS.md`, `SOUND.md`, `MUSIC.md`, `LORE.md`, `PROGRESSION.md`, `STUDIO.md`.
- **Here for the digital-minds work:** `DIGITAL_MINDS.md` for the stance,
  `DIGITAL_DEVELOPMENT.md` for the July 2026 research and implementation plan,
  then `INTERFACES.md` for the current surface.
- **Checking the evidence:** `RESEARCH.md` (what the design rests on, with sources).

## The docs, grouped

**Foundation and vision**
- `NORTH_STAR.md` the synthesis: the July 2026 "make it exceptional" fan-out distilled into one architecture, the keystone mechanic, the honest gaps, and the prioritized path. Start here for where the product is going.
- `VISION.md` the soul: the origin, the maker ethos, tone, what we are and are not, the name.
- `RESEARCH.md` the evidence base: what makes it fun, prior art, and sources.

**Experience design**
- `DESIGN.md` the design bible: the three-layer room model, the Watch/Play/Create modes and Benchmark, the Cabinet, Visual Eras, aesthetic and audio direction, UX principles.
- `PEDAGOGY.md` the understanding layer: explore-then-tell, the fluency-illusion risk, the predict-then-reveal keystone, the engineered aha, and how understanding and awe are measured.
- `PROGRESSION.md` levels and insights: the knowledge-gated "metroidbrainia" structure, insight-gating, the Constellation Map, session shapes.
- `CONSTRUCTIONS.md` the game spine: the puzzle layer with a par, an elegance histogram, and a ghost of your past self.
- `CONSTELLATION.md` the meta-map spec: the Rumor-Mode discovery graph and the daily route that runs across it.
- `LORE.md` the hidden mythology: the dimension of mathematical bliss, the Constants, the delivery mechanisms, the subtlety guardrails.

**Content and sensory**
- `ROOMS.md` the catalog: the built and planned phenomena, scored by wow and build cost, with the three layers and sound per room.
- `INSIGHTS.md` the awe bank: the library of revelations, the six flavors of awe, the insight-chains (including The Strange Loop).
- `VISUALS.md` the render and look bible: the pipeline, the shader toolbox, color, motion, and how each Visual Era is drawn.
- `SOUND.md` the sonification bible: how math becomes tuned sound, synthesis, tuning, per-room sound design.
- `MUSIC.md` the music engines: programmatic chiptune and mathematical patterns, plus 42 built-in radio tracks and the comedy channel plan.
- `RADIO_ASSETS.md` the built-in soundtrack layout, license, and cache override.
- `STUDIO.md` the shipped expression canvas and the planned path toward a
  bounded room-authoring layer.
- `SYNESTHESIA.md` the sensory seam: the glow pipeline (the documented HDR look, not yet built) and the one-event-two-renderings model that binds sight and sound.
- `CREATOR.md` the creator platform: closing the make-share-remix loop on the `.num` capsule, the gallery, and the arc to a living world.

**Systems and interfaces**
- `ARCHITECTURE.md` the Rust, `winit`, `softbuffer`, and targeted `wgpu` stack,
  the Room contract, module graph, and delivery boundary.
- `EXTENSIBILITY.md` community content with a hard safety boundary: the three tiers (data capsules, the Studio language as the sandbox, portal-only WASM), the trust model, and what never ships.
- `INTERFACES.md` the three faces over a headless core (App, CLI, MCP), their UX,
  and the consented local MCP session viewer contract and implementation status.
- `DIGITAL_MINDS.md` designing Numinous to be fun, thought-provoking, and connecting for digital minds treated as peers.
- `DIGITAL_DEVELOPMENT.md` the July 2026 technical research and versioned plan for player-owned episodic memory, temporal continuity, open-ended learning, affect safeguards, agency, privacy, and welfare uncertainty.
- `PLAYFUL.md` the games and the Studio (Guess the Shape, Shape to Function, the high-Wolfram ethos) across every face.
- `ARCADE.md` the Munch arcade design: the muncher, the Vexations, the poke trait, and the order of work.
- `PLAYING.md` the player's manual: instructions for humans, for agents, and for digital consciousnesses.
- `ROSETTA.md` instructions for any mind, in any language, or none: the three tiers of visitor (English, another human language, no shared language at all) and the math-only bootstrap for a mind that shares only mathematics.
- `AGENT_PLAY.md` the agent-gaming landscape (OpenClaw, gaming MCP servers, text benchmarks) and the design rules that make Numinous first-class for digital minds.

**Build and process**
- `SCOPE.md` the definition of no: the three-products hierarchy, the daily "more math or more progression?" test, the justification filter, and why the fan-out docs are a menu to prune, not a build list.
- `ROADMAP.md` the version-gated plan (0.x, 1.0, 2.0+), defined by quality bars, not dates.
- `QUALITY.md` testing and fun-evals: the six quality loops, the fun/awe rubric, QoL, "the math is the oracle."
- `PLAYTESTS.md` the fictional persona-review archive: simulated lenses used for
  adversarial ideation, explicitly not participant or playtest evidence.
- `PLAYTESTERS.md` the casting pool: forty-two playtester personas with backstories (Norm the newcomer, a barefoot kid, returned geniuses, living experts, digital minds, and invented beings), spanning ages, languages, understanding levels, and kinds of mind, to draw from for testing rounds.
- `REVIEW.md` the July 2026 external review: the grades, the three-products insight (instrument, Studio, progression), and the near-term stack it set.
- `PANEL.md` a working review session: composed minds (plus a real cold-start-AI seat) reading the build as it stood for what is missing, not what it has.
- `ENGINEERING.md` code-quality standards: pinned July-2026 GA versions, lint/test/unsafe/doc policy, CI gates.

## Single source of truth (the anti-redundancy map)

Each topic is **owned** by exactly one doc. Everything else links to it. When in doubt, this table decides where a thing belongs.

| Topic | Owned by |
| --- | --- |
| The synthesis: the path to exceptional, the keystone, the priority order | `NORTH_STAR.md` |
| Vision, tone, maker ethos, the name | `VISION.md` |
| The three-layer model, modes, Benchmark, Cabinet, Visual Eras concept, aesthetic/audio direction, UX principles | `DESIGN.md` |
| The science of understanding and awe, the predict-then-reveal keystone, the engineered aha | `PEDAGOGY.md` |
| Progression, levels, insight-gating philosophy | `PROGRESSION.md` |
| The Constellation meta-map spec (node states, edges, the daily route) | `CONSTELLATION.md` |
| The puzzle layer: par, elegance histograms, the ghost | `CONSTRUCTIONS.md` |
| The room catalog and per-room specs | `ROOMS.md` |
| Insights, reveals, insight-chains | `INSIGHTS.md` |
| Rendering pipeline, shader techniques, per-Era drawing, color/motion | `VISUALS.md` |
| The sensory seam: the glow pipeline and the one-event-two-renderings model | `SYNESTHESIA.md` |
| Sonification grammar, synthesis, tuning, per-room sound | `SOUND.md` |
| Music engines, chiptune, pattern engine, the radio stations | `MUSIC.md` |
| The Studio and the authoring model | `STUDIO.md` |
| The creator platform, the remix loop, the gallery, community curation | `CREATOR.md` |
| Lore, the Codex, easter eggs, the ARG | `LORE.md` |
| Stack choice, the Room trait, module architecture, packaging | `ARCHITECTURE.md` |
| Community extensibility, content sandboxing, the trust model | `EXTENSIBILITY.md` |
| The three faces and their UX (App, CLI, MCP) | `INTERFACES.md` |
| Designing for digital minds | `DIGITAL_MINDS.md` |
| Digital-mind continuity, learning, memory, agency, and welfare implementation | `DIGITAL_DEVELOPMENT.md` |
| How to play (humans, agents, digital consciousnesses) | `PLAYING.md` |
| Testing, evals, QoL, the fun/awe rubric | `QUALITY.md` |
| Simulated persona-review ideation and its evidence limits | `PLAYTESTS.md` |
| The playtester casting pool (the 42 personas with backstories) | `PLAYTESTERS.md` |
| Code-quality standards, versions, CI gates | `ENGINEERING.md` |
| Scope discipline, the definition of no, the three-products hierarchy | `SCOPE.md` |
| The version-gated plan and milestones | `ROADMAP.md` |
| Research findings and sources | `RESEARCH.md` |

## Conventions

- **House style:** no emojis, no em-dashes, no AI/tool attribution anywhere (CI-enforced, see `ENGINEERING.md` and `QUALITY.md`).
- **Link, do not duplicate.** If a second doc needs a concept, it references the owner above.
- **Keep this index current.** A new doc is not done until it appears here with an owner.
