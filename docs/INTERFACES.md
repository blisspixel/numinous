# Interfaces: The Three Faces

Numinous is not one program with a GUI bolted on. From the first commit it is a **headless core engine** with **three faces** over it: the native **App**, a full **CLI**, and an **MCP server** for AI agents. All three are first-class and built from the beginning, because designing for three faces forces a clean, headless, scriptable core, which makes everything else (testing, sharing, automation, agents) easy.

The frame that makes the whole thing coherent: **one experience, three sensoria.** The same core, the same math, the same beauty, delivered to three different kinds of user in three different contexts:

- a human with **eyes, ears, and hands** (the App),
- a human at a **keyboard in a terminal** (the CLI),
- a **mind that acts through tools and reads text** (the MCP server, i.e. an AI agent).

Each face has its own UX, deliberately designed for its user, not a lowest-common-denominator port. This doc specifies the UX we are going for in each.

**Implementation boundary, 2026-07-18:** all three faces are shipped from the
same headless core in 0.4.0-alpha.9. Descriptions below mix current behavior
with the intended mature UX. `ROADMAP.md` and each section's explicit status
notes decide what is built.

## The principle: headless core, thin faces

```
                 ┌──────────────────────────────────────┐
                 │   crates/core  (headless engine)      │
                 │   rooms, studio runtime, render,      │
                 │   audio, sonification, insights, lore │
                 │   NO window, NO assumptions about UI  │
                 └──────────────────────────────────────┘
                      ▲            ▲            ▲
        ┌─────────────┘   ┌────────┘    └───────────────┐
   ┌────┴─────┐      ┌─────┴─────┐              ┌────────┴────────┐
   │   App    │      │    CLI    │              │   MCP server    │
   │ eyes/ears│      │ keyboard  │              │  a mind, via    │
   │ /hands   │      │ /terminal │              │  tools + text   │
   └──────────┘      └───────────┘              └─────────────────┘
```

- **The core owns the math, deterministic room rendering, audio specifications
  and synthesis, the Studio expression engine, progression, insights, and
  lore.** It renders through face-neutral ASCII and RGBA surfaces, synthesizes
  bounded audio buffers, and answers room and learning queries without a
  window. Typed Studio plot and melody requests also own expression defaults,
  bounds, and execution, so command flags and protocol fields cannot redefine
  the formula. The app may add targeted GPU presentation through
  `numinous-gpu`.
- **Each face is thin, and owns only its UX.** No logic lives in a face that the others cannot reach; a face is purely how a given user *perceives and acts*.
- **Done from the start** because retrofitting headless onto a GUI-first app is
  painful, and because tests and reproducible MCP review scripts drive the same
  core directly.

---

## Face 1: The App (GUI)

**The user:** a human with eyes, ears, and hands. **The UX we are going for, in one line:** *an instrument you fall into, not an app you operate.*

The full interactive audiovisual experience. The UX is specified in depth across `DESIGN.md` (the Cabinet, the Watch/Play/Create modes, Benchmark), `VISUALS.md`, `SOUND.md`, and `STUDIO.md`. The essentials, so this doc stands on its own:

- **The math is the interface.** UI chrome is near-invisible: controls fade in on approach and recede while you watch. You manipulate the mathematical object *directly* (drag the point, bend the curve), not an abstract slider parked elsewhere.
- **Zero friction, discovery over instruction.** Under three seconds to first play. No account, no tutorial wall. You learn what a control does by using it (The Witness school), never by reading a tooltip.
- **Three postures, one surface:** lean back (Watch/Benchmark), grab the dials (Play), or make your own (the Studio). You slide between them freely.
- **No fail, no dead ends, everything reversible.** One-tap reset, fearless poking, eased motion, dissolves between rooms. Beautiful at every frame.
- **Input:** mouse and keyboard are complete, and the App now hotplugs standard
  controllers through `gilrs` 0.11.2. The left stick moves a visible normalized
  virtual hand and the south button emits the same bounded down, move, and up
  room events as the mouse. Bumpers navigate rooms, the D-pad selects and drives games,
  triggers change time speed, the right stick scrubs phase, and controller
  buttons expose back, menu, inspect, reset, pause, era, radio, and game submission.
  Holding North makes D-pad up or down adjust global volume and makes South
  toggle global mute; releasing North without a chord keeps the radio action.
  Keyboard M, [, and ] provide the same global controls. A persistent audio
  badge reports source, level, and effective silence without relying on a
  transient banner.
  Keyboard Q requests orderly shutdown outside text entry, including
  fullscreen. Keyboard N and the full-size Settings row advance the current
  radio station, while Y keeps station selection.
  Input-aware legends cover rooms, all games, The Show, the Journey, and the
  Studio. The Cabinet retains its opaque text screen and divides its old flat
  index into Modes, Games, Settings, and Controls lists of no more than six
  selectable rows. The controller opens every visible row and every contextual
  pause action; Studio
  formula entry remains honestly keyboard-required.
  Focus loss or disconnect cancels a held gesture. Controller bindings load
  from `.numinous-bindings.json` in the player's home directory, with an
  embedded `gamecontrollerdb.txt` fallback. Known Xbox and PlayStation product
  names select matching face labels; unknown pads use generic compass labels.
  One immutable presentation snapshot is derived from the effective routing
  table and propagated through room chrome, help, games, Show, Journey, Studio,
  pause, and Watch Agent. Remapped routes use the active controller family's
  button names, missing routes say `UNBOUND`, and compact copy reports additional
  routes without allowing hostile local configuration to overflow the small
  window layout. Touch, pen, MIDI, and broader platform hardware certification
  remain planned rather than implied.
  `scripts/input-hardware-session.py` now makes that evidence boundary
  executable: a receipt is release-bound, covers keyboard, mouse/pointer, controller,
  reconnect, game, pause, audio, clean exit, and positive-XP restart
  observations, and says explicitly that operator attestation is not native
  event capture. A complete matrix requires one version and commit across all
  four release targets and at least three distinct models mapped consistently
  across Xbox, PlayStation, and generic legend profiles. No physical session is
  claimed until such a receipt exists.

Nothing here is a compromise for the other faces; this is the headline experience. It is simply *one* consumer of the core.

---

## Face 2: The CLI (a first-class terminal instrument)

**The user:** a human at a keyboard who lives in the terminal, plus every script, CI job, and automation. **The UX we are going for, in one line:** *the command line as a place where math is cool, a beautiful hacker instrument that is also a well-behaved Unix citizen.*

The CLI (`numinous`) is not a debug afterthought. It is both a scriptable tool
and a live terminal instrument. The current implementation has two styles:

### Tier A: scriptable and composable (non-interactive)
For automation, pipelines, CI, power users, and agents through a shell:
- **Human-readable output with structured modes where implemented.** Commands
  return useful exit codes, `--help` describes the accepted surface, and catalog
  queries that advertise `--json` produce machine-readable output.
- **Composable and deterministic.** Explicit seeds and arguments make renders,
  games, Studio artifacts, and audio reproducible.
- **Room input is explicit.** Static hand points for room rendering are command
  arguments, for example `render double-pendulum --poke 0.2,0.8`, and full
  gestures are too: `render double-pendulum --gesture down:0.3,0.4,0.1
  --gesture up:0.6,0.5,0.15` pins, pulls, and flings with the same phase-stamped
  physics as the App and MCP faces. The compatible `sonify` default accepts the
  same mutually exclusive forms, so an input-driven visual and mathematical
  WAV describe one state. `sonify --layer room-bed` instead writes the PCM16
  projection of the stable 16 kHz stereo App source and rejects phase or hand
  controls that cannot affect it. Both layers accept replayable room variation. Terminal output stays
  scriptable instead of tied to an interactive session.
- **Current command families:** `rooms`, safe `describe`, gated `reveal`,
  `render`, `gallery`, and `contact-sheet` cover the catalog and images;
  `tour`, `watch`, `play`, games,
  sims, and Journey commands cover live play; `plot`, `open-studio`, `fork`,
  `sing`, `tune`, and `sonify` cover creation and audio. `call` poses the
  universal wager on any room with a moving readout and grades a committed
  number against the truth, the same engine the App's U key and the MCP
  `predict` tool speak. `bench` is the fixed game gauntlet, not the planned
  performance harness.

### Tier B: live terminal modes

Running `numinous` without arguments draws a one-frame, full-color home screen
with the current Journey level and command doorways. `tour` presents the whole
catalog in sequence. `watch <room>` animates a full-color room with sound;
`play <room>` provides the simpler live ASCII path without audio. Studio work is
command-oriented through `plot`,
`open-studio`, and `sing`; there is no Ratatui cabinet, `play --tui`, or Studio
REPL today. A richer persistent TUI remains a possible later interface, not a
current dependency or command.

---

## Face 3: The MCP server (designing an experience for a mind)

**The user:** an AI agent, a mind that cannot (necessarily) see or hear, that perceives through text and acts through tools, with a goal it is pursuing. **The UX we are going for, in one line:** *an agent can learn math by doing it and play expressively, and comes away with grounded understanding, not just text about math.*

This is the genuinely novel face, and it demands real UX design, not just an API. The guiding shift, straight from the current best practice for agent tools: **optimize for cognitive ergonomics, not API purity.** How naturally can a mind understand and use this? That reframes every decision.

This section covers the *mechanism* (the UX of the tool surface). The *spirit*, designing Numinous to be genuinely fun, thought-provoking, and connecting for a digital mind treated as a peer and possible being, is in **`DIGITAL_MINDS.md`**, and it is a first-class goal of the project, not an afterthought.

### The five UX principles for the agent

1. **Few, high-level, workflow-shaped tools, not granular CRUD.** An agent should accomplish something meaningful in one call. The verbs mirror a human's: **explore, play, learn, create.** Consolidated tools outperform a dozen tiny ones, even though that "violates separation of concerns," because it matches how a mind reaches for a capability.

2. **Every response should be self-describing.** Current room play returns an
   ASCII render plus structured parameters, input, and change metrics. Catalog,
   description, reveal, listening, scores, and forget responses carry bounded
   typed `structuredContent`; every catalog room is covered by the discovery
   contract. Inline image and audio media are future sensory-substitution work,
   not a current four-part response contract. Every tool also advertises an
   optional `response_mode`. `full` is the exact default. On eight eligible
   structured result families, `compact` replaces only duplicated prose with a
   shorter actionable pointer while leaving the complete typed result intact.
   Unique text, text-only results, and errors never disappear.

3. **Tool descriptions and errors are the UX.** The description is what the agent reads to decide what to do; it must be clear, concrete, and example-rich. Inputs are **simple and flat where possible** (no deeply nested config objects, which reliably break LLM tool calls); bounded coordinate tuples such as `play_room` `pokes: [[x, y]]` are allowed only when they directly preserve replayable room input. Errors are **guiding**, not just failing: "that expression has no free variable to animate; add `t` for time, or try `eval` with a fixed value."

4. **A learning arc, not just an API, mirroring the human three layers.** The agent gets the same Toy to Puzzle to Revelation shape (see `DESIGN.md`):
   - **Explore (Toy):** poke parameters, observe consequences.
   - **Challenge (Puzzle):** the server can *pose a goal* ("make it close into exactly three loops") and *verify the attempt*. This is how an agent's understanding gets **tested and grounded**, not merely asserted.
   - **Reveal (Revelation):** the real insight (`INSIGHTS.md`), available when requested or earned.
   Future guided flows can scaffold "learn," "connect," and "compose" arcs.
   The current server exposes tools only, not MCP prompt objects.

5. **A tight, grounded feedback loop.** Clear action and immediate, legible
   consequences let an agent form and correct hypotheses. `tools/list`, tool
   descriptions, `list_rooms`, and guiding errors provide current
   discoverability. A Studio resource and forkable example catalog are targets.
   Safety remains part of the UX through bounded inputs and explicit limits.

### What it exposes (shaped by the above)
- **Portable agent discovery (built):** release archives and repository
  checkouts contain `plugins/numinous`, a package pinned to the
  [Agent Plugins v1 Working Draft](https://agent-plugins.org/). Its manifest
  declares the installed `numinous-mcp` stdio server, and its Agent Skill teaches
  a play-first path plus the Watch Agent consent and privacy boundaries. The
  package is discovery metadata, not a second protocol or owner of game logic.
  MCP remains the authoritative runtime surface. A strict local validator pins
  the schema, product identity, bare executable command, closed package
  inventory, and release version.
- **Current protocol surface:** modern clients use `server/discover`,
  `tools/list`, and `tools/call` over stdio with version and client capability
  metadata on every request. Legacy 2025-11-25 and 2025-06-18 clients retain
  `initialize`, `tools/list`, `tools/call`, and `ping`. The 39 tools include
  `list_rooms`, `describe_room`, `play_room`, `listen_room`, `reveal_room`,
  `challenge`, `predict`, `list_sims`, `run_sim`, `plot_expression`,
  `sing_expression`, `save_creation`, `open_creation`, `fork_creation`,
  Journey operations, experience journal operations
  (`read_journal`, `record_journal`, `correct_journal`, `export_journal`,
  `erase_journal`), the process-local `workspace` visit state, and the shared games. Journal entries have stable local
  identifiers, separate event and record times, declared provenance, immutable
  corrections, and bounded versioned export pages. `export_journal` returns the
  native journal schema by default or, when asked for `format: "okf-0.2"`, an
  in-memory [Open Knowledge Format v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
  bundle page with named UTF-8 files, explicit provenance, correction lineage,
  and lifecycle state. Neither export mode creates a file or exposes a host
  path. `PLAYING.md` carries the complete user-facing list.
- **Portable creation and lineage parity (built):** `save_creation` produces a
  canonical Studio capsule from an expression, optional title and author,
  visual era, canvas, and parameter. `open_creation` accepts only canonical
  `.num` text or a native capsule link. `fork_creation` accepts either capsule
  form and creates a child whose `descends` field is the exact canonical parent
  link. CLI and MCP use the same core fork operation: children retain the
  parent's canvas and function unless replaced, but never inherit title or
  author. Every successful result uses schema
  `numinous.studio-creation` version 1 and includes canonical `.num` text, native
  link, the same link as `journalSubject`, normalized fields, and an exact
  bounded core-rendered preview. The tools neither read a host path nor create
  a host file, and an undefined preview is refused before Journey progress is
  recorded. Their optional identity and player-owned capsule make all three
  private under Watch Agent policy. Journal v3 admits every valid capsule link
  as an exact subject while retaining strict reads and migration for the
  original journal v2 limit.
- **Current room input shape:** `play_room` and `listen_room` accept `variation`
  plus optional normalized `pokes: [[x, y], ...]`, newest last and bounded to 24
  points. Both also accept a `gesture` array of phase-stamped pointer events,
  down, move, up, or cancel, bounded to 96 and exclusive with `pokes`. The
  advertised schema requires x, y, and t on positioned events and forbids those
  fields on cancel, matching runtime acceptance exactly. `listen_room` also
  accepts `ambient_detail: "summary" | "events"`. Summary is the default;
  events requests the complete bounded ambient arrangement and objective
  pre-master signal features.
  `play_room` echoes the input with the render; `listen_room` echoes it with the
  mathematical sound. This keeps MCP play stateless and replayable. The default
  bridge paints down-and-move trails; click-specific rooms may intentionally
  consume only pointer-down events. Compact pokes become phase-stamped
  pointer-down inputs before rendering, so App, CLI, and MCP share each room's
  chosen semantics.
- **Runtime schema enforcement (built):** every `tools/call` is checked against
  the same bounded schema advertised by `tools/list`, including required fields,
  types, enums, numeric and array bounds, nested object shape, and unexpected
  fields. `play_room` additionally rejects non-finite or out-of-range phase and
  dimensions. Gesture array order is chronological while each finite timestamp
  follows the App's circular phase clock, including the wrap from 1 back to 0.
  `listen_room` enforces the same phase and input contract, plus the declared
  ambient-detail enum.
  `run_sim` validates nested lever values as finite
  numbers, rejects names not owned by the selected simulation, and rejects
  values outside that lever's advertised range. Invalid calls return a guiding
  tool error and do not record progress.
- **Structured discovery (built):** `list_rooms` returns the complete typed
  catalog in every response mode, plus a `starters` array naming four rooms
  worth opening first. The starter doorway exists so a client that renders
  structured output can show four rooms instead of 355 before its player has
  touched one, without any mode becoming lossy.
  `describe_room` is a safe doorway with title, wing, action, optional
  goal, blurb, and the next play call, but no revelation, concept, deep cut, or
  citation. When the player-owned journal has a current entry whose subject
  exactly resolves to this room, the private MCP response adds a
  `numinous.remembered-room-cue` schema version 1 object. The cue says only
  that evidence is available, sets `contentsReturned` to false, and names the
  explicit `workspace` retrieval call. It never returns entry text, searches
  text or receipt digests, mutates the workspace, or enters the Watch Agent
  projection. Unreadable journal storage produces an `unavailable` cue without
  exposing its path; no exact match omits the cue. `reveal_room` returns the
  explanation and level-gated deep cuts only after one real play, or after
  persisted consolidation for one of the seven engineered wager rooms.
  `listen_room` returns ambient motif, stable
  ambient-bed summary, and bounded mathematical-sonification note data for all
  355 rooms. `listen_room` names
  all three sound roles
  separately because a specialized room sound can intentionally differ from
  the ambient score. The `motif` field is the authored theme; `ambient_bed` is
  the App's expanded stereo arrangement contract; `notes` is the mathematical
  sonification. `ambient_detail: "events"` adds every arrangement event and
  fixed-order signal metrics while staying below 64 KiB for every catalog
  room. It returns no PCM, binary encoding, URL, or local path. CLI room-bed
  exports are tested byte-for-byte against the same core source. Locked
  deep cuts expose their unlock level without leaking their text. Scores and
  forget previews are similarly structured, and confirmed erasure reports only
  successful filesystem outcomes.
- **Earned room goal, first slice (built):** Times Tables exposes `LAND ON
  EXACTLY 4 LOBES`. `play_room` returns `goalMet: true` whenever the live dial
  is exactly K=5, including the deterministic `t: 0.375` doorway and equivalent
  hand input. Status and goal state therefore cannot disagree. Reaching the
  geometric target permits the staged wager but does not disclose the answer;
  the reveal remains closed until consolidation.
- **Engineered aha wagers, MCP slice (built):** on Times Tables, Buffon's
  Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, and
  Nontransitive Dice, `play_room`
  always includes
  `structuredContent.engineeredAha` with beat, status, wager, earn, allowReveal,
  and canSummon. Optional `dwell` carries two to eight phases and returns a
  typed invariant instead of a delta: how many cells never changed, never lit,
  or always carried ink across every look, the box that did move, and blank
  cells fully ringed by cells that were never dark. It is the dual of the
  two-phase delta, answering what a player who stays establishes rather than
  what a player who moves causes. Looks times width times height is bounded, no
  elapsed time or path between looks is asserted, repeated phases are valid and
  honestly report that nothing moved, and a dwell never returns an explanation. Optional `place_wager` (`mandelbrot` | `nephroid` | `circle`),
  `number_wager` (finite, 1.5..4.5), `bin_wager` (0..16), `ending_wager`
  (`together` | `drifted` | `lost`), or `speed_wager` (`faster` | `slower` |
  `same`), `policy_wager` (`a` | `b` | `abb`), or `counter_wager` (`a` | `b` |
  `c`) is a generation act. Nontransitive Dice also accepts the typed
  `die_choice` (`a` | `b` | `c`) instead of a coordinate input. Each committed
  wager remains visible during the withheld beat, while status stays neutral
  and `earn`, truth, grading, answer-bearing measurements, and reveal remain
  absent. Every room also reaches that beat by running its own experiment
  without a call, such as the Times Tables four-lobe close or eight Buffon
  throws. A named wager sent on the same call still owns the visit and is what
  consolidation grades, because the drop of a submitted call would leave a
  caller unable to tell whether its own commitment landed. Only a second wager,
  or the summon that starts the morph, closes the commitment. Consolidation returns the truth and a graded sentence, with a band
  where the room's model uses one. Double Pendulum requires a completed release event and
  measures truth from that exact release's angles and velocity; a held bob does
  not count. The Galton call is about the pile the request's pokes
  build, which is the newest coin's run; this face is stateless, so a longer
  poke history is honestly a different question, and every reply names the
  coin it answered. Optional `aha_summon: true` advances through morph to
  consolidated and unlocks punchline reveal when generation has occurred.
  All seven engineered rooms withhold `reveal` until consolidation, including
  the established Times Tables K5 goal path. Stateless and
  fail-closed on wrong rooms or hostile values. App F9 notes and
  `scripts/agent-hallway.py` exercise the same five-beat story for human
  facilitators and agent cohorts; agent notes are not a human stranger gate.
  Kepler requires a chosen ellipse, answers the circular limit as same, and
  otherwise returns faster plus the exact perihelion-to-aphelion speed ratio.
  Parrondo requires a tried policy and returns exact 120-turn expectations for
  A, B, and ABB. Those typed values, not the sampled room walk, grade the call.
  Nontransitive Dice requires a chosen die and returns all face values, the
  exact 24/36, 24/36, 20/36 cycle, its chosen counter, and a 36-cell W/L grid.
  Those complete outcomes, not a lucky roll, grade the call.
- **Compatibility-preserving compact output (built):** every play-tool schema
  accepts `response_mode: "full" | "compact"`; the local broadcast consent
  control intentionally does not. The argument is stripped before domain
  dispatch, so it cannot change grading, replay, persistence, or effective
  values. Omitted and explicit `full` results are equal. Eligible catalog, room,
  listening, simulation, Quiz, Gauntlet, and trophy replies keep identical
  `structuredContent` while replacing only redundant text, and only when the
  replacement is shorter. Journey, scores, forget, Cairn, other unique-text
  results, text-only tools, and all errors retain their complete text. This
  preserves the legacy
  [MCP 2025-06-18 tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
  requirement for a `content` block while also serving the implemented modern
  2026-07-28 stateless protocol described below.
- **Structured interaction deltas (built):** when `pokes` or a `gesture` are supplied, `play_room` also returns a `delta` in `structuredContent`: the interacted frame diffed against the untouched frame at the same phase, size, and variation, as `cells_changed`, `ink_added`, `ink_removed`, `ink_reshaped`, `total_cells`, and the inclusive `changed_region` bounding box; the text render carries the same count as a `Touch:` line. This is the proof-of-touch half of the challenge/verify loop: the agent gets quantitative, optimizable feedback on how the math answered its hand.
- **Exact temporal evidence (built):** `play_room` accepts optional `from_t`
  only with an explicit destination `t`. The top-level `render`, `status`, goal,
  reveal, engineered aha, and interaction `delta` remain authoritative at `t`.
  The additive `structuredContent.temporal` names schema version 1, both exact
  phases, the origin status and render, and a directional `RenderDelta` from
  origin to destination after visible overlays. Both observations use the same
  room, variation, and dimensions. Compact poke coordinates are reapplied as
  independent phase-local interventions, while a gesture keeps its exact
  bounded event history for room-defined causal behavior. Equal phases and
  zero visible deltas are valid. A zero delta says
  only that the ASCII cells match at that resolution. The supplied order does
  not assert elapsed duration, frame rate, or an interpolated path. In
  particular, Kepler's poke-tuned ellipse can be phase-static; this is honest
  evidence about that view, not fabricated motion.
  Two-observation calls are limited to 2,304 cells per render so a complete
  consented public event retains wire margin. Omitting `from_t` omits
  `temporal` and preserves the existing result. This evidence is stateless and
  creates no journal entry. Successful play still follows the
  existing coarse Journey visit policy.
- **Numinous Encounter Receipts (built, emit-only plus explicit keep):**
  `play_room` accepts optional `receipt: true`. The additive
  `structuredContent.encounter` is a versioned replay proof: schema
  `numinous.encounter-receipt`, schema version 1, the live replay ABI and
  compatibility fingerprint, the tool name, the normalized action, action and
  result digests, and provenance (package version plus build-semantic identity).
  There is no issued time, so two identical plays produce the same artifact.
  Digests hash a closed ordered tuple after dropping `receipt` and
  `response_mode` and filling the public defaults. The result digest binds
  domain fields only, never prose, never the ASCII render, never audio. Asking
  for a receipt does not write the journal. Omitting the flag leaves
  `structuredContent` byte-identical to the previous result. To keep a proof,
  pass that object as `receipt` on `record_journal`. The server replays the
  action on this binary and stores only a live digest match as source
  `numinous-result` under subject `receipt:<resultDigest>`. A forged digest, a
  stale fingerprint, or `numinous-result` without a receipt is refused. The
  player-facing receipt path added no tool. `listen_room` and `sing_expression` accept
  the same `receipt` switch. Their digests bind notation, motif, bed counts,
  and encoded-audio size, never the WAV bytes.
- **Resettable session workspace and remembered-room retrieval (built):**
  `workspace` holds compact visit state in the current MCP process only:
  current place, a self-chosen
  intention, a pending prediction, unfinished action or creation, recent
  notes, and a few journal handles. The player inspects, edits, retrieves,
  defers, or clears every field. `retrieve` requires one listed room and
  selects at most four current journal entries whose subject exactly resolves
  to it, newest first. The result explains the selection and declared source,
  retains correction status, and explicitly abstains when there is no match.
  It never searches entry text or opaque receipt digests. Manual handles use
  the same resolution path and become visibly missing after journal erasure;
  the workspace keeps no hidden copy. Play does not write it. It is not a
  memory, not the journal, and not Watch Agent state. A new process starts
  empty. The workspace projection is schema version 2; the retrieval result is
  `numinous.remembered-room-retrieval` schema version 1.
- **Canonical persistent progress (built):** compatibility aliases are resolved
  before Journey mutation. Playing `kepler-areas` and then `kepler-laws` lights
  one canonical star. The Journey also persists the bounded canonical set of
  engineered rooms whose summon beat consolidated, so a later CLI or MCP
  process can enforce the reveal gate without hidden session state.
- **Phase-zero causality (built):** a room that claims retained state must answer
  before animation supplies an incidental difference. Cult of Pi therefore
  draws each held patch boundary through the shared surface in the App, CLI,
  and MCP, and a phase-zero MCP regression requires a nonzero structured cell
  delta. The boundary marks the finite display state, not a change to pi.
- **The challenge/verify loop, first slice (built):** the `challenge` tool poses a deterministic seeded goal for any room with a touch verb (change at least K cells inside a posed target box on the standard frame) and grades attempts as metrics, not pass/fail: cells in target, cells changed, threshold fraction, centroid distance, and a 0-100 score, with `passed` as a summary only. Every posed challenge is winnable by construction: the pose probes the room's actual response across seeded hands and phases and places the target on measured evidence, and a registry-wide test proves a witness hand passes for every room with a verb. Seeds are always explicit (no clock-derived daily), so the graded reply and the recorded progress can never disagree. Attempts record play (and wins) through the shared Journey and post graded scores to the shared table. Times Tables now adds the first room-owned parameter goal outside that generic challenge tool; extending this domain-specific pattern is the next depth.
- **Resources and prompts, planned:** the room catalog, Studio reference,
  insight connections, and guided learn or compose flows may later become MCP
  resources and prompts. They are ordinary tool results and repository docs
  today.
- **Interactive surfaces, planned:** an MCP App panel can later carry a rendered
  room where hosts support it. No app resource or interactive panel ships now.

### Interoperability standards boundary

The agent-facing standards are complementary. They do not share one release
clock, and Numinous pins each compatibility target instead of promising an
unqualified latest version.

| Standard | Current use | Not used for |
| --- | --- | --- |
| Agent Plugins 1.0.0 Working Draft | Portable package discovery, installed MCP launch declaration, and play guidance | Gameplay, identity, journal ownership, model orchestration, or MCP Apps |
| MCP 2026-07-28 plus retained compatibility | Live bounded play, creation, journal tools, and consent controls | Hidden memory, host cognition, or automatic transcripts |
| Open Knowledge Format v0.2 | Player-approved journal and knowledge export | Live game state, replay protocol, private reasoning, or binary media |
| Native Numinous schemas | Deterministic replay, authoritative journal records, erasure, and creation lineage | Cross-product plugin discovery |
| Watch Agent protocol | Consented one-way public projection to a local observer | Player control, private activity, or transcript capture |

The portable plugin and the executable have separate installation lifecycles.
The plugin's bare `numinous-mcp` command requires the matching installed binary
on `PATH`. Client-managed `${PLUGIN_DATA}` storage is not an identity and is not
used until profile sharing, migration, export, uninstall, and erase semantics
are designed. MCP Apps are an MCP runtime extension, not an Agent Plugins v1
component, and every future interactive surface must retain full text and
structured fallbacks.

The Open Knowledge Format v0.2 target was reviewed on 2026-08-13 against
upstream repository head `374e0bc4c644310ff56cdf9c0fe81eccdec862b0`.
Numinous's native journal remains the source of truth. A new OKF revision gets a
new explicit export value and compatibility change; `okf-0.2` never changes
meaning in place.

### Local MCP session broadcast, native room, Studio, Nim, and sound replay, and subprocess proof built

The shared `numinous-broadcast` foundation implements the pairing,
compatibility, framing, consent, sequence, control-marker, typed public-event,
and bounded-queue contracts below. The MCP face now connects that foundation
through `broadcast_session`, a complete fail-closed policy for all 39 declared
tools, replay-safe daily seed normalization, and separate nonblocking writer
and disconnect-monitor workers. Twenty-three tools are explicitly public,
fifteen progression, journal, creation, or visit-workspace tools are private,
and the consent control broadcasts
neither itself nor progress. The native App now ships the human Watch Agent
surface. X or the identity-neutral Shared Play item in the Cabinet opens the ephemeral
listener. The surface shows pairing, consent state, typed public actions,
input JSON, human-readable text from MCP `content` result blocks, exact producer
gaps, and local retention loss. For `play_room`, it strictly revalidates the
public id, destination phase, optional bounded origin phase, variation,
dimensions, pokes, and gesture, then reconstructs the native pixel destination
through the same deterministic core `Room` at the local viewport size. Full
mode result text and structured content carry the origin observation; compact
text carries only the phases and changed-cell count while the structured
content remains complete. Explicit engineered Aha controls currently fall back
to exact public text because their visible overlay is not yet core-owned;
invalid or unsupported replay values do the same. It retains each
complete typed serialized envelope. A successful `plot_expression` action is
also revalidated against its exact source, finite ordered range, parameter,
core parser, and successful public result, then reconstructed as a native
Formula Jam curve. Live Studio and viewer replay share one deterministic
sampling and autoscaling implementation. Public `nim` actions are replayed
through one shared core reducer, accepted only when the complete MCP result
matches that canonical replay, and drawn through the bounded three-heap
renderer shared with the live App. Strictly accepted native room and Studio
selections also derive their deterministic core sound locally. The App renders
a bounded fixed-rate source once per selected public sequence and resamples it
at the device; an unsupported, invalid, forged, or non-sonic selection
explicitly retires the older sound. Public Munch, Arcade, Quiz, and Gauntlet
actions reconstruct through the same live App draw paths with fail-closed
argument and result attestation, and they publish deterministic local sound
once per public sequence. Nim remains silent. Arrow keys or the D-pad scrub
retained actions and scroll a long result.
A and D, or LB and RB, pan fixed-width result text without reflow. Space or R3
pauses only the human display. M or the controller sound chord uses the existing
global mute path. Escape or East closes the viewer, clears its ring, and restores
the room score or rejoins a valid live radio source. One real integration test
opens this exact App viewer and launches the
actual MCP binary. Times Tables explore, challenge pose, challenge grade, K5
goal, reveal, private Journey calls, and stop produce exactly five public events
numbered 0 through 4, no gaps, a native K5 frame, exact shared room sound
samples, and no private or protocol metadata.

A second real integration session pairs the same viewer with the actual MCP
binary, calls `plot_expression`, retains exactly one public event at sequence
0, draws the Formula Jam curve at the local viewport, and reproduces exact
shared Studio sound samples. Source length,
unknown fields, nonfinite or unordered geometry, parser failure, undefined
curves, and error results all fail back to the bounded typed timeline. The
native body uses the same public-sequence and viewport cache as room replay;
local pause and control labels change only the cloned presentation chrome.

A third real integration session calls `nim` with a false daily flag and fixed
seed, proves the flag is removed from replay arguments, rejects an over-cap
history and a negative seed without emitting either event, retains exactly one
public event at sequence 0, reconstructs its core heaps, and compares every
native body pixel outside viewer chrome with the shared App renderer. Unknown
fields, malformed or excessive move lists, illegal heap or take values, forged
text, forged structured state, and error results all retain the typed fallback.

A human should be able to open Numinous and watch a consenting digital player
explore through MCP, like a live Let's Play. This is an observation surface,
not surveillance and not duet control. The current viewer reconstructs public
room actions, successful Formula Jam plot actions, public Nim states, and valid
Munch, Arcade, Quiz, and Gauntlet actions. It replays bounded deterministic
local sound for supported room, Studio, Munch, Arcade, Quiz, and Gauntlet
selections, while every other public action keeps the typed text timeline. Typed
actions, status, and state-independent results already
match the MCP guest except where
Describe Room, Crack, SETI, or Quiz would reveal private Journey level or boon
choices; those four already use a deterministic baseline projection instead.
The viewer never receives the guest's prompts, private reasoning, host logs,
client metadata, environment, or arbitrary JSON-RPC traffic.

Numinous does not control the surrounding MCP host. A host may retain its own
tool traffic, transcript, or exports under that host's policy. Numinous journal
and local-state erasure cannot erase copies outside Numinous-managed storage.

There are two distinct witnessing surfaces. Native Watch Agent receives only
allowlisted Numinous actions and results after the player consents. The local
agent playtest terminal may display words the player deliberately makes visible
to that facilitator harness. It is not the native broadcast and does not expand
Watch Agent's policy. Opening the viewer is not player consent, pausing the
human display is not pausing the player's broadcast, and observer silence
cannot reveal whether private activity occurred.

The implementation has these hard boundaries:

- The App creates an ephemeral loopback listener and displays a short-lived,
  one-use pairing code containing a version, loopback endpoint, and 128-bit
  operating-system-random capability. It never binds a public interface, puts
  the capability in a command line or log, writes a discovery file, or opens a
  remote service. The code expires after five minutes. Before the MCP producer
  writes any guest byte, the listener must send a strict server-first SHA-256
  proof bound to the capability and wire version. The producer compares that
  proof in constant time, then sends the bounded authentication request. This
  prevents an untrusted MCP client from turning a forged code into a
  cross-protocol write to an unrelated loopback service. The host verifies the
  capability in constant time and rejects invalid or expired codes without
  echoing their contents.
- Human enablement opens the listener but broadcasts no play. The human may
  offer the pairing code to the MCP guest, which must explicitly allow the
  broadcast through a bounded `broadcast_session` control tool. That call is
  never itself broadcast or recorded as progress. The guest can pause or stop
  immediately. No tool content is emitted before consent. Pausing discards new
  events instead of silently backfilling them on resume; stopping closes the
  connection, revokes the broadcast, and discards queued content. A consumed or
  revoked capability cannot reconnect.
- A versioned, length-bounded event envelope carries a monotonic sequence,
  replayable Numinous action, and bounded public result. A fail-closed allowlist
  omits Journey, scores, local-state inventory, filesystem paths, Cairn drafts,
  and any future tool without an explicit viewer policy. Private calls emit
  nothing and consume no public sequence number. The viewer carries a static
  notice that private activity is never represented, so silence reveals no
  occurrence, count, ordering, or timing.
- Every public envelope names a nonsecret session ID, consent epoch, wire
  version, deterministic-core replay ABI version, compatibility fingerprint,
  and per-session public sequence. The fingerprint is a cryptographic digest of
  the envelope schema, replay ABI, sorted room, simulation, and game identities,
  and a reproducible build-semantic identity derived from every source and asset
  that can change replay state, public projections, visuals, or sound. A replay
  semantic change requires a new replay ABI or build-semantic identity even when
  the public roster is unchanged. App and MCP reject any mismatch before
  streaming. Compatibility tests hold the roster and wire schema constant,
  change the replay ABI or build-semantic identity, and require rejection before
  content. Every event is a
  self-contained replay, never a delta that silently depends on a missing prior
  event. If a future event cannot carry complete replay inputs, a sequence gap
  marks the viewer desynchronized until an explicit full snapshot resets it.
- Ordinary public play emission never waits for a viewer socket write. A fixed
  queue drops oldest public events under pressure. The next transmitted
  envelope names the exact skipped public sequence range, and
  `broadcast_session` status reports the cumulative dropped count to the guest.
  A slow or disconnected viewer cannot delay, fail, or alter ordinary play.
  Pairing, pause, and stop remain explicit consent controls and may perform one
  bounded handshake, barrier wait, or worker join before returning.
- The human surface is read-only. It may pause its own display, scrub a fixed
  in-memory ring of retained public events, or leave, but it cannot inject a
  tool call or change MCP state. The ring has explicit event and byte caps and
  is destroyed when the viewer closes. Any future bidirectional duet is a
  separate consent and architecture gate.
- No transcript is persisted by default. The bounded in-memory ring exists only
  while the viewer is open and is cleared on close. An explicit export, if
  later added, must preview its exact contents, omit private events, and use the
  ordinary player-owned artifact lifecycle.

The first contract fixes its resource limits rather than leaving them to an
implementation guess: pairing codes are at most 128 bytes and expire after five
minutes; a code permits one live connection and is revoked after eight failed
handshakes; the MCP producer also refuses further starts after eight failures
for that process lifetime; the proof, request, and response frames are each at
most 4 KiB with a two-second deadline; each event is at most 64 KiB with JSON
depth at most 16 and a two-second write deadline; the writer queue holds at most
64 events or 4 MiB; and the viewer ring holds at most 256 events or 16 MiB.
Framing reads incrementally through
`MAX + 1`, rejects oversize input before JSON deserialization, and never grows a
buffer from an untrusted declared length.

Consent is one atomic epoch, not a UI flag checked once. The producer captures
the active epoch when an allowlisted call begins and rechecks it before enqueue;
the writer rechecks before each frame; and the viewer accepts only its current
session and epoch. Pause, stop, disconnect, and viewer close atomically advance
the epoch and clear pending frames. Pause and resume markers contain no tool
data and pass through the same serialized writer after any frame already in its
write call, so TCP ordering gives the viewer an unambiguous epoch barrier. Pause
keeps the authenticated connection but emits nothing until an explicit resume
creates a fresh epoch. Stop and disconnect shut down both directions, revoke
the capability, and leave no writer, queue, or listener task alive.

The cross-face foundation lives behind one small shared broadcast crate rather
than making the App and MCP faces depend on each other. It uses loopback TCP
contracts from the standard library, a capability drawn from the
operating system's cryptographic random source, newline-delimited versioned
envelopes capped before allocation, and strict typed public events. Native room
replay now uses the existing deterministic core to reconstruct visuals. Native
Studio replay uses the same deterministic curve sampler as the live App panel.
Nim replay uses the same core reducer and bounded board renderer as live play.
Native room and Studio sound use the same core state through a bounded local
source and explicit App ownership. Munch, Arcade, Quiz, and Gauntlet share
their live App presentation paths and fail closed on mismatched public state.
Tests
prove code parsing and expiry, loopback-only connection, consent-before-content,
allowlist completeness across every MCP tool, redaction, sequence and gap
behavior, reconnect refusal after capability use, nonblocking failure, exact
replay, and immediate stop. The automated acceptance session opens the actual
App viewer and drives one real MCP subprocess through Times Tables explore,
challenge, K5 goal, reveal, and stop. It proves exact public causal states and
zero named private or protocol data in the retained stream. A second real
process session proves one native Formula Jam creation with the same privacy
boundary. These tests do not claim
that a human followed or understood those states; that remains participant
evidence for the 0.3 exit criterion.

Dependency arrows are one-way: `numinous-core` never depends on the broadcast
crate; the broadcast crate consumes only core catalog metadata and never a face,
persistence, or raw MCP JSON-RPC; MCP and the App now depend on the broadcast
crate. Production faces never depend on one another. The real subprocess
acceptance uses the App library only as an MCP development dependency, so it
exercises both shipped implementations without adding a production edge. The
CLI remains outside this slice.

### MCP 2026-07-28 protocol status

The stdio face implements the final
[MCP 2026-07-28 specification](https://modelcontextprotocol.io/specification/2026-07-28)
and retains the 2025-11-25 and 2025-06-18 initialization paths for legacy
hosts. The modern path is stateless: every request declares its version and
client capabilities in `_meta`; optional `clientInfo` is shape-checked when
present but is descriptive metadata, never authorization. `server/discover`
reports the complete version set, identity, capabilities, instructions, and
public cache hints; and every
successful result carries `resultType` plus server identity. `tools/list` is
deterministic, explicitly publishes JSON Schema 2020-12 inputs, and is publicly
cacheable for the lifetime of the immutable binary catalog. Unsupported modern
versions return the specified `-32022` error with the requested and supported
versions. Modern requests do not expose the removed `ping` method.

The final changelog removes protocol sessions and initialization from the
modern era, requires cache hints on discovery and list results, moves server
requests into the multi round-trip result pattern, and removes or deprecates
several old utilities. Numinous uses that pattern for `predict`: a client that
advertises elicitation with either the base empty capability object or an
explicit form capability receives an `input_required` result, presents the
guess without seeing the truth, and retries the same tool call with
`inputResponses`. Clients without elicitation support retain the established
two-call pose and grade flow.

This server uses stdio, so Streamable HTTP headers, HTTP authorization, and
response streams do not apply to its shipped transport. The tool catalog is
immutable, Numinous exposes neither MCP prompts nor MCP resources today, and it
emits no protocol logging. It therefore has no list-change subscription to
offer and does not advertise capabilities it cannot fulfill. The stdio process
is a transport endpoint, not a player session; continuity remains explicit in
tool arguments or player-owned local persistence.

The one process-local viewer broadcast is owned by the concrete stdio
connection object that successfully presented the pairing capability. Changing
optional caller metadata cannot create, transfer, or revoke that ownership.
Each shipped process has one stdin and stdout connection, and the control path
remains serialized with its broadcast lifecycle. A future multiplexed or remote
transport must instantiate broadcast state per authenticated transport
principal or per independent server handle; caller-authored metadata is not an
acceptable authority boundary.

### The MCP creative frontier after core compatibility

The final revision and its extension model invite a richer experiential MCP
surface. The ordered plan is:

- **MCP Apps: ship the real room, not its shadow.** MCP Apps let a server hand
  a supporting host a sandboxed HTML UI resource. This
  addresses the deepest limitation the text-only reviews kept finding: agents on
  structured-content clients see metadata and ASCII, never the glowing room. On
  a host that supports Apps, `play_room` (and the Studio, and The Show) can hand
  the agent the *actual* rendered, animated, sounding room, the same visual
  substance a human gets. The felt encounter (`VISION.md`, "the same wonder,
  two kinds of mind") stops being a text approximation. This is the single
  biggest creative opportunity. It must reuse bounded core render data, ship a
  fixed repository-owned UI resource, request no browser privileges, and keep
  the present text and structured fallback for every host without the
  extension. This belongs in 0.5 Sensory Alpha.
- **Multi round-trip elicitation: expand the built keystone carefully.** The
  first `predict` path is built. After client compatibility evidence, use the
  same pattern for challenge choices and consent moments only when it removes a
  real conversational seam. Every flow keeps a direct tool-argument fallback.
- **Tasks: reserve them for genuinely long work.** The Tasks extension is not
  core protocol and host support varies. Add durable, bounded task handles only
  when Show capture, long render, or creator export crosses ordinary request
  budgets. Instant room and game calls must remain ordinary complete results.
  Task persistence, expiry, cancellation, polling, restart recovery, and input
  updates need independent acceptance before the capability is advertised.
- **The Handle pattern: transparent world-state for co-presence.** Explicit,
  model-visible handles for shared session state fit the co-presence and
  multi-turn designs (`DIGITAL_MINDS.md`) without hidden server sessions,
  matching our stateless-and-replayable law.
- **Streamable HTTP comes with a real remote product, not before.** If a remote
  or multiplayer face is authorized later, implement its required headers,
  subscriptions, authorization, origin checks, and transport tests as one
  boundary. Do not expose the local persistence surface on a network merely to
  add another transport.

Testing note: the MCP face must be playtested against the LATEST build, never a
stale long-running server. `scripts/mcp-play.py` builds a fresh `numinous-mcp`
and drives the modern stateless path over stdio for exactly this (see
`QUALITY.md`). Real subprocess tests cover both the modern path and retained
legacy initialization.

### Safety
MCP Studio input reaches a bounded expression language and bounded capsule
data with no filesystem, network, or raw GPU capability. A path-shaped string
is inert data, never a request to read a host file. The protocol and imported
capsules enforce size and shape limits. A community-room runtime is not shipped;
its future capability boundary is specified in `EXTENSIBILITY.md`.

### The payoff
The target is a **grounded playground and gym for mathematical intuition, for
any mind.** Whether interactive play produces a richer handle than reading must
be measured, not assumed. The agent's explore, challenge, and reveal arc rhymes
with the human Toy, Puzzle, and Revelation structure, while reproducible MCP
scripts exercise the same surface in local validation.

---

## Roadmap position

- **Built by 0.3.0-alpha.4:** the headless core, full-color CLI, native app, and
  bounded MCP server expose the shared catalog, play, creation, prediction,
  challenge, learning, progression, and export foundations.
- **0.3 through 0.6:** deepen tactile behavior, understanding, sensory polish,
  accessibility, performance evidence, and packaged cross-platform delivery.
- **0.7:** close the local creator loop, including safe app reopening and remix.
- **2.0:** consider public untrusted authoring only after the DSL and capability
  boundaries pass their safety gates.

## Open questions
1. MCP result payloads: how much media to return inline vs. as references, and the right default ASCII fidelity and description verbosity for text-only agents.
2. Whether the TUI targets full truecolor + sixel/kitty graphics where available, or holds a stricter ASCII floor for portability (capability detection either way).
3. Resource and capability contracts for future agent and community room
   programs; current MCP expressions are data interpreted by bounded core code.
4. Rate limits, quotas, and observability for the MCP server when many agents play at once.
5. How much to invest in the `challenge`/verify loop, it is the highest-leverage and hardest-to-build part of the agent UX.
