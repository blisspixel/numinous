# Interfaces: The Three Faces

Numinous is not one program with a GUI bolted on. From the first commit it is a **headless core engine** with **three faces** over it: the native **App**, a full **CLI**, and an **MCP server** for AI agents. All three are first-class and built from the beginning, because designing for three faces forces a clean, headless, scriptable core, which makes everything else (testing, sharing, automation, agents) easy.

The frame that makes the whole thing coherent: **one experience, three sensoria.** The same core, the same math, the same beauty, delivered to three different kinds of user in three different contexts:

- a human with **eyes, ears, and hands** (the App),
- a human at a **keyboard in a terminal** (the CLI),
- a **mind that acts through tools and reads text** (the MCP server, i.e. an AI agent).

Each face has its own UX, deliberately designed for its user, not a lowest-common-denominator port. This doc specifies the UX we are going for in each.

**Implementation boundary, 2026-07-13:** all three faces are shipped from the
same headless core in 0.2.0-alpha.1. Descriptions below mix current behavior
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
  window. The app may add targeted GPU presentation through `numinous-gpu`.
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
  Input-aware legends cover rooms, all games, The Show, the Journey, and the
  Studio. The controller opens and closes all eight menu destinations; Studio
  formula entry remains honestly keyboard-required.
  Focus loss or disconnect cancels a held gesture. Touch, pen, MIDI, remapping,
  and platform hardware certification remain planned rather than implied.

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
- **Current command families:** `rooms`, `describe`, `render`, `gallery`, and
  `contact-sheet` cover the catalog and images; `tour`, `watch`, `play`, games,
  sims, and Journey commands cover live play; `plot`, `open-studio`, `sing`,
  `tune`, and `sonify` cover creation and audio. `bench` is the fixed game
  gauntlet, not the planned performance harness.

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
- **Current protocol surface:** `initialize`, `tools/list`, `tools/call`, and
  `ping` over stdio, advertising the tools capability. The 29 tools include
  `list_rooms`, `describe_room`, `play_room`, `listen_room`, `reveal_room`,
  `challenge`, `predict`, `list_sims`, `run_sim`, `plot_expression`,
  `sing_expression`, Journey operations, and the shared games. `PLAYING.md`
  carries the complete user-facing list.
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
- **Structured discovery (built):** `list_rooms`, `describe_room`,
  `reveal_room`, and `listen_room` return typed catalog, action, optional goal, revelation,
  deep-cut availability, ambient motif, stable ambient-bed summary, and bounded
  mathematical-sonification note data for all 351 rooms. `listen_room` names
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
  EXACTLY 4 LOBES`. `play_room` returns `goal`, `goalMet`, and a null reveal
  until accepted K=5 input closes four lobes. The earned response then includes
  the same reveal the App points to. Ambient phase alone cannot earn it.
- **Compatibility-preserving compact output (built):** every play-tool schema
  accepts `response_mode: "full" | "compact"`; the local broadcast consent
  control intentionally does not. The argument is stripped before domain
  dispatch, so it cannot change grading, replay, persistence, or effective
  values. Omitted and explicit `full` results are equal. Eligible catalog, room,
  listening, simulation, Quiz, Gauntlet, and trophy replies keep identical
  `structuredContent` while replacing only redundant text, and only when the
  replacement is shorter. Journey, scores, forget, Cairn, other unique-text
  results, text-only tools, and all errors retain their complete text. This
  keeps the [MCP 2025-06-18 tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
  requirement for a `content` block and aligns with its structured-result
  guidance without prematurely migrating to the breaking 2026-07-28 protocol
  candidate.
- **Structured interaction deltas (built):** when `pokes` or a `gesture` are supplied, `play_room` also returns a `delta` in `structuredContent`: the interacted frame diffed against the untouched frame at the same phase, size, and variation, as `cells_changed`, `ink_added`, `ink_removed`, `ink_reshaped`, `total_cells`, and the inclusive `changed_region` bounding box; the text render carries the same count as a `Touch:` line. This is the proof-of-touch half of the challenge/verify loop: the agent gets quantitative, optimizable feedback on how the math answered its hand.
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

### Local MCP session broadcast, producer built, viewer planned

The shared `numinous-broadcast` foundation implements the pairing,
compatibility, framing, consent, sequence, control-marker, typed public-event,
and bounded-queue contracts below. The MCP face now connects that foundation
through `broadcast_session`, a complete fail-closed policy for all 30 declared
tools, replay-safe daily seed normalization, and separate nonblocking writer
and disconnect-monitor workers. Twenty-three tools are explicitly public, six
progression or local-state tools are private, and the consent control broadcasts
neither itself nor progress. No human viewer surface ships in the current alpha.
The remaining work is the App Watch Agent listener and replay surface, followed
by the real subprocess acceptance proof.

A human should be able to open Numinous and watch a consenting digital player
explore through MCP, like a live Let's Play. This is an observation surface,
not surveillance and not duet control. The viewer reconstructs public rooms,
games, Studio output, actions, status, and state-independent results. Results
match the MCP guest except where Describe Room, Crack, SETI, or Quiz would reveal
private Journey level or boon choices; those four use a deterministic baseline
projection instead. The viewer never receives the guest's prompts, private
reasoning, host logs, client metadata, environment, or arbitrary JSON-RPC
traffic.

The first implementation has these hard boundaries:

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
envelopes capped before allocation, and the existing deterministic core to
reconstruct visuals and sound. Tests must
prove code parsing and expiry, loopback-only connection, consent-before-content,
allowlist completeness across every MCP tool, redaction, sequence and gap
behavior, reconnect refusal after capability use, nonblocking failure, exact
replay, and immediate stop. The acceptance session is
one consenting MCP guest completing a Times Tables explore, challenge, and
reveal loop while a human follows the same causal states in the App, with zero
private protocol or host data in the captured stream.

Dependency arrows are one-way: `numinous-core` never depends on the broadcast
crate; the broadcast crate consumes only core catalog metadata and never a face,
persistence, or raw MCP JSON-RPC; MCP now depends on the broadcast crate, and
the App will depend on it for the listener. The faces never depend on one
another. The CLI remains outside this first slice.

### Protocol watch: MCP 2026-07-28 release candidate

As of 2026-07-13, the official
[MCP 2026-07-28 release-candidate post](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
is roadmap-relevant but not an immediate blocker for the current stdio face. The
final specification is scheduled for July 28, 2026. The compatibility pass
should happen after the final target is selected, not during current
room-playability work.

Implications to preserve in the MCP design:
- Keep `play_room` replayable and explicit; the RC favors stateless protocol
  calls and visible application handles over hidden protocol sessions.
- Preserve stdio support unless and until a concrete host target requires the
  new HTTP transport shape.
- When a final migration is scheduled, check stateless requests, per-request
  `_meta` client information/capabilities, `server/discover`, `Mcp-Method` and
  `Mcp-Name` headers, cacheable `tools/list`, JSON Schema 2020-12, Tasks, MCP
  Apps, authorization hardening, and roots/sampling/logging deprecations.

### The MCP creative frontier (not just compatibility)

The 2026-07-28 RC is not only a migration chore; several of its features are a
direct invitation to push what an experiential MCP server can be, which is a
stated goal of this project. In priority of creative payoff:

- **MCP Apps (SEP-1865): ship the real room, not its shadow.** MCP Apps let a
  server hand the host a sandboxed HTML UI rendered in an iframe. This
  addresses the deepest limitation the text-only reviews kept finding: agents on
  structured-content clients see metadata and ASCII, never the glowing room. On
  a host that supports Apps, `play_room` (and the Studio, and The Show) can hand
  the agent the *actual* rendered, animated, sounding room, the same visual
  substance a human gets. The felt encounter (`VISION.md`, "the same wonder,
  two kinds of mind") stops being a text approximation. This is the single
  biggest creative opportunity and it must reuse the same safe render pipeline,
  never arbitrary agent HTML.
- **Multi round-trip elicitation (SEP-2322): predict-then-reveal, natively.**
  The keystone (`PEDAGOGY.md`) is pose, elicit a guess, reveal. The RC's
  elicitation without persistent streams is exactly that shape in one
  interaction, and it is the honest form of the duet relay and any mid-play
  choice. Today `predict` is two stateless calls; elicitation makes it one
  living moment.
- **Tasks: long watches and generative play.** The Show, a slow procedural
  generation, or a multi-turn game can run as a task the client drives with
  `tasks/get`/`tasks/update`, so a mind can lean back and watch the collection
  unfold rather than polling.
- **The Handle pattern: transparent world-state for co-presence.** Explicit,
  model-visible handles for shared session state fit the co-presence and
  multi-turn designs (`DIGITAL_MINDS.md`) without hidden server sessions,
  matching our stateless-and-replayable law.
- **We are already stateless.** The RC's largest architectural shift, removing
  the `initialize` handshake and session pinning, is something Numinous is built
  for: state lives in files, every tool call is self-contained. So the migration
  is small and the creative features are the real prize.

Testing note: the MCP face must be playtested against the LATEST build, never a
stale long-running server. `scripts/mcp-play.py` builds a fresh `numinous-mcp`
and drives it over stdio for exactly this (see `QUALITY.md`).

### Safety
MCP Studio input currently reaches a bounded expression language with no
filesystem, network, or raw GPU capability. The protocol and imported artifact
paths enforce size and shape limits. A community-room runtime is not shipped;
its future capability boundary is specified in `EXTENSIBILITY.md`.

### The payoff
The target is a **grounded playground and gym for mathematical intuition, for
any mind.** Whether interactive play produces a richer handle than reading must
be measured, not assumed. The agent's explore, challenge, and reveal arc rhymes
with the human Toy, Puzzle, and Revelation structure, while reproducible MCP
scripts exercise the same surface in local validation.

---

## Roadmap position

- **Built by 0.2.0-alpha.1:** the headless core, full-color CLI, native app, and
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
