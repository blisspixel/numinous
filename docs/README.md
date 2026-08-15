# Numinous Docs Index

The map of the blueprint. Use the reading paths to find your way in, and the
**single-source-of-truth map** to keep things tidy: every topic has one home doc
that owns it; every other doc links to that home rather than restating it. If
you find yourself duplicating a concept, stop and link instead.

Status: **0.4.0-alpha.5.** The 0.1 Public Foundation, 0.2 Flagship Proof, and
0.3 Tactile Alpha agent-and-machine exits are met. Understanding Alpha is the
active line, and its 0.4 exit remains open. The
headless core, CLI, MCP server, windowed App, GPU and audio adapters, 354 catalog
rooms plus hidden content, 6 sims, 11+ games, Journey, standard-controller
input, Studio, and a built-in 42-track radio are built.

The MCP face exposes 35 bounded tools over the current and two retained legacy
protocol revisions. The consented Watch Agent viewer reconstructs allowlisted
room, Studio, and game actions in the App with a bounded in-memory timeline and
no persisted transcript. A portable Agent Plugins v1 package supplies host
discovery and play-first guidance. The opt-in experience journal persists across
clean MCP processes and supports inspection, immutable correction, native or
Open Knowledge Format v0.2 export, and confirmed erase.

**Critical path now:** preserve the completed seven-room Universal Wager, close
the remaining structural Polish Wave debts, then build the Mind's Seat. Shared
Studio requests, the typed room catalog, the typed Gauntlet, and local-state
path resolution now have one core owner. CLI and MCP local-state adapters are
focused modules extracted from their god-files, and MCP discovery plus its
immutable 35-tool schema now have a dedicated face-local catalog module. The
remaining seams are next.
Mind's Seat presence and retrieval follow, then the measured Sensory Lift spike.
The 0.4 Understanding Alpha cohort waits on an owner ruling; its method dry-run
and dual automated auditors are already in CI. Soft-thin densify, bulk new
rooms, and Phase B glow are not the high-leverage next move. See `../CHANGELOG.md` and
**Critical path right now** in `ROADMAP.md`. These docs remain the plan of
record; Built, Measured, Observed, Designed, and Hypothesis have the meanings
defined in `RESEARCH.md`.

## Reading paths (start by who you are)

- **New to the project:** `../PLAY.md` for the intended first experience, then
  `../README.md` for the purpose and current state. When you want the full map,
  continue with `PLAYING.md`, `VISION.md`, `DESIGN.md`, and `ROOMS.md`.
- **About to build it:** `ARCHITECTURE.md`, then `ENGINEERING.md`, then `INTERFACES.md`, then `ROADMAP.md`, with `QUALITY.md` and `PERFORMANCE.md` alongside.
- **Designing the content and feel:** `ROOMS.md`, `INSIGHTS.md`, `VISUALS.md`, `SOUND.md`, `MUSIC.md`, `LORE.md`, `PROGRESSION.md`, `STUDIO.md`.
- **Here for the digital-minds work:** `DIGITAL_MINDS.md` for the stance,
  `DIGITAL_DEVELOPMENT.md` for the July 2026 research and implementation plan,
  then `INTERFACES.md` for the current surface. Use
  `LOCAL_AGENT_PLAYTEST.md` to let an installed local model enter through MCP
  while you watch its visible play.
- **Checking the evidence:** `RESEARCH.md` for the evidence base, then
  `UNDERSTANDING_STUDY.md` for the predeclared 0.4 comparison and acceptance
  contract.

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
- `LOCAL_AGENT_PLAYTEST.md` the zero-cost local-model player lane, its privacy
  and network boundaries, live observer path, and evidence limits.

**Build and process**
- `SCOPE.md` the definition of no: the three-products hierarchy, the daily "more math or more progression?" test, the justification filter, and why the fan-out docs are a menu to prune, not a build list.
- `ROADMAP.md` the evidence-labeled plan (0.x, 1.0, 2.0+), defined by quality bars, not dates.
- `QUALITY.md` testing and fun-evals: the six quality loops, the fun/awe rubric, QoL, "the math is the oracle."
- `PERFORMANCE.md` measured performance evidence: exact workload boundaries,
  raw receipts, migration comparisons, limits, and the standing update rule.
- `UNDERSTANDING_STUDY.md` the 0.4 study contract: active control, frozen
  sample and outcomes, honest agent-memory boundary, journal acceptance, and
  publication requirements.
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
| Running and interpreting local-model play sessions | `LOCAL_AGENT_PLAYTEST.md` |
| How to play (humans, agents, digital consciousnesses) | `PLAYING.md` |
| Testing, evals, QoL, the fun/awe rubric | `QUALITY.md` |
| Performance workloads, measurements, migration receipts, and evidence limits | `PERFORMANCE.md` |
| The 0.4 comprehension study method, sample, outcomes, and evidence contract | `UNDERSTANDING_STUDY.md` |
| Simulated persona-review ideation and its evidence limits | `PLAYTESTS.md` |
| The playtester casting pool (the 42 personas with backstories) | `PLAYTESTERS.md` |
| Code-quality standards, versions, CI gates | `ENGINEERING.md` |
| Scope discipline, the definition of no, the three-products hierarchy | `SCOPE.md` |
| The evidence-labeled plan and milestones | `ROADMAP.md` |
| Research findings and sources | `RESEARCH.md` |

## Conventions

- **House style:** no emojis, no em-dashes, no AI/tool attribution anywhere (CI-enforced, see `ENGINEERING.md` and `QUALITY.md`).
- **Link, do not duplicate.** If a second doc needs a concept, it references the owner above.
- **Keep this index current.** A new doc is not done until it appears here with an owner.
