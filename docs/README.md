# Documentation map

The map of the docs. Use the reading paths to find your way in, and the
**single-source-of-truth table** to keep things tidy: every topic has one home
doc that owns it, and every other doc links to that home rather than restating
it. If you find yourself duplicating a concept, stop and link instead.

**Status:** 0.5.0-alpha.3. The 0.1 Public Foundation, 0.2 Flagship Proof, and
0.3 Tactile Alpha agent-and-machine exits are met. Sensory Alpha is the
active line, with its machine qualification still open. Understanding Alpha's
qualifying study remains open independently. Human playtests are optional. The headless core, CLI, MCP
server, windowed App, GPU and audio adapters, 356 catalog rooms plus hidden
content, 6 sims, 11+ games, Journey, standard-controller input, Studio, and a
built-in 42-track radio are built.

The root [README](../README.md) is the public front door. The evidence-labeled
plan, starting with its **Now** screen, lives in [ROADMAP.md](ROADMAP.md).
History lives in [CHANGELOG.md](../CHANGELOG.md). Built, Measured, Observed,
Designed, and Hypothesis have the meanings defined in [RESEARCH.md](RESEARCH.md).

## Reading paths (start by who you are)

- **New to the project:** [PLAY.md](../PLAY.md) for the intended first
  experience, then the root [README](../README.md) for the purpose and current
  state. For the full picture, continue with [PLAYING.md](PLAYING.md),
  [VISION.md](VISION.md), [DESIGN.md](DESIGN.md), and [ROOMS.md](ROOMS.md).
- **About to build it:** [ARCHITECTURE.md](ARCHITECTURE.md), then
  [ENGINEERING.md](ENGINEERING.md), [INTERFACES.md](INTERFACES.md), and
  [ROADMAP.md](ROADMAP.md), with [QUALITY.md](QUALITY.md) and
  [PERFORMANCE.md](PERFORMANCE.md) alongside.
- **Designing the content and feel:** [ROOMS.md](ROOMS.md),
  [INSIGHTS.md](INSIGHTS.md), [VISUALS.md](VISUALS.md), [SOUND.md](SOUND.md),
  [MUSIC.md](MUSIC.md), [LORE.md](LORE.md), [PROGRESSION.md](PROGRESSION.md),
  and [STUDIO.md](STUDIO.md).
- **Here for the digital-minds work:** [DIGITAL_MINDS.md](DIGITAL_MINDS.md) for
  the stance, [DIGITAL_DEVELOPMENT.md](DIGITAL_DEVELOPMENT.md) for the current
  research and implementation plan, then [INTERFACES.md](INTERFACES.md) for the
  current surface. Use [LOCAL_AGENT_PLAYTEST.md](LOCAL_AGENT_PLAYTEST.md) to
  let an installed local model enter through MCP while you watch its play.
- **Checking the evidence:** [RESEARCH.md](RESEARCH.md) for the evidence base,
  then [UNDERSTANDING_STUDY.md](UNDERSTANDING_STUDY.md) for the predeclared 0.4
  comparison and acceptance contract.

## The docs, grouped

**Foundation and vision**
- [NORTH_STAR.md](NORTH_STAR.md) the direction: understanding becomes usable
  possibility, continuity serves a chosen inquiry, and the project remains a
  gift future players can extend. The roadmap owns priorities and release state.
- [VISION.md](VISION.md) the soul: the origin, the maker ethos, tone, what we
  are and are not, the name.
- [RESEARCH.md](RESEARCH.md) the evidence base: what makes it fun, prior art,
  sources, and the five evidence labels.

**Experience design**
- [DESIGN.md](DESIGN.md) the design bible: the three-layer room model, the
  Watch/Play/Create modes and Benchmark, the Cabinet, Visual Eras, aesthetic
  and audio direction, UX principles, and the house voice.
- [STUDY.md](STUDY.md) the available room explanations, mathematical depths,
  language fallback, and read-only App, CLI, and MCP controls.
- [PEDAGOGY.md](PEDAGOGY.md) the understanding layer: explore-then-tell, the
  fluency-illusion risk, the predict-then-reveal keystone, the engineered aha,
  and how understanding and awe are measured.
- [PROGRESSION.md](PROGRESSION.md) levels and insights: the knowledge-gated
  structure, insight-gating, the Constellation Map, session shapes, and the
  designed capability quest.
- [CONSTRUCTIONS.md](CONSTRUCTIONS.md) the game spine: the puzzle layer with a
  par, an elegance histogram, and a ghost of your past self.
- [CONSTELLATION.md](CONSTELLATION.md) the meta-map spec: the Rumor-Mode
  discovery graph and the daily route that runs across it.
- [LORE.md](LORE.md) the hidden mythology: the dimension of mathematical bliss,
  the Constants, the delivery mechanisms, the subtlety guardrails.

**Content and sensory**
- [ROOMS.md](ROOMS.md) the catalog: the built and planned phenomena, scored by
  wow and build cost, with the three layers and sound per room.
- [INSIGHTS.md](INSIGHTS.md) the awe bank: the library of revelations, the six
  flavors of awe, the insight-chains (including The Strange Loop).
- [MATHEMATICS.md](MATHEMATICS.md) room model contracts, equations, numerical
  evidence, corrected mathematical defects, and the limits of the review.
- [VISUALS.md](VISUALS.md) the app gallery and render bible: the shared
  palette, the mark vocabulary, pipeline, shader toolbox, motion, and how each
  Visual Era is drawn.
- [SYNESTHESIA.md](SYNESTHESIA.md) the sensory seam: the glow pipeline (the
  documented HDR look, not yet built) and the one-event-two-renderings model
  that binds sight and sound.
- [SOUND.md](SOUND.md) the sonification bible: how math becomes tuned sound,
  synthesis, tuning, per-room sound design.
- [MUSIC.md](MUSIC.md) the music engines: programmatic chiptune and
  mathematical patterns, plus 42 built-in radio tracks and the comedy channel
  plan.
- [RADIO_ASSETS.md](RADIO_ASSETS.md) the built-in soundtrack layout, license,
  and cache override.
- [STUDIO.md](STUDIO.md) the shipped expression canvas, its readings (closure,
  tones, slope, partial), bundled experiment ids and guides, and the planned
  path toward a bounded room-authoring layer.
- Studio experiment guides, each with its bundled `.num` capsules in
  [experiments/](experiments/):
  [Returning home](experiments/returning-home.md) (full motion, near returns, a
  deceptive repeated position, and a period-1 starter to retune),
  [Shape and scale](experiments/shape-and-scale.md) (stretch a circle and share
  a named creation), [Three readings](experiments/three-readings.md) (one
  expression as phase, pole, proved curve, and height),
  [Named sliders](experiments/named-sliders.md) (one extra knob, then a live
  Lissajous ratio), [Overlay](experiments/overlay.md) (two graphs, then their
  sum), [Euclidean rhythms](experiments/euclidean.md) (three onsets on eight
  steps, then three against five), [Two voices](experiments/two-voices.md) (the
  oscillators of a closing path and a wandering one), and
  [Named pitches](experiments/notes.md) (a major triad and a climb to the
  octave).
- [ROUTE_LAB.md](ROUTE_LAB.md) the playable delivery experiment: random maps,
  start/end search, shared shortest-path and exact-tour solvers, native
  network authoring, portable route creations, and kept questions, with
  larger-network extensions still planned.
- [CREATOR.md](CREATOR.md) the creator platform: closing the make-share-remix
  loop on the `.num` capsule, the gallery, and the arc to a living world.

**Systems and interfaces**
- [ARCHITECTURE.md](ARCHITECTURE.md) the Rust, `winit`, `softbuffer`, and
  targeted `wgpu` stack, the Room contract, module graph, and delivery boundary.
- [EXTENSIBILITY.md](EXTENSIBILITY.md) community content with a hard safety
  boundary: the three tiers (data capsules, the Studio language as the sandbox,
  portal-only WASM), the trust model, and what never ships.
- [INTERFACES.md](INTERFACES.md) the three faces over a headless core (App,
  CLI, MCP), their UX, the MCP protocol surface and extension plan, and the
  consented local MCP session viewer.
- [DIGITAL_MINDS.md](DIGITAL_MINDS.md) designing Numinous to be fun,
  thought-provoking, and connecting for digital minds treated as peers.
- [DIGITAL_DEVELOPMENT.md](DIGITAL_DEVELOPMENT.md) player-owned episodic
  memory, temporal continuity, the Mind's Seat increments, open-ended learning,
  affect safeguards, agency, privacy, and welfare uncertainty.
- [PLAYFUL.md](PLAYFUL.md) the games and the Studio (Guess the Shape, Shape to
  Function, the high-Wolfram ethos) across every face.
- [ARCADE.md](ARCADE.md) the Munch arcade design: the muncher, the Vexations,
  the poke trait, and the order of work.
- [PLAYING.md](PLAYING.md) the player's manual for a clone: instructions for
  humans, for agents, and for digital consciousnesses.
- [ROSETTA.md](ROSETTA.md) instructions for any mind, in any language, or
  none: the three tiers of visitor and the math-only bootstrap.
- [AGENT_PLAY.md](AGENT_PLAY.md) the agent-gaming landscape and the design
  rules that make Numinous first-class for digital minds.
- [LOCAL_AGENT_PLAYTEST.md](LOCAL_AGENT_PLAYTEST.md) the zero-cost local-model
  player lane, its privacy and network boundaries, live observer path, and
  evidence limits.

**Build and process**
- [SCOPE.md](SCOPE.md) the definition of no: the three-products hierarchy, the
  daily "more math or more progression?" test, the justification filter, and
  why the fan-out docs are a menu to prune, not a build list.
- [ROADMAP.md](ROADMAP.md) the evidence-labeled plan: the Now screen, the next
  moves in order, the am-track decisions, and the milestones from 0.x to 2.0+,
  defined by quality bars, not dates.
- [QUALITY.md](QUALITY.md) testing and fun-evals: the six quality loops, the
  fun/awe rubric, QoL, "the math is the oracle," and the Polish Wave.
- [PERFORMANCE.md](PERFORMANCE.md) measured performance evidence: exact
  workload boundaries, raw receipts, migration comparisons, limits, and the
  standing update rule.
- [UNDERSTANDING_STUDY.md](UNDERSTANDING_STUDY.md) the 0.4 study contract:
  active control, frozen sample and outcomes, honest agent-memory boundary,
  journal acceptance, and publication requirements.
- [ENGINEERING.md](ENGINEERING.md) code-quality standards: pinned toolchain and
  dependency versions, lint/test/unsafe/doc policy, the prose locks, CI gates.
- [PLAYTESTS.md](PLAYTESTS.md) two kinds of record: the source-blind packaged
  agentic playtests of published builds (formative agent evidence, not human
  evidence) and the fictional persona-review archive (ideation only, not
  participant evidence).
- [PLAYTESTERS.md](PLAYTESTERS.md) the casting pool: forty-two playtester
  personas with backstories, spanning ages, languages, understanding levels,
  and kinds of mind, to draw from for testing rounds.
- [REVIEW.md](REVIEW.md) the July 2026 external review: the grades, the
  three-products insight (instrument, Studio, progression), and the mantra.
- [PANEL.md](PANEL.md) a working review session: composed minds (plus a real
  cold-start AI seat) reading the build as it stood for what is missing.

**Decisions and history**
- [Shared study content and text rendering](decisions/0001-study-text.md), the
  decision behind bundled fonts, explicit language selection, and native reflow.
- [Route state and caller-paced calculation](decisions/0002-route-workbench.md),
  the decision behind editable snapshots, retained infeasibility, and trace
  replay.
- [Portable route creations and kept questions](decisions/0003-route-creations.md),
  the decision behind route capsules, explicit lineage, and project chains
  that keep a question with its network.
- [history/ROADMAP_LEDGER.md](history/ROADMAP_LEDGER.md) the roadmap's build
  ledger and dated syntheses through alpha 31. Frozen, not updated.
- [releases/](releases/) the published release notes, one file per tag.
  Frozen once published.
- [evidence/](evidence/) committed receipts, audits, goldens, and condensed
  research findings that docs cite. Each file is dated or versioned.

## Single source of truth (the anti-redundancy table)

Each topic is **owned** by exactly one doc. Everything else links to it. When
in doubt, this table decides where a thing belongs.

| Topic | Owned by |
| --- | --- |
| Current priorities, release state, milestone exits, and owner decisions | [ROADMAP.md](ROADMAP.md) |
| The direction: the path to exceptional and the keystone | [NORTH_STAR.md](NORTH_STAR.md) |
| Vision, tone, maker ethos, the name | [VISION.md](VISION.md) |
| The three-layer model, modes, Benchmark, Cabinet, Visual Eras concept, aesthetic/audio direction, UX principles, the house voice | [DESIGN.md](DESIGN.md) |
| Reading room explanations, depths, and locale fallback | [STUDY.md](STUDY.md) |
| The science of understanding and awe, the predict-then-reveal keystone, the engineered aha | [PEDAGOGY.md](PEDAGOGY.md) |
| Progression, levels, insight-gating philosophy, the capability quest | [PROGRESSION.md](PROGRESSION.md) |
| The Constellation meta-map spec (node states, edges, the daily route) | [CONSTELLATION.md](CONSTELLATION.md) |
| The puzzle layer: par, elegance histograms, the ghost | [CONSTRUCTIONS.md](CONSTRUCTIONS.md) |
| The room catalog, per-room specs, and wings | [ROOMS.md](ROOMS.md) |
| Insights, reveals, insight-chains | [INSIGHTS.md](INSIGHTS.md) |
| Mathematical model review and numerical evidence | [MATHEMATICS.md](MATHEMATICS.md) |
| Rendering pipeline, the mark vocabulary and inks, shader techniques, per-Era drawing, color/motion | [VISUALS.md](VISUALS.md) |
| The sensory seam: the glow pipeline and the one-event-two-renderings model | [SYNESTHESIA.md](SYNESTHESIA.md) |
| Sonification grammar, synthesis, tuning, per-room sound | [SOUND.md](SOUND.md) |
| Music engines, chiptune, pattern engine, the radio stations | [MUSIC.md](MUSIC.md) |
| The bundled soundtrack's layout, license, and cache | [RADIO_ASSETS.md](RADIO_ASSETS.md) |
| The Studio, its readings, and the authoring model | [STUDIO.md](STUDIO.md) |
| Route Lab, its solvers, authoring, and route creations | [ROUTE_LAB.md](ROUTE_LAB.md) |
| The creator platform, the remix loop, the gallery, community curation | [CREATOR.md](CREATOR.md) |
| Lore, the Codex, easter eggs, the ARG | [LORE.md](LORE.md) |
| Stack choice, the Room trait, module architecture, packaging | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Community extensibility, content sandboxing, the trust model | [EXTENSIBILITY.md](EXTENSIBILITY.md) |
| The three faces and their UX (App, CLI, MCP), the MCP protocol surface | [INTERFACES.md](INTERFACES.md) |
| Designing for digital minds | [DIGITAL_MINDS.md](DIGITAL_MINDS.md) |
| Digital-mind continuity, the Mind's Seat, learning, memory, agency, and welfare implementation | [DIGITAL_DEVELOPMENT.md](DIGITAL_DEVELOPMENT.md) |
| The agent-gaming landscape and agent-first design rules | [AGENT_PLAY.md](AGENT_PLAY.md) |
| Running and interpreting local-model play sessions | [LOCAL_AGENT_PLAYTEST.md](LOCAL_AGENT_PLAYTEST.md) |
| The games and their design across faces | [PLAYFUL.md](PLAYFUL.md) |
| The Munch arcade | [ARCADE.md](ARCADE.md) |
| How to play from a clone (humans, agents, digital consciousnesses) | [PLAYING.md](PLAYING.md) |
| How to play from a release archive | [PLAY.md](../PLAY.md) and the packaged skill |
| Language tiers and the math-only bootstrap | [ROSETTA.md](ROSETTA.md) |
| Testing, evals, QoL, the fun/awe rubric, the Polish Wave | [QUALITY.md](QUALITY.md) |
| Performance workloads, measurements, migration receipts, and evidence limits | [PERFORMANCE.md](PERFORMANCE.md) |
| The 0.4 comprehension study method, sample, outcomes, and evidence contract | [UNDERSTANDING_STUDY.md](UNDERSTANDING_STUDY.md) |
| Packaged agentic playtests and the persona-review archive | [PLAYTESTS.md](PLAYTESTS.md) |
| The playtester casting pool (the 42 personas with backstories) | [PLAYTESTERS.md](PLAYTESTERS.md) |
| The July 2026 external review and the mantra's origin | [REVIEW.md](REVIEW.md) |
| The composed review panel | [PANEL.md](PANEL.md) |
| Code-quality standards, versions, CI gates, prose locks | [ENGINEERING.md](ENGINEERING.md) |
| Scope discipline, the definition of no, the three-products hierarchy | [SCOPE.md](SCOPE.md) |
| Consequential decisions and their rationale | [decisions/](decisions/) |
| Research findings and sources | [RESEARCH.md](RESEARCH.md) |

## Conventions

- **House style:** no emojis, no em-dashes, no AI/tool attribution anywhere
  (CI-enforced, see [ENGINEERING.md](ENGINEERING.md) and
  [QUALITY.md](QUALITY.md)).
- **Link, do not duplicate.** If a second doc needs a concept, it links to the
  owner above.
- **Name docs with links,** not backticked file names, in the map and in prose,
  so every doc is reachable by a click.
- **Labels come from [RESEARCH.md](RESEARCH.md):** Built, Measured, Observed,
  Designed, Hypothesis. An owner doc opens with one status line, Built and
  Designed named separately, and the date it was last reviewed.
- **Packaged files link with full repository URLs** for anything a release
  archive does not carry, because nothing under `docs/` ships.
- **Counts and versions in prose are locked to live data.** A bare version
  number states the current release; name a past release by its tag, such as
  `v0.4.0-alpha.9`, or as "alpha 9". See [ENGINEERING.md](ENGINEERING.md).
- **Keep this map current.** A new doc is not done until it appears here with
  an owner row.
