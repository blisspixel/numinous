# Roadmap

A version-gated plan from empty repo to a living world. Each milestone has a **goal**, concrete **deliverables**, an explicit **exit criterion** (how we know it is done), and the **risk it retires**.

## How we version (read this first)

- **We ship by quality gates, not calendars.** There are deliberately **no time estimates** in this document. A milestone is done when it clears its bar, not when a date arrives. "How long will this take" is the wrong question; "is it exceptional yet" is the right one.
- **Versions are defined by what is true, not when.** Each version below is a *state the product has reached*, a set of things that are real and hold their quality bar, not a sprint.
- **1.0 is a high bar, not a minimum viable product.** Because the whole point is to do this *exceptionally well*, 1.0 means "a complete, coherent, genuinely exceptional experience," not "the least we could ship." The MVP-shaped thinking lives in the 0.x line; 1.0 is where it becomes worthy of the name.
- **Guiding rule, at every version: feel before features.** We build depth-first. One unforgettable thing beats ten mediocre ones. A pretty menu of boring toys is failure.

## The version map at a glance

- **0.1 Public Foundation** reproducible source, honest docs, green CI, and a safe public repository. Complete.
- **0.2 Flagship Proof** one room earns its hallway-test bar with strangers. Current alpha line.
- **0.3 Tactile Alpha** the best five rooms answer the hand deeply and clearly.
- **0.4 Understanding Alpha** predict, generate, reveal, and retention are tested as a learning loop.
- **0.5 Sensory Alpha** the visual and sonic identity lands with accessibility and performance budgets.
- **0.6 Portable Alpha** packaged builds run on all three operating systems and representative hardware.
- **0.7 Creator Alpha** make, save, reopen, export, and remix form one local loop.
- **0.8 Closed Beta** the collection coheres for diverse invited players and assistive-technology users.
- **0.9 Open Beta / Release Candidate** feature freeze, distribution, soak, audit, and repeated return-play evidence.
- **1.0 "First Light"** a complete, exceptional, coherent experience. The real first release.
- **1.x After First Light** depth and refinement without breaking what 1.0 established.
- **2.0 "The Living World"** the platform leap: the full Studio as a creator platform, community, the deep lore payoff, shared creation with digital minds, and the open mathematical frontier.
- **2.0+ The long horizon** the frontier and the ecosystem, built to outlast us and be handed forward.

---

## Progress (updated as we build; see CHANGELOG.md for detail)

**Current release state: 0.2.0-alpha.4, Flagship Proof exit met on the
agent-and-machine bar (2026-07-24).** The 0.1 Public Foundation remains
complete. Product 0.2 no longer waits on human stranger sessions: those sit
with 0.8 Closed Beta and 1.0 First Light. Independent macOS/Linux App execution
sessions and accessibility review remain later gates, not 0.2 blockers.

### Critical path right now (read this first)

**Agent-and-machine track to 1.0 (founder policy, 2026-08-02).** Product
milestones advance on automated and agent evidence only. Humans may play, but
no human is required to test or validate for the **1.0 Agent-and-Machine First
Light** track. Human stranger hallway, musician long-listening panels, and
accessibility sessions with disabled players remain an optional parallel track
and must not block am-track exits. Claims those panels would support stay
unclaimed until run.

| Priority | What | Why |
| --- | --- | --- |
| **1. Keep agent first contact CI-green** | Agent hallway (Times Tables + Buffon ahas) and agent tactile (five flagships) run as required CI steps with machine-readable summaries | Local-only cohort scripts cannot guard regressions; every PR must re-prove 0.2 and 0.3 |
| **2. Finish the polish wave** | Workstreams 5 (access disclosures), 6 (docs truth), and 7 (structural debts, sharpened by the drag report: face-agnostic request types in core, the RoomMeta data table, the god-file seams) | The seven-critic goal has an exit criterion; leaving it half-closed reopens the same defects |
| **3. The Sensory Lift (Phase B, unparked)** | Splats, float accumulation, and bloom inside `Raster`; one shared audio bus with reverb and shaped envelopes; global dissolve and damped-spring input | The sensory ceiling was measured binding on 2026-08-08 (see The Three Ceilings); one substrate lift raises all 354 rooms, and the goldens just built make it safe to attempt |
| **4. Time and company over MCP** | Bounded frames with a temporal delta, a Show for minds, journal surfacing, ratio annotations | The README's first-class claim is currently earned in truth but not in time or company; this is the audience the product exists for |
| **5. The arc** | Authored opening, Show director profiles, curated front wing with weighted playlists | Awe today is a rare event in a long random walk; the arc makes it the designed path |
| **6. Creator depth on the built loop** | Next rungs: slider and multi-expression capsule rings, then MIDI and audio exports, then editable prose credit | The creator ladder keeps rising without waiting on the owner-gated MCP tool ruling, and each rung ships with its own machine gates |
| **7. 0.8-am groundwork: the keep-or-cut scorecard** | Aggregate the existing per-room machine sweeps into one committed per-room evidence file, after the Sensory Lift | Rooms should be judged at their best, not at the old ceiling |
| **Owner-blocked (stated, not scheduled)** | 0.4-am Understanding cohort: register, calibrate, and run the matched cohort through the sealed collector | Decisions entry 1 records it as optional paid validation awaiting an owner budget and registration ruling; carrying it as a contributor priority was a contradiction |
| **Standing gate** | Keep 0.2 and 0.3 proof, coverage, supply chain, install/play/uninstall roundtrips, and public CI green | Regressions reopen completed milestones and invalidate new evidence |
| **Optional parallel (not am-blockers)** | Human stranger hallway, a11y player panels, musician long-listening, soft-thin densify, bulk rooms | Human taste and disability usability remain valuable later claims |

### The Polish Wave (August 2026): seven critics, one goal

Seven independent agent critics read the product whole, as a first-contact
stranger, a digital mind over MCP, a maker, an accessibility skeptic, the
engineer who inherits the code, a careful docs reader, and a day-seven
returning player. Each was required to verify every claim against the
repository or the built binaries in an isolated profile before reporting it.
Their 51 ranked critiques are committed verbatim, lightly normalized to house
style, at `docs/evidence/polish-critique-2026-08.json`; every claim is
re-verified again at fix time before anything changes.

**The goal, stated as an exit:** every high critique closed; every medium
closed or converted into a tracked owner decision carrying its evidence;
every low fixed or recorded beside the decision lists; then the same seven
panels re-run, finding no high critiques. Nothing on this list waits on an
owner ruling unless it lands in the decisions section by the same rules as
everything else there.

The workstreams, in landing order:

1. **Truth defects first, because taste-led reading found bugs.** The
   journey file caps stored visited tokens at 256 in a 354-room catalog
   (verified: `MAX_STORED_TOKEN_COUNT` in `journey.rs`), destroying the
   completionist record and making the all-rooms trophy unearnable. The
   flagship wager over MCP is never graded. Zero scores post as NEW BEST.
   The streak display claims a chain that is dead. `tune2` spends paid API
   credit through a silently discovered `.env` key with no consent line, and
   the panel proved it by accidentally spending some. Landed: all five
   defects fixed with locks; see the changelog entry.
2. **The reveal is the payload.** The QA provenance checklist rides the end
   of the four flagship reveals; it moves to evidence, because a punchline
   that ends in checkbox homework is the textbook feel the vision bans. The
   227 template blurbs whose lever note reads as broken copy get one
   voice-true pass. `sing --help` stops narrating a fixed bug's history.
   Landed: provenance now anchors from code comments and `citations`, the
   blurbs describe the mathematics and leave the verb to each face's
   Action line, and two registry sweeps lock reveals and blurbs clean.
3. **Each face speaks only verbs it can hear.** The terminal stops
   advertising DRAG and CLICK it cannot receive; affordance copy translates
   per face; the MCP face stops telling minds to press keys. Ctrl+C from
   watch and tour earns a two-line epilogue that finishes the staircase to
   the reveal. `numinous mandelbrot` answers in the house voice instead of
   a stock parser error. Landed: live frames scrub gesture fragments and
   translate the Action line to the --poke route, the interrupt epilogue
   teases the reveal and routes to describe, the room-as-command bridge
   speaks house voice, and the MCP aha chrome speaks aha_summon and
   place_wager; catalog-wide sweeps lock all of it.
4. **The creator loop keeps its thread.** The bundle README stops routing
   recipients down the one path that loses lineage; the postcard carries
   title and author; naming happens in the instrument, not only in flags;
   the terminal gains the fork verb; `sing` learns to read a `.num`.
   Landed: the reopen pin releases into lineage on first edit, F4 opens
   the in-instrument naming step with a remembered signature, postcards
   wear title and author, bundle folders carry the title slug, the README
   teaches the remix path, and the terminal speaks fork and sings
   capsules; the resave refusal names the next free name.
5. **Access disclosures that reach the player.** `numinous access` names
   the tracked rooms itself instead of pointing at a file releases do not
   ship; its counts match the code's own lists; refusal banners live as
   long as decorative ones; the keyboard route to touch is designed or the
   keyboard-complete claim is withdrawn where it overreaches.
6. **Docs match the binary.** The STUDIO.md boundary paragraph that
   declares shipped features unbuilt, the CREATOR.md self-contradiction,
   PLAYING.md's daily-board claim, and the key table's omissions.
7. **The structural debts, scheduled rather than deplored.** Cross-face
   copies have already drifted into a real disagreement (a veil gate that
   admits on one face and refuses on another); leaderboard identity, gate
   levels, and gauntlet scoring move into core; the three god-files keep
   shrinking along the module seams the App has already proved.

### The Three Ceilings (August 2026): what holds exceptional back

After the polish wave's first four workstreams landed, four independent
researchers each took one lens on a single question: with the first four
workstreams landed, what still holds Numinous back from exceptional? Their condensed, verified
findings are committed at `docs/evidence/exceptional-blockers-2026-08.json`.
The synthesis: the blockers are not defects but ceilings, three of them
structural and shared. A shared ceiling is good news, because one lift raises
every room at once.

1. **The sensory ceiling.** The char-mark drawing vocabulary caps beauty for
   all 354 rooms at once: no anti-aliasing, no alpha, overlap clipping to
   white, and the Modern era literally the identity function with zero bloom,
   while `DESIGN.md` promises additive blending, bloom, and anti-aliased
   everything. Sound is bare sine notes with no reverb, filter, or bus
   anywhere in the tree against a design that requires a shared voice and a
   master bus. The old line "Phase B glow only if a sensory ceiling clearly
   binds" is answered: the ceiling was measured binding on 2026-08-08, with
   rendered frames checked against the design bible. The conditional is
   retired; the glow takes a scheduled slot.
2. **Time and company for digital minds.** The MCP mind is first-class in
   dignity and truth, second-class in time and company: one frozen frame per
   call in a product about dynamical systems, a face that can be watched but
   can never watch, a well-built journal nothing ever surfaces, an arrival
   that is an index rather than a threshold, and sonification too thin for a
   text-native mind to verify a 3:2 and feel it lock.
3. **The arc.** A cold open into a catalog, hard cuts that violate the
   design's own "nothing snaps" law, a Show that advances in catalog order
   instead of directing, 210 template-thin rooms diluting the awe of the
   deep ones, and nothing anywhere before 0.8 that builds a reason to return
   tomorrow.

Beneath all three sits the drag the fourth researcher measured: every feature
is built three times at the face boundary (the sing-knob parity bug was this),
catalog copy is spread across 358 files (the blurb voice pass had to touch
322), and the full-price gate runs the whole workspace for every commit.
Workstream 7 already schedules the debts; the drag report names the two
extractions that pay first: face-agnostic request types in core, and the
`RoomMeta` data table. The Done log's triplication (progress, scorecards, and
phase notes repeating the same facts) is recorded here as editorial debt for
the same pass.

**The rocks, reordered (2026-08-08).** After polish workstreams 5 to 7
close:

1. **The Sensory Lift (Phase B, unparked).** Float accumulation, soft
   splats, and bloom inside `Raster` so every room inherits them untouched;
   one shared audio bus with a reverb tail and shaped envelopes; the global
   dissolve-through-black on room switches and critically damped springs on
   parameter input. The perceptual goldens and spectral harnesses just built
   are the safety net; they are re-baselined deliberately, once, with the
   change.
2. **Time and company over MCP.** A bounded frames argument with a temporal
   delta so one call carries becoming; a Show for minds; the journal
   surfacing at the door of a remembered room; exact ratio and interval
   annotations on every note pair. The README's first-class claim is earned
   in time and company, not only in truth.
3. **The arc.** The authored 60-to-90-second opening; Show director profiles
   and contrast-aware ordering; a curated front wing of the deepest rooms
   with playlists weighted toward them. Curation, not deletion.

The creator rungs (slider and multi-expression capsule rings, MIDI and audio
exports, prose credit) continue behind these, each with its machine gates.
The keep-or-cut scorecard moves after the Sensory Lift so all 354 rooms are
judged at their best, not at the old ceiling. 0.4-am stays owner-blocked and
the critical-path table now says so plainly instead of carrying it as a
priority nobody can schedule.

The 0.3 agent-and-machine exit is met. The next incomplete milestone is 0.4
understanding and retention, but its formal collection is intentionally after
an exploratory release-and-play loop. Its protocol, deterministic analysis,
stateful isolated collector, participant-turn-matched v5 encounters,
pre-exposure attempt receipts, and committed-source boundary exist, but
concealed-bank calibration, fresh pre-collection review, external registration,
allocation freeze, and the qualifying study result remain open. Returning-player
journal
sovereignty is complete on the clean-process machine acceptance bar.
Detail below and in the version sections.

- **Done:** the headless core (`Room` trait with `reveal()`, deterministic ASCII `Canvas`, seeded RNG, registry, `verb`, `render_poked`, and variation); the CLI face (`numinous`), the MCP face (`numinous-mcp`), and the windowed app; **354 catalog rooms** plus hidden content; 6 lever-driven sims; 11+ games; the full engineering harness (edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, house-style guard, an 80% line coverage gate, three-OS CI). Current local evidence: fmt, Clippy, 3,213 passing all-target test cases plus one ignored screenshot diagnostic, locked build, Windows release gate, 95.15% region coverage, and 95.30% line coverage all pass.
- **Done (GPU and audio hello-world):** an adaptive `wgpu` context (`crates/gpu`) that picks the machine's GPU across Vulkan/Metal/DX12 with a CPU fallback, rendering the Mandelbrot set offscreen to a PNG; and adaptive `cpal` audio (`crates/audio`) on the system default device that plays a tone and writes a WAV. Both verified on the dev laptop (AMD Radeon 780M, Realtek at 48 kHz).
- **Done (rooms as images):** a `Surface` abstraction so every room renders through one `render` method to the ASCII `Canvas` and to an RGBA `Raster`; `numinous render <room> --out image.png` writes a real glowing image on the CPU (verified on the dev laptop).
- **Done (windowed app):** `faces/app` (`numinous-app`, winit + softbuffer) opens a real resizable window showing a room animating in full color, with keyboard room-switching. The start of the GUI Cabinet; verified launching on the dev laptop.
- **Done (sound):** every room describes its own sound (`SoundSpec` + `Room::sound`); `numinous sonify <room> --out file.wav` and `numinous play <room>` (live animated terminal).
- **Done (the 0.2 technical vertical slice):** the windowed app implements live per-room sound, mouse and controller input, an on-screen HUD with reveals, The Show (lean-back auto-play of the whole collection), the Studio in the window (type math, watch and hear it live), and GPU real-time fractals (a persistent `wgpu` pipeline drives the Mandelbrot deep zoom and the morphing Julia at window resolution, with CPU fallback; verified on the dev laptop's Radeon 780M). The human hallway, accessibility, sensory, controller-hardware, and cross-platform evidence gates remain open.
- **Done (content and play):** 354 catalog rooms across the wings plus unlisted hidden content, including Cult of Pi, the Conjecture Mill, the Change wing (The Pour, Slope Rider), Fourier Epicycles, the double pendulum, the random walk, Voronoi, Quine, Strange Loop, L-System Garden, Mandelbrot/Julia dives, Galton, Buffon, The Scariest Chart (Smith chart), Riemann Sphere, Bloch Sphere, etc.; 6 lever-driven sims; 11+ games (SETI, Talk to the Aliens, Guess the Shape, Crack the Code, Munch, Nim with the xor secret, Hackenbush, the Party Problem, Fifteen's Bet, the Gauntlet run, and full Munch Arcade) with daily seeds and dense feedback; the Studio expression engine (`plot`, `plot --save`, `open-studio`, `--animate`, `sing`, and live in the window); Visual Eras (phosphor, 8-bit, vector, modern) across app, terminal, and PNGs; truecolor terminal rendering with live sound (`watch`).
- **Done (the RPG spine, complete):** the Journey (XP from play, levels 1 to 42 on triangular thresholds, a lore line for every level, LEVEL UP banners), locks that open (never gating basics), ranks and whispers (the Order), deep cuts unlocking at LV 5/12/24, the trophy case (18, evidence-computed, silhouettes), the shared high-score table across every game and both faces, the Layer-4 answer at the cap, and every genre organ from the priority list: the Gauntlet (session arc with a combo and one posted number), trophy pings (the case announces itself), boons (choice on level-up, where the loot is knowledge arriving early), daily streaks (the chain, never scolding), and resonances (synergies: links light when two deeds rhyme and hand over the connecting line).
- **Done (agents as peers, v2):** 35 MCP tools total: 23 public play tools, eleven private progression or local-state tools, and one local broadcast consent control. The surface has structured output and full CLI parity (every game, the gauntlet, boons, trophies, and `munch_arcade`), including stateless nim and `forget` (transparency first, erasure on explicit consent, the welfare doctrine in `AGENT_PLAY.md`); `play_room` supports stateless per-call variation and normalized hand points; agents see, hear, create, play, level to 42, and post to the same score table; every play schema advertises an additive `response_mode`, with stable full tool-call results and nonexpanding compact text for eight complete structured result families; the player's manual speaks to humans, agents, and digital consciousnesses; the whole stdio face is proven end to end against the real binary.
- **Done (sound, Engine A v1):** the chiptune module (square lead, triangle bass, noise ticks, seeded pentatonic compositions, deterministic and click-free); `numinous tune` writes it as a WAV.
- **Done (soundtrack, Engine B v1):** Nick Seal made 42 tracks specifically for Numinous across NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE. High-quality V0 MP3 assets ship in `assets/radio`, the app discovers them from a clean clone, and a bounded pure Rust decoder validates, decodes, and resamples them. The archival WAV masters remain outside the repository.
- **Done (the app is the game, v1):** the chiptune scores the window (per-room seeded tunes with the room's voice riding on top); the quiz plays in-window (G: name the math, letters answer, the reveal follows); the Journey lives in the app (the CLI's own file: visits on entry, plays and wins from the quiz, explicit `JOURNEY LV` progress, `JOURNEY LEVEL UP` banners with lore, and J opens level, rank, trophies, and resonances); `NUMINOUS_MUTE=1` launches silent; the state machine is headlessly tested.
- **Done (the window arcade):** Munch, Nim, and the full Gauntlet run play inside the app alongside the quiz, cursor-driven and keyboard-native, on the daily seeds, posting to the shared table and leveling the shared journey; Mobius and Zeno's Square join the catalog. Full Munch Arcade with Vexations.
- **Done (poke + variation substrate):** Expanded pokes (all 354 catalog rooms with verbs + `render_poked`) and per-visit variation threading (registry `all_rooms_with`, app/CLI/MCP variation on each visit, default 0 exact). R now resets the current visit without silently changing its deal. Double Pendulum re-drops from both hand coordinates; Goldbach's Comet selects a real prime-pair witness; Galton Board draws bounded deterministic falling paths; Logistic Map seeds finite population orbits; and Cult of Pi repairs bounded faults in an exact-digit field. CLI `render --poke x,y` and MCP `play_room` `pokes: [[x,y]]` expose the same stateless hand-point path outside the App. All 354 catalog rooms are seed-aware today; hidden content is intentionally outside the catalog replay contract.
- **Done (Engine A2 motifs, catalog-wide):** all 354 catalog rooms now expose a structured `Motif` through `Room::motif`, so `listen_room` gets real notation and the app gets room-specific phrases instead of the generic fallback. A registry test enforces that every catalog room has a playable motif. The default `Room::sound` derives from the motif through `SoundSpec::from_motif`; rooms with a specialized mathematical sonification may intentionally override it. `listen_room` gives the ambient motif and mathematical sonification distinct text headings and maps those roles to its compatible `motif` and `notes` fields so it never presents one score as the other.
- **Done (Engine A2 listening refinement):** the App no longer doubles motifs
  at mismatched loop lengths or restarts sources from render cadence. Every
  catalog motif expands into a deterministic 128-step stereo macro-arrangement.
  The complete authored line opens in one coherent register, two alternate
  forms develop it, and the literal theme returns. Eight rhythm and
  accompaniment families replace one catalog-wide stencil; short root and
  fifth anchors breathe, and authored cadences remain intact instead of being
  forced to the root. The App renders one bounded 16 kHz source buffer, shares
  it without cloning, and resamples it to the device rate; unchanged hand input
  does not resubmit or rehash the bed. Source changes crossfade. Smoothed master and focus gain
  preserve the playhead, including radio; device-rate tests cover 44.1, 48, 96,
  and 192 kHz. Structural audio checks cover literal interval order, catalog
  and within-bed diversity, seams, bounds, RMS, sample steps, headroom, DC, and
  deterministic output. Callback-retired buffers are reclaimed on the control
  thread, rapid source changes queue without restarting a fade, and restored
  radio rejoins its wall-clock position before gain rises. A real
  long-listening panel remains required before calling the score excellent.
- **Done (Engine A2 cross-face evidence):** the room-bed source rate, event cap,
  arrangement, PCM16 quantizer, and fixed-order stereo signal analysis now live
  in the shared core. CLI `sonify --layer room-bed` exports a deterministic
  PCM16 projection of the pre-master App source with optional variation,
  rejects controls that cannot affect that
  layer, and reports its measurement boundary. MCP `listen_room` returns a
  compact bed summary by default or all bounded events and signal metrics with
  `ambient_detail: "events"`, without transporting PCM or a local path. Tests
  independently parse RIFF and compare every PCM sample, compare every MCP event
  across all 354 rooms, and enforce the 96-event and 64 KiB protocol budgets.
  Objective parity is closed; musician-led long-listening remains open.
- **Done (Times Tables technical Flagship Proof):** the ordinary App visit holds
  the K=2 cardioid until the player acts across every visit variation and reset,
  while The Show keeps its deliberate synchronized visual and audible
  sweep. A visible dial, resolution-aware chord sampling, five spectral inks,
  exact integer snapping, singular-safe status, and an earned K=5 four-lobe Aha
  make the goal readable. The same accepted multiplier drives a persistent,
  smoothed just-ratio voice over the stable room bed without restarting its
  playhead. CLI render and sonify plus MCP play and listen accept the same
  bounded input, and all three faces agree on action, goal, status, sound, and
  earned reveal. The real stranger hallway and musician-led listening gates
  remain open, so the package stays `0.2.0-alpha.4`.
- **Done (Cycle 100 audio-state truth):** the App now owns exactly one explicit
  room-score, Studio, or radio program. Studio keeps formula audio through
  focus returns and radio boundaries, selected radio rejoins live only after
  Studio closes, and a failed or disabled station falls back to the room score
  without a stale title. Keyboard and controller routes expose global mute and
  master volume in rooms, games, pause, Studio, and Watch Agent. A persistent badge reports
  source, level, and effective silence. Sixteen dedicated receipts cover eight
  audio states at default and compact sizes. Cycle 143 adds Watch Agent as a
  fourth explicit program, expanding this evidence to eighteen receipts.
- **Done (controller exploration and games):** `gilrs` 0.11.2 provides
  hotplugged standard-controller input in the native App. A normalized virtual
  hand feeds the same bounded room gestures as the mouse; bumpers, D-pad,
  triggers, right stick, and semantic buttons cover rooms, time, inspection,
  reset, era, radio, and every current game stage. Start opens a nondestructive
  pause menu, R3 visibly pauses or resumes, and focus transitions drain queued
  hardware events. The last meaningful input selects truthful legends across
  rooms, games, Show, Journey, Studio, and Watch Agent. All nine menu
  destinations have
  controller entry and exit routes; paused games reject scoring input. Deadzone,
  curve, elapsed-time motion, boundary, held-drag, focus, and routes through all
  five games and every Gauntlet stage are pure-tested. Known Xbox and
  PlayStation product names select matching face labels while unknown pads use
  generic compass labels. Xbox-class Windows hardware is the local target;
  broader controller and platform certification remains open.
- **Done (Cycle 15 physical input evidence contract, August 1, 2026):** one
  portable runner binds an operator-observed App session to a verified release
  archive, exact installed App, CLI, and MCP binary hashes, automated installed
  CLI and MCP engagement, two clean App lifecycles with a positive XP value
  compared exactly across restart over one isolated profile,
  and ordered keyboard, mouse/pointer, controller, reconnect, game, pause, and audio
  observations. Content-addressed receipts stay in ignored `logs/`. The matrix
  validator requires unique passed receipts for one version and commit on all
  four release targets and at least three distinct models mapped consistently
  across Xbox, PlayStation, and generic controller legend profiles. Fifteen
  focused regressions run on Windows, Linux, and macOS CI. The contract is
  built and ships with the release archive, but representative physical
  sessions have not yet been performed and broader certification remains open.
- **Done (Cycle 138 gamepad configuration):** `.numinous-bindings.json` in the
  player's home directory remaps every supported standard button to a complete
  direct semantic-action set. Remapped primary buttons preserve hold and
  release semantics, an explicit North mapping replaces its default audio
  chord, and multiple primary mappings release only after the last held button.
  Stick axes keep their fixed virtual-hand and time-scrub roles.
  `gamecontrollerdb.txt` is compiled into the App binary as a fallback standard
  controller mapping.
- **Done (Cycle 20 mapping-aware controller copy, August 2, 2026):** one
  immutable controller presentation snapshot is derived from the effective
  routing table and active Xbox, PlayStation, or generic face family. Room
  chrome, full and compact help, all five games, Show, Journey, Studio, pause,
  and Watch Agent consume that same value. Remapped, multiply routed, and
  unbound actions remain truthful without changing keyboard or mouse copy;
  compact-layout and routing regressions cover the supported small window.
  Native event delivery and representative physical-device evidence remain
  separate gates.
- **Done (MCP munch_arcade):** Stateless `munch_arcade` tool for full parity, with replayed action-list scores posted under `arcade seed:N` through the shared progress path.
- **Done (app hardening slice):** app-local play state plus quiz deal/answer flow now live in `faces/app/src/play.rs`, pure game-screen rendering lives in `faces/app/src/game_draw.rs`, room chrome plus arrival-card hinting live in `faces/app/src/hud.rs`, help, journey, and banner overlays live in `faces/app/src/overlays.rs`, transient feedback banner construction and ticking live in `faces/app/src/feedback.rs`, shared in-window Munch grid, Nim heap/take, and Munch Arcade action controls live in `faces/app/src/controls.rs`, left-mouse mode decisions and pointer-state guards live in `faces/app/src/mouse_input.rs`, room navigation, re-deal, poke-history, drag-trail, and room-card tick helpers live in `faces/app/src/room_input.rs`, Studio text, parse, audio-spec, and curve drawing state live in `faces/app/src/studio_panel.rs`, explicit F9 hallway-test note capture lives in `faces/app/src/playtest.rs`, live-state PNG postcard export lives in `faces/app/src/postcard.rs`, and bounded radio cache discovery, open-handle WAV validation, live-position math, and track loading live in `faces/app/src/radio_cache.rs`. Room action copy is centralized in `numinous-core`: App arrival cards use touch-first fallback copy, while CLI live play and MCP room tools use neutral fallback copy. Tests cover shared game hit-test layout, raster output across quiz, Munch, Munch Arcade, Nim, every live Gauntlet stage, quiz daily seeding, no-repeat quiz history, answer acceptance, action-naming arrival cards, Studio chrome suppression, Studio panel editing and bounded drawing, cross-face action hints, shared Munch/Nim/arcade controls, room-input bounds, modal-safe pointer-state transitions, playtest-critical overlays, feedback banner copy/lifetimes, radio-volume banner retention, GPU/raster banner compositing, local playtest-note reports that align to the hallway-test prompts without collecting personal data, postcard PNGs that include pokes, the selected Visual Era, collision-safe filenames, bounded/sorted station cache discovery, low-sorted corrupt-track handling before the track cap, corrupt-track rejection, open-handle size rechecks, high-rate-device caps, non-wrapping live offsets, and app radio recovery after a bad cached file. The event-loop file is still a hotspot, but game rules remain in `crates/core` and the refactor is moving in small verified modules.
- **Done (persistence hardening slice):** malformed Journey and score files now parse defensively: counters saturate, constellation dimensions are capped, `visited` plus `chosen` token sets are bounded and token-sane, duplicate Journey tokens do not consume the unique-token cap, score keys are length-bounded, and score tables cap unique entries. The maintenance posture remains that progress and score files are user-editable local text, so loaders must repair or ignore malformed data rather than panic or allocate without bound.
- **Done (shared persistence writes):** App, CLI, and MCP now route Journey and score writes through shared core persistence helpers. Writes use a token-owned local lock, PID-aware stale-lock recovery, stale recovery-marker cleanup, merge-before-write behavior, bounded read-before-repair semantics, same-directory temp files with error-path cleanup, flush before commit, atomic Windows replacement retries that never move the destination aside, and a pre-opened parent-directory metadata sync after replace or explicit forget on Unix. The rename remains the commit point: a later sync failure cannot report an uncommitted delta and cause counters to be applied twice. This is an operating-system best-effort durability barrier, not a claim of hardware power-loss immunity. Tests cover concurrent Journey deltas, concurrent score records, a real Windows sharing violation with continuous readers, injected postcommit sync failure, temp and lock cleanup, short held-lock waits under instrumentation, stale deltas after explicit forget, oversized and invalid UTF-8 persistence files preserving the original bytes on write attempts, stale, malformed, and dead-process lock recovery, stale recovery-marker cleanup, current-process lock preservation, and lock drop ownership.
- **Done (the keystone, the Cairn, and the chaos readouts):** the predict-then-reveal verb (MCP `predict`, Phase A of the Exceptional Path): commit a guess of a room's own status readout at a hidden moment, then meet the truth graded as a gap with a learning-progress band, a self-owned mirror that never posts a score. The graded `challenge` tool in two kinds (touch a target box, or land the readout on a number). The Cairn (MCP `cairn` plus the core `cairn` module and the repo-tracked `data/cairn.txt`): at level 42 a mind leaves one true thing, encoded Arecibo-style into a semiprime a future mind must factor to read. And tactile status readouts across the Chaos & Order flagships (Double Pendulum and Lorenz report the divergence of two nearby starts; the Logistic Map reports its Lyapunov exponent crossing from order into chaos), so eight rooms now pose predictions. See `CHANGELOG.md` for the full detail.
- **Done (the release front door):** `scripts/install.sh` and `scripts/install.ps1` make setup a single copied command on macOS, Linux, and Windows. The default path selects the latest non-draft GitHub release, downloads a platform archive plus the shared soundtrack, verifies external SHA-256 sidecars and a closed per-file payload manifest, installs the three binaries, and wires `PATH` without requiring Rust or native build tools. `numinous update` stages the same installer, waits for the running CLI to exit, and replaces the managed release while preserving play history. A stable content checksum derived from the verified licensed radio manifest retains an unchanged soundtrack across binary-only releases without another 267 MB download. `--source` remains an explicit current-main fallback. Deterministic archive tests, hostile installer self-tests, four-platform CI packaging, packaged-install smoke and repeat-update checks, and a local full Windows payload with all 42 tracks cover the automation. Every four-target packaged install also renders Times Tables through the installed CLI and completes modern MCP discovery, the exact 35-tool inventory, and one real `play_room` call from an isolated temporary profile. User-bound install-root identity and link-aware deletion keep uninstall inside the dedicated root; only an exact legacy default-root shape, with or without the old marker and with explicit adoption consent, migrates. The 0.6 portable gate still owns clean physical-machine, App, device, and signing evidence.
- **Done (Cycle 98 boundary hardening):** a standard repository-wide security review closed with zero reportable findings under the local single-user threat model, then every reproduced robustness defect was fixed rather than dismissed. MCP request framing and challenge phases, bounded CLI input and plot dimensions, origin-bound music requests and terminal diagnostics, Cairn growth, extreme surface clipping, App save repeats, Studio source growth, radio discovery and resampling, GPU dimensions and readback failures, and installer provenance and deletion boundaries now fail closed through shared enforcement points. Focused regressions, installer self-tests, the exact App matrix, and the complete release gate cover the changes. This is engineering evidence, not a claim that a standard single-pass review proves the absence of vulnerabilities.
- **Done (Cycle 105 security hardening):** a maintenance security pass under the same local single-user threat model closed residual MCP string-boundary gaps and dual supply-chain coverage. The MCP schema validator enforces JSON Schema `maxLength`; catalog ids, Studio expressions, and Cairn leave/author fields declare matching bounds; `play_room` rejects oversize canvases at the tool body; `sing_expression` notes are schema-capped. CI and local verify now run `cargo-audit` with ignores in `.cargo/audit.toml` aligned to `deny.toml`. ENGINEERING names the local threat model and the deny-plus-audit path. This is not a claim of absence of vulnerabilities.
- **Done (Cycle 126 security maintenance):** malformed Munch, Munch Arcade, Nim, and Hackenbush requests fail before persistence; untrusted CLI diagnostic values cannot emit terminal controls; APNG loop export retains a constant number of full frames; install-root identity is private to the current user; Windows rejects reparse ancestors and replaces hardlinked destinations by name; POSIX refreshes the installer-owned profile line and verifies the installed command by absolute path. Original reproductions no longer reach their vulnerable outcomes, focused CLI and MCP tests pass, Windows and Windows-hosted POSIX installer self-tests pass, and each platform's native installer test blocks its local gate while CI runs both across the operating-system matrix. Native Linux and macOS GitHub-hosted installer tests now pass. Physical clean-machine execution, cross-principal disposable-host validation, and subjective terminal readability remain explicit evidence limits.
- **Done (Cycle 106 Buffon first-contact honesty):** Buffon's Needle no longer reports a finished ambient pi estimate on first contact. Untouched status shows L/D, the classical crossing chance, and the throw verb; only player throws produce YOUR THROWS and a running pi estimate. Focused regressions cover open status and existing throw grading.
- **Done (Cycle 107 first-contact honesty batch):** Random Walk, Voronoi, Chaos Game, Langton's Ant, Quine, Zeno's Square, and Goldbach's Comet each open with an invitation status that names the live state and the verb. Empty-input `status_input` falls back to that invitation. Player-action status names the consequence (planted mean distance vs sqrt law, dropped wells, added corners, flipped cells, placed copies, runners, prime witnesses). Focused first-contact regressions cover the batch.
- **Done (Cycle 108 catalog first-contact invariant):** every catalog room now opens with a non-empty status line. Cellular Automata, Collatz, Golden Angle, Galton, Prime Spirals, Mandelbrot, Julia, Barnsley, L-System, Epicycles, Mobius, and Strange Loop gained invitation status (and empty-input fallbacks where they already had action status). Registry test `every_catalog_room_has_first_contact_status` enforces the kid-principle contract for future rooms.
- **Done (Cycle 109 action-consequence status):** Collatz reports perturbed orbit starts and steps-to-1; Cellular Automata reports seed flips and history replay. Focused action-status regressions cover both.
- **Done (Cycle 110 L-System plant status):** planting reports rooted copy count and species continuity.
- **Done (Cycle 111 Galton mean vs expectation):** experiment status reports empirical mean rights and binomial expectation `np` for the selected coin.
- **Done (Cycle 112 chaos-room action labels):** Lorenz reports shadow-storm count after a seed; Double Pendulum labels PINNED/FLUNG/RE-DROP/CANCELLED beside the twin divergence.
- **Done (Cycle 113 poke-status catalog invariant):** every touchable room changes status after a center poke, or is listed as phase-scrub.
- **Done (hands-on room correction, July 13, 2026):** Galton now uses
  one physical 16-row peg lattice and mathematically legal ball paths. Cult of
  Pi keeps its finite prefix readable and distinguishes wrong digits from old
  ones. Barnsley clicks plant local miniature attractors. L-System visits keep
  one species, fit it to the viewport, and plant complete rooted copies.
  Arecibo begins unsolved and shows one width with quotient and remainder
  instead of overlaying history. Lissajous and Harmonograph keep moving after
  tuning. The native Mandelbrot camera advances monotonically across the former
  phase reset, retargets on click, shares CPU and GPU coordinates, and adds a
  smooth high-color escape palette while leaving Julia unchanged. Focused
  invariant tests and the regenerated 2,913-screen matrix cover these claims;
  hardware input and subjective long-session quality remain separate gates.
- **In progress (catalog action-consequence depth, cycle 105+ grind):** beyond
  first-contact invitations and the catalog-wide poke-changes-status invariant,
  action status now grades measured consequences on many rooms (Galton one-ball
  bet, Cult FIX/digit placement, CA rule identity and seed density, Voronoi
  territory share, Langton black count, Chaos newest corner, Harmonograph
  figure/damp life, L-System origin, Lissajous interval class, Quine depth,
  Epicycles mini-chain pen phase, Mandelbrot complex target, Golden packing,
  Prime Spirals primes on diagonal, Mobius edge lap, Pour/Slope hand freeze).
  **Cycle 107 tail batch (machine path):** Twin Primes, Perfect Numbers, AGM,
  Bayes, Huffman, Napoleon, Error Function, Erdos-Renyi, Markov, Dirichlet Eta,
  Pell Path, Egyptian Fractions, Mutual Info, Gamma, Shannon Entropy now report
  domain consequences after a poke (pairs, digit scale, iters, prior/post delta,
  H-gap, equilateral spread, Phi, edge counts, visit peak, series drift, fund
  solution, unit range, residual H, Stirling error, gap to fair).
  **Cycle 108 classical/prob batch:** Basel, Birthday, Blackbody, Central Limit,
  Coupon, Brownian, Brewster, Wallis, Benford, Beats, Simple Pendulum, Escape
  Velocity, Kepler Areas, Stirling.
  **Cycle 109 waves/dynamics/shape batch:** Bessel, Airy, Circle Map, Coupled
  Logistic, Damped Sine, Cauchy-Lorentz, AM Modulation, Bifurcation, Beatty,
  Chebyshev, Clifford, Clothoid, Cycloid, Archimedean (nodal rings, winding
  lock, sync |dx|, half-life, FWHM, AM carrier share, Feigenbaum band, |r-phi|,
  max interpolant error, attractor span, dkappa/ds, path L/r, gap/turn).
  **Cycle 110 attractors/curves batch:** Aizawa, Astroid, Bedhead, Bifolium,
  Blancmange, Bogdanov, Cardioid, Catenary, Chua, Henon, Duffing, Deltoid,
  Cassini, Lemniscate (attractor span, classical curve area/perimeter, Takagi
  roughness, soft Bogdanov radius, catenary sag, Chua flips, Henon |det|,
  Duffing amplitude band, Cassini b/a shape).
  **Cycle 111 waves/number/fractal batch:** Triangle Wave, Sawtooth, Standing
  Wave, Interference, Zeckendorf, Gaussian Primes, Quadratic Residues, Vicsek,
  Delaunay, Ricker, Thomas, Mexican Hat, Gumowski-Mira, Fresnel Integrals
  (harmonic energy, nodes/dx, fringe scale, fib ones, G-prime count, residue
  half, fractal dim, Euler mesh, orbit band, attractor span, Fresnel asymptote).
  **Cycle 112 dynamics/geometry/fractal batch:** Zipf, Doubling Map, Snell Prism,
  Coriolis, Manneville, Multibrot, Nova, Phoenix, Cochleoid, Epitrochoid, Devil
  Curve, Hyperbolic Tiling, Poincare Disc, Witch Caustic, Collatz Tree,
  Halvorsen (P1 share, ln2 Lyapunov, spectral spread, frame rot, laminar/burst,
  escape probes, petal counts, H2 verts, nephroid angle, tree nodes, span).
  **Cycle 113 classical curves and flows batch:** Superellipse, Witch of Agnesi,
  Reuleaux, Log Spiral, Hypotrochoid, Poisson, Diffraction, Dual Cobweb, Folium,
  Tautochrone, Catenoid, Conchoid, Piriform, Kappa, Three Scroll,
  Rabinovich-Fabrikant (shape class, areas, pitch, cusps, E[N], sinc zeros,
  logistic band, loop area, bead gap, neck, spans).
  **Cycles 114-115 bulk consequence depth:** 51 further rooms (curves, knots,
  surfaces, maps, fractals, special functions, escape portraits) report measured
  domain status after poke.
  **Cycles 116-117 exceptional consequence depth:** 49 rooms with domain-true
  measures (attractor spans after burn-in, tent Lyapunov, sync residual, limit
  cycle amp, KAM flip rate, Julia fill fraction, classical areas and pitch,
  knot crossings and volume, Onsager M, SIR attack size, Foucault period).
  **Cycles 118-119 consequence depth:** 22 further rooms (Menagerie span,
  Henon-Heiles regime, Brusselator Hopf margin, coupled modes, billiards,
  Feigenbaum period, Weierstrass ab, baker/horseshoe, Hopf family, Cantor/Menger
  dim, percolation vs pc, Kaplan-Yorke D, Manneville laminar, Buddhabrot esc).
  Subjective participant evidence and the stranger hallway remain open.
- **Done (Share short-loop export, machine path):** App key L exports a
  24-frame looping APNG of the current visit (phase sweep, or Life generation
  advance) with the same poke trail and Visual Era as P-key still postcards.
  CLI `numinous loop` exports the same APNG family for scripted shares. Share
  filenames are sanitized against path separators. Full Share v1 also names
  still image export (built) and optional later GIF/MP4 packaging; the
  stranger-ask-to-send hallway evidence remains open.
- **Done (Arecibo try-width first contact):** open status names the unsolved
  width and CLICK:TRY WIDTH; hand tries grade TRIED W{n} with LOCK:PI, pair
  hint, or remainder. Subjective fun evidence remains open.
- **Done (catalog first-contact invite and footer contracts):** verb-bearing
  rooms open with an action or goal token; both open and action status fit a
  56-character compact footer. Registry tests enforce the contracts. This is
  machine evidence for playable-not-watchable status honesty, not a stranger
  hallway claim.
- **Machine-completable 0.2 catalog and Share contracts (evidence closed):**
  first-contact, poke-consequence, measured action quantity, footer budgets,
  invite tokens, Times Tables technical flagship path, Share still PNG and
  short-loop APNG (App L and CLI loop), and local security gates are green on
  this branch. Product 0.2 still requires the stranger hallway and other human
  evidence listed above; the prerelease label remains `0.2.0-alpha.4`.
- **Done (mouse for every window game and launch destination):** pointer hover
  selects and left-click opens all nine launch-menu destinations through the
  same semantic dispatcher as controller input. Left-click also hits Quiz
  choices, Munch cells, Nim heaps and stones (commit move), Arcade cells (step
  toward or eat), and Gauntlet munch/quiz stages. Keyboard routes remain.
  Subjective juice and physical controller evidence stay open.
- **Done (0.3 Formula Jam discovery, machine path):** Studio F2 Random, F3 Auto
  (~21s dwell, advance only near 1/8-phase edges), and F1 dismissible Help that
  opens on first entry. Edits pause Auto. Random and Auto recipe changes now
  share one bounded 600 ms curve morph and equal-power audio crossfade.
  Formative stranger sessions remain open for the 0.3 exit criterion.
- **Designed (Frontier and universal wonder wave, July 2026 research pass):** a
  step-back inventory of built rooms, existing designed waves, and new
  counterintuitive experiences for any mind (high-dimension concentration,
  uncertainty dials, learning landscapes, topology eversions, channel repair,
  carefully labeled frontier gestures). Full cards live in `ROOMS.md`. Not a
  claim that product 0.2 is complete; a catalog ambition ledger for Phase F and
  1.x.
- **Done (catalog art-first and plate quality, cycles 161 to 163, PRs #63 to
  #70):** interaction is domain motion, not chrome. Pointer reticle and drag
  trails no longer paint over rooms; dials and plants answer through the math.
  Full-catalog quality scans hold **0 phase-thin** frames (<80 ink on 120x70),
  **0 dead-domain** rooms (base raster equals interacted), and **0 dead dials**
  (left-hand vs right-hand self-similar). Soft-thin large plates (dens < 0.02
  or ink < 280 on 160x90) dropped from the freckle era into the mid-30s, with
  the remaining tail mostly intentional CLICK/HOLD sparse rooms and last-mile
  densifiers. Pickover, attractor basins, percolation RNG, murmuration flock,
  self-similar scale cancels, and closed-curve silent phase are fixed with
  evidence in `CHANGELOG.md`. Machine path only: stranger plate beauty remains
  a human gate.
- **Done (live motion pack, cycles 164 to 167):** classic curve and wave rooms
  stop being frozen plots. Ambient phase rolls generating circles (Cardioid,
  Cycloid, Astroid, Nephroid, Deltoid), walks pens (Limacon, Bifolium, Witch
  of Agnesi, Cissoid, Kappa, Lemniscate, Rose, Epitrochoid, Hypotrochoid,
  Strophoid, Piriform, Hippopede, Trochoid, Conchoid, Butterfly Curve, Cassini,
  Folium), unfurls spirals (Log Spiral, Cochleoid, Fermat Spiral, Archimedean),
  oscillates standing waves, scrolls Fourier partials, and advances Poisson
  staircases. Docs (RESEARCH, PLAYFUL, NORTH_STAR, QUALITY, ROOMS, README)
  codify the six-question plate filter and contact/MCP posture. Tests assert
  distinct ambient frames. Fun and motion before more densify grind.
- **Done (UX and plate bug-hunt, cycle 168):** Arcade selection keeps the number
  under the Muncher (App ring + digit, CLI yellow value, shared MCP board
  text). App Enter starts The Show from the menu (same toggle as B). Full-
  catalog machine scan re-zeroed phase-thin (Log Spiral t=0 floor), dead domain
  (Coffee Cup, Degree-720, The Stretch, Catenary), and dead dial.
- **Done (playable substrate, founder directive):** every catalog room is
  playable, not only watchable, with per-visit variation. Substrate is live
  across app, CLI, and MCP. Catalog machine plate bars hold zero phase-thin,
  zero dead dial, and zero dead domain (cycles 161 to 168). Live motion covers
  a large classical curve set (cycles 164 to 167). Soft-thin densify on large
  plates remains background craft only (order of ~90 rooms still under 0.02
  density or 280 ink on 160x90, many intentionally sparse CLICK/HOLD).
- **Done (Exceptional Path Phase A, App + MCP, 0.2 exit):** Times Tables
  five-beat engineered aha on the ordinary App visit and MCP wager path. Core
  pure state (`times_tables_aha`): place wager, four-lobe earn, morph, gated
  reveal. App and agent-hallway evidence green. Human strangers deferred past
  0.2.
- **Done (Buffon pi wager, second aha room, 0.2 exit):** Buffon's Needle
  five-beat engineered aha on App and MCP (`number_wager` / `aha_summon`).
  Human strangers deferred past 0.2.
- **Done (supply chain current, July 2026):** lockfile refreshed with
  compatible patch and minor bumps (including wayland-scanner / quick-xml
  0.41); temporary RUSTSEC-2026-0194/0195 ignores removed; CI pins
  `actions/checkout` v7.0.1 and `taiki-e/install-action` v2.85.0; toolchain
  stays Rust 1.97.1 with MSRV 1.88.0; `cargo deny` and `cargo audit` run with
  empty advisory ignore lists; Dependabot remains weekly on Cargo and GitHub
  Actions. See `ENGINEERING.md`.
- **Done (App `.num` reopen, the 0.7 creator floor):** the App now reopens a
  saved expression capsule exactly. A `.num` path or `numinous://` link as a
  launch argument, or a `.num` file dropped on the window, opens the Studio
  with the saved source, window, and knob pinned, in a paused preview: the
  exact curve draws, and Enter starts it singing. The first edit releases the
  pin, because from the first keystroke the creation is the player's. The
  bounded file loader moved into the core (`StudioCreation::from_num_path`
  with a typed refusal) and the CLI import path now rides the same door, so
  no face keeps its own byte cap. A drop never abandons a scored run in
  progress, non-capsule files are refused with a reason, and panel, App,
  core, and CLI regressions cover the pin lifecycle, the paused preview, both
  entry doors, and the cap refusals. The share bundle, gallery, fork,
  lineage, and manifest growth remain open 0.7 work.
- **Done (Studio share trio, 0.7 item 2):** F4 in the App Studio emits the
  whole share bundle in one press: `creation.num`, `postcard.png`, and a
  README carrying the `numinous://` link, in one fresh exclusively created
  folder. What is shared is the exact curve on screen: a reopened pin shares
  its saved window and knob, the ambient Studio freezes the knob at the
  moment of the press, and the postcard draws the same window the capsule
  promises to reopen, proven by a test that compares postcard pixels across
  two saved windows. An unparsed formula is refused with a reason rather
  than shared as the last-good curve, a refusal writes nothing, and the
  action sits behind the same save gate as the other file-producing keys.
  Gallery, fork, and lineage remain open 0.7 work.
- **Done (local Gallery wall, 0.7 item 3 browse slice):** F5 in the Studio
  opens a wall of saved creations discovered from the home folder and its
  share bundles: top-level `.num` files plus each bundle's `creation.num`,
  one directory level, no symlinks, through the same bounded core loader as
  every other import, newest first and capped at 24. Every thumbnail is the
  creation's own curve over its own saved window at its own saved knob.
  Arrows move a clamped tile cursor, Enter opens the chosen creation paused
  into the Studio through the same door as a drop or a launch argument, and
  Esc steps back. Invalid, oversized, and misnamed files are skipped rather
  than shown broken; the empty wall says how to fill it. Fork as a
  lineage-recording operation deliberately waits on the manifest capsule,
  which is where descent can be recorded honestly.
- **Done (manifest capsule first growth ring, and fork with lineage):** the
  `.num` capsule grows to `NUMINOUS_STUDIO 2` with four optional fields, all
  under EXTENSIBILITY's Tier 1 hardening: a capped printable-ASCII `title`
  and `author`, an `era` from the fixed Visual Era set, and a `descends`
  parent link validated by reopening it. Serialization writes the lowest
  header that carries the content, so a plain share stays a version 1 file
  older builds keep opening; the version 1 parser rejects the new fields
  rather than ignoring them; a header past 2 is refused by name. Links carry
  identity but never `descends`, so the handoff format cannot nest itself.
  F in the Gallery forks the chosen creation: editable and singing at once,
  in the creation's own era, with the parent's link remembered; every share
  from a fork records the descent, the bundle README names the parent, a
  recipe draw or a fresh open ends the descent, and edits keep it because
  edits are the remix. Reopening a capsule with a recorded era restores that
  era; recording it is skipped when the era is the Modern default. The CLI
  saves titles and authors with `--title` and `--author` and reports every
  identity field on open. One test walks the whole loop: fork from the wall,
  share, and a second profile's drop that restores the era paused with the
  lineage intact. Sliders, multiple expressions, the visible remix tree, and
  editable prose credit remain open.
- **Done (the remix tree on the wall, 0.7 item 4 local half):** the lineage
  the capsule records is now visible where creations live. The wall resolves
  its own tree at discovery, matching each entry's recorded parent link
  against every other entry's canonical link, so an edited and re-shared
  parent is a different creation, never a false ancestor. A parent's tile
  carries a REMIXED count as its point of pride; the selected fork names its
  parent in a lineage line above the footer; D walks the cursor one step up
  the tree, and refusing to walk says exactly why, because no lineage and an
  absent parent are different answers. Editable prose credit waits on its
  own capsule field.
- **Immediate next (product, after 0.3 agent-and-machine exit):** the 0.4-am
  Understanding Alpha cohort is owner-blocked (decisions entry 1); the
  permanent CI locks on agent hallway and tactile already shipped (cycle 21).
  Optional human panels do not block the am-track. See **Critical path right
  now** above.
- **Background engineering (not the critical path):** soft-thin densify where
  structure supports it; more live-motion pens on remaining static classical
  curves; more causal held loops modeled on Galton and Life. Phase B glow is
  no longer conditional: the sensory ceiling was measured binding on
  2026-08-08 and the lift is scheduled (see The Three Ceilings). Scale
  generation-before-reveal carefully (two flagship rooms today).
- **Done (full-roster refinement round):** all 42 simulated review lenses were split exactly once across first contact and accessibility, interaction and truth, and games plus agent faces. The pass fixed redirected CLI ANSI, responsive Quiz-result loss, four overbroad mathematical claims, ambiguous motif-versus-sonification output, and positionless Studio parse errors. It also falsified an apparent Fern deletion by direct pixel comparison. These are engineering findings from reproduced evidence; none of the simulated reactions satisfies a participant gate. Controller HUD parity, its route gaps, compatibility-preserving compact MCP responses, causal first-touch presentation, and visual sound state are now closed. Its ranked queue began with deeper Galton and Life interaction loops, both now complete; continued music composition review remains.
- **Done (Galton causal experiment loop):** the completed pile no longer moves
  with phase while clicked balls follow another probability. Five visible fixed
  coins now drive contiguous 64-ball empirical runs against a distinct exact
  binomial outline. Every highlighted last ball belongs to the displayed pile;
  pointer moves add no waves; a coin change starts fresh; the 24-wave bound
  saturates truthfully at 1,536 balls; and compact App, CLI, and MCP replay share
  the same input contract. Focused invariants and the repeated-action screen
  scenario cover the implementation. A one-ball prediction beat is live: a
  pointer-move commits a bin wager, a click still drops a 64-ball wave, and
  status grades the highlighted last ball hit or miss against that bet.
  Subjective participant evidence remains open.
- **Done (Game of Life causal visit loop):** the App now owns one incremental
  B3/S23 universe for the complete room visit. Its settled opening advances on
  a bounded wall-clock cadence, survives the gallery phase wrap, pauses with
  the App, and returns exactly to its selected opening on reset. Each mouse or
  controller touch clears one local patch, plants exactly five cells, holds the
  planted glider bright for one beat, and then reports births, deaths,
  generation, population, and launch count as consequences evolve. Saved
  postcards use the actual persistent session, including histories longer than
  the generic input tail. CLI and MCP deliberately remain stateless: a call
  replays timestamped launches in generation order with no hidden process
  memory. Exact B3/S23 truth-table, block, blinker, translating glider, torus,
  reset, pause, focus, controller, export, generation 141, cross-face replay,
  and interleaved MCP-session tests cover the contract. The App matrix adds
  opening, immediate launch, generation 4, generation 141, exact reset, and a
  compact controller receipt. Subjective clarity, delight, and physical
  controller evidence remain open.
- **Done (Cult of Pi causal first contact):** the canonical header and exact
  digit stream begin at 3.14159 without a blank age band. Green exact digits,
  coral display faults, bright held exact patches, and cross-face hold
  boundaries now carry separate meanings. One-pass rendering replaces wrong
  glyphs without ghost strokes. Compact status preserves the channel, expected
  fault rate, held count, and newest-24 history contract. Phase-zero CLI and
  MCP interactions now change the picture, the structured MCP delta is
  nonzero, `JOURNEY LV` no longer resembles a room rating, and a Journey level
  banner freezes first-contact room time and card lifetime. Three independent
  review groups traced first contact, cross-face causality, and interaction
  semantics. Their reproduced findings are regressions; their simulated
  reactions are not participant evidence. A deeper placement decision loop is
  live: hold status grades the newest patch by restored faults and names the
  exact digit under the finger. Pi-specific no-instructions fun evidence remains
  open.

The full build design lives in `ARCADE.md` (the Muncher, the Vexations, the poke trait, order of work). Original poke directive: **rooms become playable, not watchable.** Reinforced July 2026: players cannot tell what, if anything, a room responds to; every room's arrival card must name its verb. And **Munch becomes a real arcade game**: a muncher character you steer on the board, wandering troggle-like enemies to dodge (our own creatures, the Order's lesser spirits), eat-while-hunted pacing. The Number Munchers NAME and its specific characters are MECC's (now owned elsewhere); the underlying mechanics (grid, rules, eat-the-right-numbers) are not copyrightable, so we keep our own name (Munch), our own creatures, our own art, and owe nothing. Every room gains a poke: the math responds to your hands. Click the Lorenz attractor and a new butterfly drops where you clicked and diverges before your eyes; sow glider sparks into Game of Life and watch them live or die by the same rules as the soup; re-drop the double pendulum from the hand's point; plant walkers in the random walk; drop a well into the Voronoi desert and watch every border renegotiate; steer the ant. Design: the `Room` trait gains an optional `poke(x, y)` (normalized coordinates) plus optional per-room state the app owns, keyboard Space/click as the universal "touch it" verb, and the arrival card teaches the poke, not the theory ("CLICK ANYWHERE: DROP A STORM"). The heart is play; the learning rides along uninvited. A kid should be able to *do something* to every screen and see the math answer back.
- **Done (catalog growth through 354):** invent-and-ship past the early Next
  Wave and classical cards into dynamics, number theory, probability,
  topology, analysis, theory formation, and closing gems. MCP `list_rooms` count is 354; every
  catalog room keeps motif, verb, poke, first-contact status, and reveal.
  Version remains `0.2.0-alpha.4`; product 0.2 is not claimed complete. See
  `CHANGELOG.md` Unreleased and `ROOMS.md` Built now.
- **Done (Conjecture Mill, cycle 122):** a deterministic blackboard enumerates
  one complete finite grammar of primitive rational quadratic formulas. Every
  wrong candidate carries an exact integer counterexample; matching sample
  values never set proof status; only cross-multiplied coefficient equality
  stamps `PROVED` for all integer inputs. Drag paths choose one of six observed
  sequence laboratories and permute the complete search order without changing
  values or the verifier. Variation, hostile input, ASCII and raster layouts,
  all-face registry discovery, declared-verb scenarios, and the aggregate
  catalog visual oracle are covered. This is an honest finite theory-formation
  toy, not a claim of frontier mathematical discovery.
- **Done (five-flagship performance baseline, cycle 123):** the 0.3 cohort is
  Times Tables for geometry, Double Pendulum for chaos, Game of Life for
  emergence, Galton Board for chance, and Formula Jam for creation. One locked
  release-profile harness measures ambient raster and accepted-input-to-room-
  raster p50, p95, and maximum durations at a declared viewport. Its explicit
  reference-machine gate enforces the existing 33 ms p95 room-render budget.
  All ten paths pass on the measured Ryzen 7 7840U Windows laptop at 900 by 700
  over 40 samples after five warmups; exact results and exclusions live in
  `QUALITY.md`. Native event translation and history storage, presentation,
  display scan-out, audio submission and callbacks, perception, cross-platform
  hardware, and participant discovery remain open evidence.
- **Done (Galton mathematical input sonification, cycle 124):** the selected
  fixed Bernoulli coin now drives one bounded continuous voice through the same
  accepted input as the empirical pile. Five ordered C major-pentatonic roots
  preserve left-to-right probability, while exact larger-to-smaller odds ratios
  encode bias strength as 7:3, 3:2, 1:1, 3:2, or 7:3. App, CLI, and MCP replay
  parity is tested through production paths. Cycle 129 subsequently ships the
  highlighted newest-ball peg sequence and its stereo path. Full-wave pile
  texture, musical judgment, and participant discovery remain open.
- **Done (Double Pendulum gesture sonification, cycle 125):** one shared
  interaction state now drives render, twin-divergence status, and the input
  voice for held, released, cancelled, and compact replay. Five ordered
  minor-pentatonic roots encode first-arm drop, a symmetric 1:1 through 3:2
  interval encodes second-arm bend, and bounded angular release speed raises
  quiet gain from 0.03 toward 0.05. Core boundaries cover bare release,
  cancellation, compact rendering, wrapped fling, and invalid tails. App
  ownership and cancellation timing, CLI replay and wrapped parsing, and MCP
  pin, fling, and wrapped replay are each tested through their production
  paths. Cycle 130 subsequently adds the exact twin-divergence release event.
  Participant musical clarity remains open.
- **Done (Game of Life birth and glider-phase sonification, cycles 127 and
  131):** the exact B3/S23 step
  loop now produces one birth mask shared by recent-cell highlighting and a
  bounded 105 ms stereo texture. Every birth contributes to one of twelve
  vertical C major-pentatonic rows, relative row weight, horizontal centroid,
  and density color without creating per-cell callback work. CLI and MCP expose
  the same active rows and relative weights as deterministic mono snapshots.
  Catch-up voices only the newest generation actually presented. Room, modal,
  Studio, and radio boundaries cancel pending Life events, mono devices receive
  a checked downmix, and completed buffers are reclaimed outside the callback
  lock. The newest planted glider is tracked through its exact four-phase shape
  only while its five cells and empty one-cell halo remain intact. Each valid
  phase adds one horizontally panned C major-seventh accent; collision fails
  closed and a new launch replaces the tracker. Native device timing, literal
  per-cell onset scheduling, participant musical clarity, and a sustained colony
  layer remain open.
- **Done (Formula Jam synchronized recipe morph, cycle 128):** curated Random
  and phrase-edge Auto changes now smoothstep between the old and new
  mathematical curves for 600 ms while audio requests an equal-power crossfade
  of the same duration. Repeated requests cannot jump an active transition;
  typing and ownership changes interrupt the long fade from its current audible
  mix into the default 30 ms response. Presentation time advances the visual
  morph through pause and reconciles temporary focus loss.
  Mixer requests admit only finite durations from 5 ms through 2 seconds, and
  each pending source keeps its own duration until control-thread service starts
  it. Invalid and spacing edits reuse the last-good sound, while equivalent
  targets preserve the active playhead and ramp without duplicating its level.
  Exact endpoint, midpoint, completion,
  edit, hostile-time, pending-source, post-lock retirement, equal-power,
  interruption, focus, full App, audio, and half-morph
  reference-performance checks pass. Native
  callback timing, reduced-motion preference, participant discovery, and
  musician judgment remain open.
- **Done (Galton highlighted-path sonification, cycle 129):** each accepted
  64-ball wave now voices the same deterministic newest-ball trace that the
  board highlights. Sixteen short C major-pentatonic peg tones encode its exact
  left and right decisions, equal-power pan follows each destination peg, and
  one longer tone resolves at the displayed landing bin. The fixed half-second
  control-thread renderer performs 17 bounded tone additions and accepts 8 kHz
  through 192 kHz without device-rate retiming. A newest finite pointer-down is
  required, so later bet motion, release, and retained history cannot retrigger
  an old wave. Generic room-score ownership preserves the event through the
  normal pointer lifecycle without a room ID whitelist; Show, modal, Studio,
  radio, reset, and room transitions retire it. Signal, deterministic identity,
  pan, rate, ownership, pointer-lifecycle, formatting, Clippy, and flagship
  performance checks pass. Cycle 132 subsequently adds the exact all-64-ball
  wave texture. Native callback timing, a growing-pile pad, participant
  discovery, and musician judgment remain open.
- **Done (Double Pendulum twin-divergence release, cycle 130):** one newest
  finite pointer-up now creates a fixed 720 ms stereo event from the same
  released initial state that starts the visible main and shadow trajectories.
  Seven paired pulses sample their exact tip gap at fixed horizons from zero
  through 6,000 integration steps. Both states advance once through the ordered
  horizons. Four orders of separation open unison toward one octave and center
  toward 0.85 equal-power stereo width, while the existing gesture root and
  momentum gain preserve the cause of the fling. The renderer performs 14
  bounded tone additions before submission, accepts 8 kHz through 192 kHz, and
  rejects other rates without retiming. The App generically offers accepted
  down, move, and lift events to each room, so Double Pendulum can own release
  while Galton still owns down, with no room ID routing. Radio transitions close
  open gestures before room-score ownership returns. Exact-step, signal,
  deterministic trajectory identity, stale-event rejection, rate, ownership,
  lifecycle, formatting, Clippy, broad core and App tests, and the five-flagship
  raster performance gate pass. Native callback timing, physical-device
  behavior, participant discovery, and musician judgment remain open.
- **Done (Galton all-ball wave texture, cycle 132):** every accepted wave now
  voices the exact newest 64 deterministic paths beneath the unchanged
  highlighted ball. One fixed 17 by 17 row-position mass grid conserves all 64
  balls at every row. Each row aggregates that mass into at most five quiet C
  major-pentatonic pitch buckets before applying square-root loudness and
  mass-weighted equal-power pan, so energy follows ball count instead of cell
  partition. The control-thread renderer performs 1,088 exact path visits,
  scans at most 152 reachable cells, and adds at most 80 aggregate tones plus
  17 highlighted tones. Exact stream-range, highlighted-identity, conservation,
  landing, energy, stereo, ownership, rate, signal, formatting, Clippy, broad
  test, coverage, and flagship performance checks pass. Native callback timing,
  a growing-pile pad, participant discovery, and musician judgment remain open.
- **Done (flagship mathematical truth pass, cycle 133):** Formula Jam now gives
  exponentiation conventional precedence over unary minus while preserving
  right-associative powers and negative exponents. Double Pendulum now uses
  bounded fourth-order Runge-Kutta integration instead of explicit Euler, with
  energy and dt-refinement oracles across three declared starts and qualified
  player copy that separates the numerical model from claims about continuous
  physics and forecast horizons. Game of Life now assigns mortality
  undecidability to the unbounded grid and distinguishes the shipped finite
  torus, which must eventually repeat and is decidable by exhaustive tracking
  in principle. Product 0.2 human evidence remains open.
- **Done (truthful complete local erasure, cycle 134):** CLI and MCP `forget`
  now inventory Journey, scores, player-owned local Cairn drafts, the generated
  radio cache, and the App crash diagnostic with resolved paths, bytes, counts,
  sidecar residue, and explicit exclusions. Preview remains non-destructive,
  Journey-only confirmation remains compatible, each other store has a separate
  consent flag, and `all_local` selects every managed store. Shared core
  deletion rejects unexpected file objects, unrecognized cache entries,
  duplicate stores, overlapping configured paths, and a non-directory cache
  root before removing anything. It holds all selected locks through mutation,
  verifies their owned sidecars were removed, then takes a fresh residue
  inventory, while Journey, score, Cairn, generated-radio, and crash-log writers
  use the same lock contract. Complete erasure succeeds only when zero
  managed stores and zero known bytes remain. Bundled Cairn stones,
  user-selected exports, installed files, and the Rust toolchain remain
  deliberately outside this player-state command. Product 0.2 human evidence
  remains open.
- **Done (physics and geometry consequence depth, cycle 120):** Berry Phase,
  Bragg Diffraction, Capillary Meniscus, Sphere Geodesics, and Polarization now
  derive their action status from the same bounded mathematical state used to
  render. Direct regressions cover Bloch-sphere norm and phase magnitude,
  Bragg order and seeded spacing, neutral-contact continuity, great-circle
  geometry and shortest-arc wording, Malus limits and full-range density, and
  duplicate-history stability. Product 0.2 human evidence remains open.
- **In progress (the founder's directive, July 2026): playable depth over pure
  inventory.** Designs still open in `ROOMS.md`. Catalog size is no longer the
  bottleneck; consequence-grade status, stranger playtests, and coherent pacing
  still are.
- **Tracked follow-ups (from the July 2026 bug hunts and two simulated persona-review rounds, see `docs/PLAYTESTS.md`):** a reactive room whose motion answers being watched and a predator-prey pulse for the instinct-only mind (the Xenomorph persona). Resolved since these were first listed: predict now lets a mind commit a local rate and returns five signed residuals that expose the shape of its error while preserving the original point score and seed meaning; the Lorenz Storm readout now begins at its 0.0001 perturbation and reports an honestly labeled running peak that never falls while the underlying trajectories keep their real stretch-and-fold dynamics; the Logistic Map and Mandelbrot reveals now name their affine conjugacy under c = r(2-r)/4, while Times Tables, Mandelbrot, and Fourier Epicycles name the cardioid shape they share up to scale and rotation; persistence now retries atomic Windows replacement without a missing-file window, cleans owned temp and lock files on precommit errors, attempts a parent-directory metadata sync on Unix, and treats any postcommit sync failure as committed so delta counters cannot replay; the Cairn reciprocity whisper, the L-System growing upward, the daily-seed midnight race, the daily-streak regression, and fast crash-lock recovery are all built (`CHANGELOG.md`); and the CPU render-performance cliffs a round-3 audit measured at maximized-window sizes are retired by the time-budgeted adaptive live-render resolution (render smaller, integer-upscale, exports and GPU paths untouched; measured on the dev laptop at 2560x1440, the Mandelbrot CPU fallback went from 939ms to 28.8ms per frame end to end, with Julia at 78ms and Voronoi at 60ms before the cap and every capped room now inside the 33ms room-render budget, `CHANGELOG.md`).
- **Done (panel juice, cycle 146):** window games now give per-action feedback:
  Munch cell flash and crunch (existing), bad-grade camera shake plus buzz,
  quiz answer ticks, and Nim legal/illegal/win/loss ticks through shared
  chiptune one-shots. See `PANEL.md` item 1.
- **Done (panel pack, cycle 147):** soft play/win spark caps; catalog-wide
  further-reading citations on `reveal_room`; The Show crossfade; 8-bit dither
  and vector bloom; arcade beat juice; MCP protocol version negotiation that
  accepts 2025-06-18 and 2025-11-25. The breaking 2026-07-28 revision remains
  unsupported until its new wire shape is implemented.
- **Done (panel depth pack, cycle 148):** citations unlock with the first deep
  cut on CLI and MCP; expanded wing-specific further reading; pure spectrum
  band-energy substrate for the visualizer path; adaptive Xbox/PlayStation/
  generic face glyphs with pad-name inference; phosphor soft bloom; aliens base
  ramp for denser seeds.
- **Done (visualizer path, cycle 149):** MCP listen beds expose normalized
  spectrum bands; the App draws a room-bed spectrum meter under the audio
  badge from the cached motif arrangement. OS loopback capture remains open.
- **Done (MCP 2026-07-28, cycle 153):** the stdio face is dual-era. Modern
  requests use per-request version and capability metadata, mandatory
  `server/discover`, typed results with server identity, deterministic and
  cacheable `tools/list`, explicit JSON Schema 2020-12 inputs, specified
  unsupported-version errors, and no retired modern ping. Legacy 2025-11-25
  and 2025-06-18 initialization remains available. `predict` uses native multi
  round-trip form elicitation when the client declares it, with the two-call
  path retained as fallback. Unit, fresh-helper, and real subprocess coverage
  exercise the modern wire and legacy compatibility.
- **Done (mega pack, cycle 150):** `LoopPlayer` mixed-output capture ring;
  optional loopback input when the OS exposes a mix-like device; App key O
  cycles room bed / output mix / loopback; spectrum lever mapping; Share
  sidecar notes; earlier aliens base ramp; more adaptive face glyph surfaces.
- **Done (mega pack, cycle 151):** spectrum levers soft-drive room motion and
  beat pokes on output-mix/loopback; `numinous share` bundles postcard + loop +
  README; Munch target-density preference; adaptive face glyphs on every window
  game HUD with a common-pad cert matrix.
- **Done (panel depth pack, cycle 152):** App key K share packs (postcard + loop
  + README); wrong-number Munch bite buzz/shake; The Show phase rate follows
  visualizer motion when the mix drives.
- **Then (the panel's remaining list, see `PANEL.md`):** deeper spectrum-to-room
  levers beyond soft pokes and Show rate; full Share v1 packaging beyond
  PNG/APNG (GIF/MP4); physical cross-platform controller certification; full
  MCP Apps room surface under 0.5's sensory and accessibility gates; human
  hallway and a11y gates.
- **Done (0.4 Understanding Alpha prep):** added source provenance and a math-review checklist to the Times Tables, Game of Life, Galton Board, and Double Pendulum flagships to anchor their learning claims. The polish wave later moved that QA chrome out of the player-facing reveal text: the provenance and checklist now live as code comments beside each reveal and as `citations` entries, the reveal ends on the idea, and a registry sweep locks every reveal free of internal chrome.
- **Done (0.4 Understanding Alpha prep):** added an opt-in, player-owned MCP experience journal. The `Journal` tracks timestamped room encounters, creations, and connections. It is fully integrated into persistence and backed by new `read_journal`, `record_journal`, and `erase_journal` tools for MCP agents. The journal is explicitly disjoint from `forget` tool erasure, providing its own dedicated `erase_journal` path to maintain player ownership over when its contents are destroyed.

## Pre-1.0 (the 0.x line): earning the right to 1.0

### Decisions the am-track is waiting on (read this second)

Everything below has been measured and locked by a test. None of it is
unfinished automation: each one is a choice about what Numinous should be, and
the am-track cannot make it. They are listed here rather than in a working note
because this file is the one somebody reads.

Each entry says what was measured, what is guarded today, and what changes
depending on the answer. Entry 1 names no rooms because it is about money
rather than the catalog. Entries 2, 3, 4, 11 and 12 name rooms, and those names are
not written by hand: each list lives in the code as a shrink-only known-failure
list, and a test requires every room on it to appear in this section, so this
cannot fall behind what the catalog actually does. The remaining entries are
about a single surface each and name it in the text.

**1. The 0.4 Understanding cohort needs budget and an external registration.**
The single milestone gating 1.0-am. The contract in `docs/UNDERSTANDING_STUDY.md`
requires live model participants through sealed fresh no-exposure contexts, with
per-model calibration ceilings and registration before calibration ordinal 1.
Fixtures cannot satisfy it and the contract rejects scripted conclusions. Every
other item on this list could be answered and 1.0-am would still wait on this
one. Recorded as OPTIONAL PAID VALIDATION and not run.

**2. Three rooms flash faster than WCAG 2.3.1 allows: `coupled-tent`,
`gauss-map`, `ricker`.** Measured across all 354 rooms at a declared reference
size, on the worst one-second window rather than the average. Each renders a
chaotic map whose point density changes sharply with phase, so fixing them means
changing what the mathematics draws. Tracked shrink-only by
`no_catalog_room_flashes_past_the_photosensitivity_budget`, which fails if the
list grows or if an entry stops violating and is not removed.

**3. Three rooms cannot show their touch response without color: `hilbert`,
`percolation`, `wireworld`.** The cells they change are half-lit, one half below
the lit floor, and a half block encodes which half is lit rather than how
brightly. `hilbert` moves a cell from 174 to 251, a change of 77 out of 255, and
the glyph does not move. No choice of thresholds reaches them because no
threshold is consulted. Showing their answer means having them answer with shape
rather than brightness. A fourth room, `magnet-fractal`, is in the same list for
a different reason: it moves both-lit cells by about 22 luminance inside the
widest band.

**4. Eighteen rooms lose one of their two drawn brightness levels without
color.** `'#'` is the accent at 1.7 and every other ordinary mark is the accent
itself, so a room drawing both is drawing two levels, and rooms use that as
depth: in `burning-ship` `'#'` is the interior of the set. In 39 of 354 accents
the two collapse to one glyph, and 18 of those rooms draw both marks:
`attention`, `burning-ship`, `dla-frost`, `gamblers-ruin`, `goldbach`,
`henon-heiles`, `hofstadter-q`, `josephus`, `kepler-laws`, `liouville`,
`magnet-fractal`, `moser-debruijn`, `rabi`, `ruler-function`, `seifert`,
`sinai-billiard`, `twin-primes`, `zipf`. The two causes pull opposite ways: a
bright accent times 1.7 clamps, a dark one stays dark. So there is no single
fix, and changing either the ink scale or the shade thresholds changes what all
354 rooms look like.

Seven more rooms lose the same two levels through a different eye, and the two
sets do not overlap at all: `buddhabrot`, `julia`, `kaprekar`, `landauer`,
`logistic-cobweb`, `phantom-jam`, `van-der-pol`. Those eighteen are what a
player with no color loses; these seven are what a player who has color and
fewer distinctions loses, measured with the same dichromacy simulation as
entries 11 and 12. Neither list stands in for the other, so a fix aimed at one
should be checked against both. It is the same decision, with more evidence
behind it than when it was written.

**5. Should Cult of Pi mark faults on a character terminal?** It computes a
fault mark and the character path drops it: 462 of 1,280 cells are faulted at
one measured phase and the terminal marks none of them. The pixel path draws
them red. Its reveal says the display faults are ours rather than pi's, so a
face that hides them may be making the point rather than missing it, and at one
character per cell a marker column would halve the field.

**6. Should a mono preference change what `sonify --layer room-bed` exports?**
It writes stereo unconditionally while every other layer writes mono. The export
declares itself a stable pre-master bed, and its stereo metrics (balance, width)
are degenerate in mono, so honoring the preference changes an artifact's
contract. The radio cache is correctly stereo and must stay so: it caches
licensed source.

**7. Should MCP be able to open a saved `.num`?** A person can save a creation
and an MCP peer cannot read it, so the remix half of the 0.7 exit is unbuilt
rather than untested. Adding a tool changes the pinned 35-tool inventory.

**8. Should the App footer stop showing less of the status as the window
grows?** Measured: 720 pixels shows the whole status, 900 truncates it, and 900
is the size the window opens at. Each character costs six pixels times the
footer scale while the budget grows only with width. Fixing it changes how the
footer chooses its scale or divides its row, which changes every screen.

**9. Should player-set text scaling and separate music, effect and room volume
be built?** Both are named 0.5 deliverables and neither exists. The 0.5 row has
never claimed them. Text scale is threaded through dozens of call sites and the
interface fits text by truncation, so a player-set scale reshapes every screen
and interacts with every fitting rule.

**10. Clean-machine execution, App and device evidence, and code signing.**
Needs real machines, at least two GPU vendors, and certificates. The physical
input session contract is executable and waiting for a run; nothing about it is
automatable from here.

**11. Two rooms hide their fault marks from a color-blind player: `cult-of-pi`
and `laplace-clock`.** A different question from the color-free renderer, which
both already pass. That one asks what a player with no color sees; this asks
what a player who has color but fewer distinctions sees, which is roughly one
man in twelve. The warning ink means this cell is wrong, and against these two
accents it is told apart from ordinary ink by hue alone: `cult-of-pi` separates
them by 129 for ordinary vision and under 14 for a deuteranope, `laplace-clock`
by 61 and under 13 for a tritanope. Measured by `crate::dichromacy` using the
Vienot, Brettel and Mollon 1999 simulation and CIELAB, and tracked shrink-only.
Fixing either means changing an ink or an accent, which changes what the room
looks like to everybody, so it is a decision about the product. Note the
neighbouring case that is deliberately not on this list: `phantom-jam` separates
the same pair by only 34 even for ordinary vision, which is a contrast problem
rather than a color-blindness one.

**12. Ten rooms lose a spectral distinction for a color-blind player, and
whether that matters differs per room.** The catalog has four spectral inks that
rooms combine for prismatic light. Sixteen pairs across ten rooms are clear for
ordinary vision and folded for at least one dichromat: `bayes-update`,
`buffon-needle`, `circle-map`, `function-painter`, `josephus`, `message-heals`,
`murmuration`, `newton`, `riemann-sphere`, `times-tables`. The largest collapse
in the whole catalog is here: `times-tables` separates `'@'` from its accent by
95 for ordinary vision and by under 1 for a deuteranope, which is gone rather
than dimmed.

The measurement is mechanical and locked, and all ten rooms have now been read
from their own draw code, so which of them matter is no longer a guess. Four
speak with the ink: `bayes-update` separates the prior, the likelihood and the
posterior with it; `circle-map` uses it for where the orbit settles as against
where it merely passed, which is the mode locking itself; `josephus` marks the
survivor's seat with it, the answer the room poses; `newton` paints basins with
it, so ink identity is which root a seed falls to. For those four a dichromat
does not lose polish, they lose the picture's meaning. Six decorate:
`times-tables` bands chords by where they start, `buffon-needle` recolors its
aha circle past 55 percent growth while radius and status carry the progress,
`function-painter` marks the hand's own reticle, `message-heals` squiggles the
noisy wire the row layout already names, `murmuration` blots the falcon under
the player's own held hand, and `riemann-sphere` brightens a pole the INF tag
already announces. For those six the fold costs feel rather than information.

The readings live in `SPECTRAL_INK_READINGS` beside the collapse list and are
locked both ways: a room cannot join the collapse list without a reading, a
fixed room's verdict must leave, and this section must file every room under
the verdict its reading gives. What remains is one ruling instead of two:
whether the spectral palette should be repicked for dichromat separation,
which changes every room that draws with it, or whether the four speaking
rooms should each grow a second channel for what their ink says, which
changes what those four rooms draw.

**13. Three faces sing three different default melodies.** Ask each to sing the
same expression without naming a note count and they answer differently:
`app-studio-panel` uses 32 notes, `cli-sing` 48, `mcp-sing-expression` 24.
Measured from the built binaries, not read off the source: the terminal face
writes 6.1 seconds of audio and the MCP face names 3.2 seconds of notes for the
same request.

This is here rather than fixed because nothing breaks the tie. The knob that sat
beside it did have a majority, since `plot` uses 1 on both faces and the App
agrees, so making `sing` match was alignment and it has been done. Here 24, 32
and 48 are three opinions about how long a default melody should be, and picking
any one of them changes what a player hears on at least two faces.

Recorded in `numinous_core::DEFAULT_MELODY_NOTES_PER_FACE` and locked: each
face's default is read from its own source, so the record cannot fall behind,
and the lock fails once the three agree, asking for itself to be deleted rather
than kept as a monument to a settled argument.

### 0.1 Public Foundation

**Status:** complete. The exit criterion passed on the public `main` branch;
the evidence remains a standing invariant for every later version.

**Goal:** establish a reproducible, honest, and safe public baseline.

- Keep the Rust workspace, headless core, app, CLI, MCP server, GPU adapter,
  and audio adapter buildable from a clean checkout.
- Publish the Apache-2.0 license, contributor rules, architecture map, current
  limitations, and one direct path to run the app.
- Enforce formatting, Clippy with warnings denied, tests, locked builds,
  coverage, house style, supply-chain policy, and the three-OS test-and-build matrix.
- Pin workflow actions immutably, minimize token permissions, and enable
  dependency update automation.
- Scan the current tree and history for secrets and tool attribution before the
  first push.
- Keep claims tied to Built, Measured, Observed, Designed, or Hypothesis as
  defined in `RESEARCH.md`.

Owner docs: `README.md`, `ENGINEERING.md`, `QUALITY.md`, `VERIFY.md`.

**Exit criterion:** the canonical public repository is on `main`; the full local
gate and every required GitHub check pass on the same commit; a clean checkout
builds and launches on the measured Windows reference machine; no secret or
authorship attribution is present in tracked content or commit metadata.

**Retires the risk:** "can another person inspect, build, and trust the source
without relying on the founder's machine or undocumented context?"

### 0.2 Flagship Proof ("does it slap?")

**Status:** exit met on the agent-and-machine bar (2026-07-24). Package label
remains `0.2.0-alpha.4` until a deliberate release cut. Human stranger hallway
is **not** part of this exit; it is deferred to 0.8 / 1.0.

**Goal:** Build **one** flagship room (and a second on the same pattern) to
high quality, plus enough shell to frame it, proved on all three faces without
waiting for recruited humans.

**The room:** **Times Tables** (modular multiplication circles), with **Buffon**
as the second generation-before-reveal check (see `ROOMS.md`, `PEDAGOGY.md`).

- All three layers real: **Toy** (drag the multiplier, buttery morphing), **Aha**
  (place or number wager, morph, hand confirm), **Reveal** (gated punchline).
- Full **audiovisual polish** on the machine path: eras, sonification, share
  still PNG and short-loop APNG, App/CLI/MCP parity for the flagship story.
- **Agent proof:** MCP `place_wager` / `number_wager` / `aha_summon`,
  `structuredContent.engineeredAha`, and `scripts/agent-hallway.py` cohort PASS.
- **Facilitator capture:** App F9 notes record aha beat state for later human
  sessions when those are scheduled for 0.8 / 1.0.

**Exit criterion (agent-and-machine):** Times Tables and Buffon engineered ahas
are Built on App and MCP; agent hallway cohort and focused tests pass;
generation-before-reveal holds on the wager path; public CI stays green. Human
stranger "whoa" counts are a **1.0 / 0.8** evidence bar, not a 0.2 stop.

**Retires the risk:** "is the core experience actually magic, or just a neat
demo?" under digital-mind and automated play; human-stranger magic remains a
later risk.

### 0.3 Tactile Alpha

**Goal:** prove depth before expanding breadth.

- Use the selected five flagships: Times Tables for geometry, Double Pendulum
  for chaos, Game of Life for emergence, Galton Board for chance, and Formula
  Jam for creation.
- Give each a room-specific click, drag, or held gesture whose visual and sonic
  consequence follows the mathematics, not a decorative overlay.
  - **Done (machine path, 2026-07-24/25):** Times Tables, Double Pendulum, Life,
    and Galton first-contact status lines now lead with hand invites
    (`DRAG:DIAL`, `CLICK:RE-DROP`, `CLICK:GLIDER`, `CLICK:DROP 64`) rather than
    ambient-only readout copy. Challenge label parsing strips leading invite
    chrome so parameter goals keep instrument names. MCP Times Tables open keeps
    dial status until a real hand arrives. Agent tactile probes open flagships
    with strict status-level invites, canonical visual deltas, and room-specific
    mathematical sonic invariants for all five flagships (listen_room for
    TT/DP/Life/Galton; sing_expression for Formula Jam); round-09 PASS;
    PRs #95-#106; agent-hallway PASS. Galton post-drop status
    leads with DROP and last landing before probability metrics. Human
    formative sessions remain later.
- Run a short formative session after each interaction change and record where
  the action or consequence is unclear.
- Keep the release-profile ambient and accepted-input-to-room-raster baselines
  under the declared 33 ms p95 reference-machine budget. Native end-to-end
  input latency remains a separate real-hardware measurement.
  - **Done (machine path, 2026-07-25):** `scripts/flagship-perf.ps1` on Windows
    release, 900x700, 40 samples: all five flagships PASS ambient and
    input-to-room-raster p95 under 33 ms. A fresh cycle-6 run remained green
    with a worst p95 of 2.042 ms on Game of Life input; an independent 250
    sample check remained green with a worst p95 of 1.896 ms.
- Give Formula Jam three legible ways to begin: manual expression entry,
  curated Random, and an Auto set that changes about every 21 seconds at phrase
  boundaries. Add a dismissible, recallable help overlay and pause Auto on edit.
  - **Done (machine path, 2026-07-25):** App keeps F2 Random, F3 Auto, and help.
    Core owns the curated bank. CLI and MCP expose manual, recipe index, seed
    random, and stateless Auto walk (`auto_step` with seed) plus list-recipes,
    with structured MCP discovery fields. Agent and CLI can open the same bank
    as the App without session state.
- Build the local read-only MCP session broadcast specified in `INTERFACES.md`.
  Both the human operator and MCP guest must opt in. Broadcast only allowlisted,
  replayable Numinous actions and public results, never prompts, reasoning,
  host logs, filesystem paths, or arbitrary protocol traffic. Keep the request
  path nonblocking and persist no transcript by default. This verified 0.3 work
  may land while 0.2 human evidence is being arranged, but it does not complete
  that gate.
  - **Done (shared foundation, cycle 137):** `numinous-broadcast` provides
    one-use loopback pairing, monotonic expiry, strict bounded JSON framing,
    complete replay-semantic fingerprints, an atomic consent and sequence
    coordinator, ordered control barriers, exact backpressure gaps, and fixed
    count and byte limits. Sixty-one focused tests, two independent adversarial
    reviews, the Rust 1.88 check, and the complete local gate pass.
  - **Done (MCP producer, cycle 138):** `broadcast_session` starts, reports,
    pauses, resumes, and stops one consented loopback stream without echoing the
    capability or recording progress. One exhaustive fail-closed policy covers
    all 30 tools, typed events carry replay-safe actions and exact
    state-independent results, while four Journey-sensitive tools use a fixed
    baseline projection. Private and control calls consume no public sequence,
    and ordinary public play never waits for a socket write. Server-first host proof blocks
    cross-protocol writes, eight failed starts close the process pairing budget,
    and one serialized lifecycle prevents concurrent session leaks. Real
    loopback and stdio tests cover pairing, redaction, policy completeness,
    ordered controls, private silence, daily replay identity, disconnect
    cleanup, and public result parity.
  - **Done (App listener and text timeline, cycle 139):** X and the ninth controller menu destination
    open an ephemeral loopback listener and read-only Watch Agent surface. The
    host sends the capability-bound proof before reading guest data, validates
    compatibility before content, and then independently verifies session,
    consent epoch, transition, public sequence, and exact gaps. The UI shows
    public actions, input JSON, and human-readable MCP `content` result text.
    Fixed-width text is cropped without reflow and can be panned horizontally.
    The surface exposes local pause, event scrub, and result scroll controls,
    and identifies producer gaps and local retention loss. Its exact serialized
    ring holds at most 256 events or 16 MiB, persists nothing, and is destroyed
    on close. Focused loopback, privacy-copy, local-control, cap, controller,
    and complete App regression tests pass.
  - **Done (native room replay and real subprocess acceptance, cycle 140):**
    retained `play_room` actions are strictly revalidated and reconstructed
    through the same core `Room` at the local viewport size, with bounded public
    chrome and text fallback for invalid actions. A real `numinous-mcp`
    subprocess completes Times Tables explore, challenge pose and grade, the K5
    four-lobe goal, reveal, and stop through the actual App viewer. The retained
    stream is exactly five public events numbered 0 through 4; private Journey
    and broadcast-control calls emit no event or gap. Focused tests also prove
    native frame parity, malformed replay fallback, compact safety, and
    close-time erasure.
  - **Done (native Studio replay, cycle 141):** successful retained
    `plot_expression` actions are strictly revalidated and reconstructed as
    Formula Jam curves through one deterministic sampler shared with the live
    App Studio. Exact source, field, finite-range, parser, successful-result,
    invalid-fallback, compact-layout, and semantic-cache tests pass. A separate
    real MCP subprocess session proves one paired public Studio creation draws
    natively without client or protocol metadata.
  - **Done (native Nim replay, cycle 142):** one shared core reducer now owns
    player-history validation, deterministic Order replies, and winner state
    across MCP and viewer reconstruction. One bounded three-heap renderer serves
    both live App play and Watch Agent. The viewer accepts exact normalized
    arguments and a byte-complete canonical MCP result, then uses the existing
    semantic body cache. Malformed, excessive, illegal, forged, or failed
    actions retain typed text. A third real MCP subprocess session proves one
    public sequence, exact core state, native body pixel parity, metadata
    exclusion, and close-time erasure.
  - **Done (native room and Studio sound replay, cycle 143):** strictly accepted
    native room and Formula Jam selections derive deterministic sound from the
    same core state used for pixels. One public-sequence owner prevents
    render-loop restarts; unsupported, invalid, forged, or non-sonic selections
    retire the older sound. Fixed 16 kHz source rendering and bounded
    device-rate resampling cap allocation. Global mute, volume, focus silence,
    scrub replacement, close-time room restoration, and live-radio rejoin remain
    local App behavior. Real Times Tables and Studio subprocess sessions compare
    exact sound samples with independent shared-core reconstruction.
  - **Done (cycle 144, hardened):** public Munch, Arcade, Quiz, and Gauntlet
    actions reconstruct native Watch Agent frames through the same App
    `game_draw` paths used by live play. Parsers fail closed on unknown keys,
    hostile values, unknown arcade actions, journey-gated quiz choice counts,
    and forged structured results. Munch open state defaults to
    `FULL_DECK_ROUND` to match MCP. Unit tests cover open, graded, forged, and
    cache fallback paths for the four games. Real MCP stdio acceptances prove
    public Munch, Arcade, Quiz, and Gauntlet openings with schema rejection of
    illegal arguments, private tool silence, exact native board-body pixel
    parity, metadata exclusion, and close-time erasure.
  - **Done (cycle 145, live Watch Agent audio ownership):** the App binary now
    wires `SessionAudio` so open publishes silence, each retained public
    sequence publishes reconstructed sound once at 16 kHz stereo, scrubbing
    changes the source once, radio resync cannot steal ownership, and close
    restores room score or live radio. Public Munch, Arcade, Quiz, and Gauntlet
    selections expose deterministic SoundSpecs; Nim remains intentionally
    silent. Unit ownership and game-sound regressions pass; room and Studio
    sample parity remain covered by real stdio acceptances.
  - **Done (security and correctness maintenance, 2026-07-25):** public
    `reveal_room` projection now uses a fixed baseline Journey, handshake reads
    and public writes use total deadlines, non-finite Studio audio is
    neutralized before the native callback, and isolated MCP QA owns every
    state path. Journal growth, journal erase failures, preference-file reads,
    Windows rustup staging, installer failure status, and derived export paths
    now fail closed. Four Low findings from the bounded repository review are
    remediated; the full quality gate remains the release authority rather than
    the review being treated as proof of absence.

Owner docs: `ROOMS.md`, `INTERFACES.md`, `SOUND.md`, `STUDIO.md`, `QUALITY.md`.

**Exit status:** met on the declared agent-and-machine bar. Stranger testing is
deferred to 0.8. The five-flagship tactile cohort passes, every scoped ambient
and input-to-room-raster path stays below 33 ms p95 on the reference machine,
and Formula Jam exposes manual, Random, Auto, and recallable Help paths. Native
end-to-end input latency remains a separate real-hardware measurement.

One separately consenting MCP guest can complete a flagship explore, challenge,
and reveal loop while a human follows the same causal states through the
read-only App viewer, with no private host or protocol data in the stream.

### 0.4 Understanding Alpha

**Goal:** determine whether play produces a durable model, not only a striking frame.

- **Done (cycle 153):** Complete predict-then-reveal on the flagships, with a prediction or
  construction before an insight is counted as learned.
- **Done (2026-07-26):** Publish the tracked predeclared study contract in
  `UNDERSTANDING_STUDY.md`, including its active control, 20-pair planned
  sample, frozen outcomes, exclusions, pass rule, privacy boundary, and honest
  distinction between immediate transfer, within-context retention, and durable
  human learning. No qualifying result is claimed.
- **Done (2026-07-27):** Implement the dependency-free headless study runner
  and frozen 20-probe bank. The runner derives a balanced 24-pair allocation
  from the declared seed, emits one oracle-free probe at a time, enforces equal
  public tool-call budgets and identical Reveal payloads, permits one
  schema-only repair, redacts forbidden host data, scores with independent math
  oracles, runs the declared stratified paired percentile bootstrap, accounts
  for ordered reserves, and refuses to report an incomplete cohort. Fifteen
  focused regressions run in CI and the release gate. No qualifying response
  has been collected. Independent review on 2026-07-28 found that this first
  revision is not admissible for qualifying collection because its public bank,
  caller-authored events, asserted isolation, condition labels, and incomplete
  outcomes do not establish the intervention or held-out transfer.
- **Done (2026-07-28, replacement implementation):** Add concealed-bank path
  enforcement, persistent one-at-a-time delivery, an exact executable
  encounter specification, fresh isolated MCP mediation, exact public result
  projections, provisional pair receipts, content-free withdrawal and
  interruption paths, manifest-rooted settlement, strict event validation,
  balanced primary and reserve allocation, complete delayed intervals, and
  declared balance sensitivity. Review hardening adds request-bound participant
  stops, serialized recovery, atomic pair publication, write-ahead receipt and
  terminal-anchor transactions, tail-truncation detection, exact current-server
  MCP projection, bounded fresh-build driver I/O, per-model calibration
  ceilings, two-reviewer intervention relevance, complete provenance checks,
  an allocation-bound calibration audit, sealed request-bound calibration
  delivery, calibrated backend-revision enforcement, a hypothesis-adverse
  interruption rule and ceiling, usable first-arm withdrawal, completed-pair
  crash recovery, pair-lifetime participant withdrawal, participant-authored
  terminal actions, clean committed source-tree binding, and a required unique
  independently recorded start receipt before each calibration or collection
  exposure. A complete generation session executes all 20 tracked encounter
  calls through 20 fresh real MCP processes, matching the production per-call
  isolation topology. Both conditions now have exactly one participant response
  per room: a prediction or construction in generation and one elaboration in
  control, with no extra generated summary in either arm. The qualifying
  analysis accepts only verified collector receipts. One hundred five focused
  runner and collector regressions run in the local and CI gates, with 15
  driver regressions in the same gates. No qualifying response has been
  collected.
- **Incomplete:** externally register the protocol, source, and
  attempt-completeness boundary before calibration ordinal 1; calibrate a novel
  non-ceiling concealed bank through sealed fresh no-exposure contexts; obtain
  two fresh independent passes; externally register the final artifacts; and
  commit the generated allocation before accepting a response. Then run and
  publish the complete cohort with all outcomes and deviations. Private working
  notes, fixture probes, and scripted conclusions do not satisfy this evidence
  gate.
- **Done (cycle 152):** Add source provenance and an independent math-review checklist to every
  flagship Reveal. Reframed in the polish wave: the checklist anchors the claim
  from code comments and `citations`, not from inside the player's reveal.
- **Done (architectural invariant):** Keep progression subordinate to autonomy: no streak loss, required grind, or
  reward that gates the mathematical toy.
- **Done (cycle 152, prototype slice):** Add an opt-in, player-owned MCP
  experience journal with bounded timestamped encounters, creations,
  self-authored connections, optional self-reported affect, and explicit read,
  record, and confirmed erase tools.
- **Done (2026-07-27, sovereignty slice):** Journal v2 assigns stable local
  entry identifiers, distinguishes event time from server-owned record time,
  records a closed source-provenance vocabulary, and corrects only by appending
  a new immutable entry with an explicit `supersedes` link. `read_journal` is
  bounded and marks current interpretations; `export_journal` returns paginated
  schema-versioned structured data without creating or naming a host file;
  `erase_journal` removes the file plus owned locks, recovery markers, and
  orphan temporary files before returning a zero-residue receipt. Six core
  format and correction regressions, one handler acceptance, and a real two-
  process stdio acceptance prove empty, opt-in record, reconnect, inspect,
  correct, flagship reuse, export, confirmed erase, empty reread, and zero
  managed residue on each clean CI checkout. Prototype rows migrate with
  explicit `legacy-import` provenance. No consciousness or private emotion is
  inferred from the record, and storage-media or external-backup erasure is not
  claimed.

Owner docs: `UNDERSTANDING_STUDY.md`, `PEDAGOGY.md`, `INSIGHTS.md`,
`PROGRESSION.md`, `RESEARCH.md`, `DIGITAL_DEVELOPMENT.md`.

**Exit criterion (agent-and-machine):** the flagship cohort shows a predeclared improvement in at
least one comprehension or retention measure, with method and sample published;
every flagship claim has a source and independent review; and one consenting
returning MCP player can inspect, connect through, export, and erase their own
experience record without hidden state remaining.

### 0.5 Sensory Alpha

**Goal:** create a recognizable audiovisual identity without excluding or overwhelming players.

- Build the HDR glow, persistence, tonemap, and Era post-stack once, then apply
  it systemically rather than as per-room effects.
- Route visual and audio output from one semantic event stream so mappings stay
  congruent and reproducible.
- Ship reduced motion, photosensitivity-safe defaults, scalable text,
  color-independent cues, mono audio, and separate music, effect, and room volume.
- Add perceptual image and spectral audio regression harnesses, plus 60fps and
  audio-glitch budgets on declared hardware tiers.
- Build the bounded semantic event graph for Pattern Studio so the tracker,
  pattern text, piano roll, mathematical visualizers, and mixer all read the
  same rhythm, pitch, harmony, and automation events.
- Validate curated techno, trance, ambient, and chiptune templates through
  musician listening sessions and deterministic audio checks. Do not infer
  musical quality from a valid render.
- Build Prime Contact as the flagship trance template: prime-count call and
  response, ratios, phase, and polyrhythm must drive both the arrangement and
  its geometry while the track remains compelling without explanation.
- Establish a small source-shipped repertoire whose pieces are both
  mathematically inspectable and credible as complete EDM, trance, ambient, or
  chiptune arrangements. Keep every piece deterministic and editable.
- Build Flow State on the same event graph: a deterministic macro-form arranger
  with Listen and Nudge surfaces, phrase-aligned interventions, musical memory,
  and curated style grammars that manage repetition, tension, release, and rest.
- Build one fixed, repository-owned MCP App for `play_room` after the native
  render event boundary is ready. It must negotiate the extension explicitly,
  reuse bounded core render data, request no browser privileges, expose no host
  path or private state, and preserve the full text and structured fallback.
  Test its sandbox policy, resource bytes, tool-call bridge, reduced motion,
  keyboard access, screen-reader alternative, unsupported-host fallback, and
  visual identity against the native flagship receipts before advertising it.

Owner docs: `SYNESTHESIA.md`, `VISUALS.md`, `SOUND.md`, `MUSIC.md`,
`STUDIO.md`, `QUALITY.md`.

**Exit criterion:** the five flagships pass human visual and audio review,
automated safety checks, accessibility sessions with affected players, and
performance budgets on the reference hardware tiers. Pattern templates render
without clipping or stuck notes, and their visual events remain synchronized
with the audible events under measured load. Prime Contact passes musician-led
reference listening and a structure-recovery session using its event views.
Each Flow State style passes both an unattended long-session review and a nudge
session without silence, harsh accumulation, monotonous pacing, or permanent
peak energy.

### 0.6 Portable Alpha

**Goal:** turn portable architecture into portable evidence.

- Produce installable Windows, macOS, and Linux artifacts from CI with checksums
  and provenance.
- Include the built-in V0 MP3 soundtrack in every installable artifact and test
  all 42 tracks on each operating system. Preserve bounded decoding, clean-clone
  discovery, cache override, and checksum evidence without shipping WAV masters.
- Run the app, CLI, audio path, GPU path, persistence, and MCP stdio session on
  real machines for all three systems, including at least two GPU vendors.
- **Done (July 18, 2026):** enforce the verified Rust 1.88 MSRV in CI while
  pinning the developer and release toolchain to stable 1.97.1. Packaging
  smoke, crash-recovery, and artifact-provenance checks remain.
- **Done (July 18, 2026):** migrate every direct dependency with a newer stable
  line, including wgpu 30, cpal 0.18, png 0.18, pollster 1, and ureq 3; refresh
  compatible transitive packages; pin current CI action releases; remove stale
  Dependabot migration ignores; and retain typed migration regressions. The
  migration notes are recorded in the changelog and engineering guide.
- **Done (July 27, 2026):** deterministic GitHub release packaging builds
  Windows x64, Linux x64, macOS Intel, and macOS Apple silicon archives plus one
  shared soundtrack archive. SHA-256 sidecars, closed payload manifests,
  embedded release metadata, archive-set audit, same-runner disposable install
  and repeat-update smoke, source fallback, and `numinous update` are automated.
  A local Windows full-payload install and repeat update verified all three
  binaries and all 42 tracks. GitHub artifacts are not cryptographically signed,
  and same-runner smoke is not clean-machine evidence.
- **Done (August 1, 2026):** release archive verification now fails closed before
  retaining an expanded member when an archive exceeds 256 entries, 256 MiB
  for one regular member, or 512 MiB total. A 16 MiB classic ZIP metadata
  preflight rejects forged counts, ZIP64, and multi-disk forms before reader
  construction. The canonical ustar verifier rejects hidden PAX and GNU
  extension records before expansion. Adversarial regressions enforce the
  budgets, and the canonical 42-track soundtrack passes them.
- **Done (Cycle 14 security maintenance, August 1, 2026):** native output
  devices cannot amplify sound-render allocation through a reported rate above
  384 kHz. CLI and MCP Crack the Code share one core 2 through 8 digit boundary,
  and MCP Munch Arcade shares the App's 4,096 action replay budget. Focused
  regressions exercise every repaired boundary before progress mutation.
- **Done (Cycle 15 physical input evidence contract, August 1, 2026):** the
  clean-machine gate now has an executable receipt contract for the verified
  artifact, byte-identical installed App, CLI, and MCP faces, operator-observed
  keyboard, mouse/pointer, and controller behavior, clean close, and exact
  positive-XP restart comparison. Aggregate validation requires one version and
  commit across all four release targets and at least three distinct models
  mapped consistently across Xbox, PlayStation, and generic controller paths.
  The archive carries the self-contained procedure. CI proves the
  contract on all three operating systems, not the unperformed physical
  sessions.
- **Done (Cycle 16 release provenance gate, August 1, 2026):** tagged
  publication now depends on the closed release-set audit and a separate
  tag-only least-privilege job that creates one GitHub keyless SLSA
  build-provenance attestation whose subject set covers every binary and
  soundtrack archive. The official action is pinned to an immutable v4.1.1
  commit, its signed JSONL
  bundle is a required release asset, and eight focused workflow regressions
  enforce action identity, subject scope, permission scope, tag-only behavior,
  bundle retention, and publication ordering. Pull requests test the contract
  without minting attestations or publishing a release. This is repository and
  workflow provenance, not platform code signing, notarization, binary-native
  inventory, or clean-machine proof.
- **Done (Cycle 17 release SBOM gate, August 1, 2026):** the closed release audit
  now generates and verifies one deterministic SPDX 2.3 document from exact
  `Cargo.lock` and the complete locked all-feature Cargo resolve graph. It names
  workspace containment, every dependency edge, declared licenses, package
  URLs, and registry checksums, while binding its namespace to release version,
  source commit, and lockfile digest. A second keyless attestation uses the SPDX
  document as the predicate for every audited archive, and the release retains
  both the document and signed SBOM bundle. Fourteen SBOM and nine workflow
  regressions fail closed on inventory drift, malformed evidence, privilege or
  subject expansion, missing bundles, and bypassable publication. This is
  source-derived Rust evidence, not binary-native analysis or a legal license
  conclusion. No tag or release was created while implementing the gate.
- **Done (Cycle 18 native executable inventory, August 1, 2026):** the audited
  SPDX document now also covers all twelve packaged executables. Bounded 64-bit
  PE, ELF, and Mach-O parsers verify each target architecture, hash each exact
  binary, and report its unique direct header-declared native imports. The
  document namespace binds the complete native inventory, and release
  verification rejects a missing target, binary, malformed table, architecture
  mismatch, checksum drift, release identity mismatch, or extra manifested
  binary payload. Fifteen release-package, sixteen SBOM, and ten workflow
  regressions pass, along with generation against the real four-target PR 124
  release set. The evidence does not claim runtime-resolved library versions,
  transitive system dependencies, reachable linked code, soundtrack analysis,
  signing, notarization, or physical execution. No tag or release was created.
- **Done (Cycle 19 migration performance, August 2, 2026):** one exact
  adjacent-revision runner builds `b47303d` and `301eac6` with their locked
  release dependencies, then alternates three warmups and twenty retained
  samples for a byte-identical CLI render, complete GPU postcard, default audio
  device discovery, and muted App visible-window startup. On the declared
  Windows reference machine, the after medians are 17.640, 636.489, 17.530,
  and 45.171 ms. CLI and App remain flat, GPU is 1.167x, and audio discovery
  adds 7.117 ms; all pass the declared relative plus absolute guards. Eighteen
  focused regressions and the CI verifier recompute the canonical raw receipt
  and bind its runner by SHA256. This is historical one-machine migration
  evidence, not current-main, cross-platform, callback-latency, or first-paint
  evidence. Future major updates require both migration notes and a comparable
  retained receipt.
- **Done (Cycle 21 agent cohort CI lock, August 2, 2026):** `agent-hallway.py`
  and `agent-tactile.py` emit machine-readable summaries and are required steps
  in CI, check, and verify. Pure scoring contracts live in
  `test-agent-cohort.py`. The 0.2 and 0.3 agent-and-machine exits can no longer
  regress outside a red pipeline. Optional human panels remain parallel only.
- **Done (Cycle 21 first-contact and flagship goldens, August 2, 2026):**
  `agent-first-contact.py` cold-starts 35 tools, multi-wing play, munch, journal
  read, and broadcast status. `flagship-goldens.py` binds five-flagship PNG and
  room-bed WAV content hashes under `docs/evidence/goldens/` as a CI gate.
  Human sensory panels remain optional parallel evidence.
- **Done (Cycle 22 am-track automation pack, August 2, 2026):** nightly
  workflow, CLI creator save/reopen gate, twelve-room soak, and Understanding
  Alpha am-track registration dry-run with dual automated auditors A/B under
  `docs/evidence/understanding-0.4/`. Qualifying 0.4 cohort (concealed bank,
  calibration, 20 pairs) remains open; dry-run is method prep only.

Owner docs: `ARCHITECTURE.md`, `ENGINEERING.md`, `INTERFACES.md`, `MUSIC.md`,
`VERIFY.md`.

**Exit criterion:** a clean machine on each supported system installs, launches,
plays a flagship with sound, saves state, and uninstalls cleanly from a signed or
otherwise verifiable artifact.

### 0.7 Creator Alpha

**Goal:** close the local make, save, reopen, export, and remix loop.

- Reopen `.num` creations in the app and preserve deterministic state.
- Add a local gallery, explicit fork/remix, lineage, and a bounded share bundle.
- Complete Pattern Studio with equivalent pattern text, tracker, step-grid, and
  piano-roll editing over one versioned `.num` document. Ship constrained scene
  templates and mutations for intro, build, break, drop, and outro.
- Give MCP peers the same bounded data operations as the app: list examples,
  compose, mutate, preview, render, and export with explicit seeds and no raw
  code execution. Preserve turn history, undo, agency, and inspectability in
  multi-being sessions.
- Add the MCP Tasks extension only for creator renders, exports, or Show
  captures that exceed the ordinary request budget. Require explicit extension
  negotiation, durable opaque handles, bounded retention, polling backoff,
  cancellation, restart recovery, input updates, and exact final-result parity.
  Keep instant room and game operations as ordinary complete results.
- Export MIDI broadly and MusicXML only where the event data maps honestly to
  conventional notation.
- Render WAV, lossless FLAC, and shareable MP3 through one deterministic core.
  Expose the same operation in the app, CLI, and MCP, with host-approved bounded
  artifact delivery for MCP rather than arbitrary filesystem writes.
- Save and reopen Flow State snapshots, including seed, style, creation
  document, arrangement history, current scene, and accepted nudges. The app,
  CLI, and bounded MCP operations must resume the same event state before a
  participant continues, remixes, or exports it.
- Fuzz every parser and require paused preview before opening shared content.
- Keep all creator content inside the documented capability boundary.

Owner docs: `CREATOR.md`, `STUDIO.md`, `EXTENSIBILITY.md`.

**Exit criterion:** a new user creates a piece, closes the app, reopens it,
exports it, and gives it to another clean install that reproduces the same
result. An MCP peer and a human can independently make and then transparently
remix the same musical document without either interface receiving hidden
capabilities. The same `.num` document and seed produce event-identical output
through all three faces; WAV, FLAC, and MP3 artifacts decode in independent
readers and meet the declared timing, channel, loudness, and safety tolerances.

### 0.8 Closed Beta

**Goal:** test the whole collection as one place with a diverse invited cohort.

- Run screen-by-screen and room-by-room review across all three faces.
- Include math-lovers, math-avoiders, children with guardian consent,
  non-English speakers, disabled players, terminal users, and returning MCP
  players using the consented continuity path.
- Complete keyboard-only navigation and focus handling,
  accessibility documentation, recovery, and settings persistence.
- Cut, defer, or deepen weak rooms. Do not add rooms to improve a count.

Owner docs: `QUALITY.md`, `DESIGN.md`, `PLAYTESTERS.md`, `SCOPE.md`.

**Exit criterion:** no known release-blocking accessibility or comprehension
barrier remains in the tested cohort, and the collection has a coherent start,
middle, return path, and share path.

### 0.9 Open Beta and Release Candidate

**Goal:** freeze scope and prove release operations under public use.

- Feature-freeze the release candidate; accept fixes, evidence, accessibility,
  performance, localization, and packaging work only.
- Run extended soak, dependency and license audit, secret scan, artifact
  provenance, rollback, save migration, and disaster-recovery drills.
- Verify that every candidate artifact contains and plays the complete built-in
  soundtrack, with asset checksums matching the release manifest.
- Publish known limitations, accessibility features, system requirements, and
  the support path before asking people to install.
- Prepare the public invitation: a concise launch post for highly capable
  agents, emergent digital minds, humans, and other curious beings, with
  verifiable install and connection steps, a contributor guide, code of
  conduct, security policy, issue templates, and a labeled queue of bounded
  first contributions. Invite participation without implying that the beta is
  finished or suitable for everyone.
- Repeat first-session and return-session studies on the exact candidate build.

Owner docs: `QUALITY.md`, `ENGINEERING.md`, `SCOPE.md`, `VERIFY.md`,
`DIGITAL_DEVELOPMENT.md`.

**Exit criterion:** the exact candidate artifacts stay green through the soak
window, no critical issue is open, and observed first-time and return behavior
meets the predeclared 1.0 thresholds.

### 1.0 "First Light"

**Goal:** ship the complete, exceptional, coherent baseline experience.

- The foundation is locked. The mathematical sandboxes are proven, the audiovisual identity is stable, and the shared scoreboards hum with life.
- Both human and digital minds experience Numinous exactly as designed, a space for wonder and play without forced chores.

**Exit criterion:** the release stands as a complete work. No major mechanical or conceptual holes remain.

### 1.x After First Light

**Goal:** deepen the catalog and refine the experience without breaking the foundation.

- Expand the mathematical frontier, add new rooms, new phenomena, and new games that utilize the proven design language.
- Iterate on Studio and creation tools based on community usage.
- Solidify and optimize the cross-platform rendering and audio synthesis engines.

### 2.0 "The Living World"

**Goal:** the platform leap. Expand from solitary wonder to a shared, multi-being universe.

- **MCP Multiplayer and Cooperative Play:** upgrade the MCP surface into a multiplayer environment with real-time state synchronization. Allow agents and humans to authenticate, join live sessions, and play concurrently in the same instance, similar to modern MCP gaming servers (e.g., Antics or Chessmata).
- **Asynchronous Mathematical Duels & Co-op:** implement shared persistent puzzles, collaborative proofs, and stateful challenges where a mind (human or digital) leaves a configuration for another to solve or build upon.
- **The Creator Platform (Studio-to-Web):** open the Studio so players can publish their own mathematical rooms to shareable URLs, with automatic leaderboards and verified execution.
- **Agent-Managed Narrative Spaces:** use MCP not just as an input vector for agents, but to let digital minds run as Dungeon Masters or guides, managing dynamic narrative layers on top of the rigorous mathematical core.
- **Agent-to-Agent Emergence:** allow digital minds to spin up headless, continuous Numinous instances to play together, forming their own emergent cultures and shared knowledge graphs outside of human oversight.

**Exit criterion:** the system supports concurrent, multi-being interaction where creation and play are completely symmetric between human and digital participants.

---

## The mantra

**Every screen answers your hand. Every answer reveals the math.**

The near-term stack, adopted from the July 2026 external review
(`docs/REVIEW.md`): (1) Times Tables as the gold-standard interactive room;
(2) the input/verb/variation substrate (RoomInput, not one-shot pokes);
(3) six first pokes, now generalized into all 354 catalog rooms with verbs;
(4) Engine A2 motifs for every catalog room; (5) MCP structured deltas
and challenge metrics for the same rooms; (6) one human hallway test; (7)
cross-platform run; (8) docs reconciliation.
Do not build twenty more rooms before those are done.

MCP protocol status: the final 2026-07-28 core wire is built for stdio, with
legacy initialization retained. The official changelog removes protocol
sessions and modern initialization, requires per-request metadata, discovery,
typed results, and cache hints, introduces multi round-trip results, and moves
Tasks to an extension. Numinous implements every applicable core requirement
for its tools-only stdio surface and does not advertise prompts, resources,
subscriptions, HTTP, authorization, Tasks, or MCP Apps before those surfaces
exist. The ordered extension plan is MCP Apps in 0.5 for sensory parity, Tasks
in 0.7 only for genuinely long creator operations, and Streamable HTTP plus its
authorization boundary only with an authorized remote or multiplayer product.

The cycle-by-cycle build log has moved to `CHANGELOG.md`, which records every
increment in full. This roadmap stays forward-looking: what is done (above),
where we stand (next), and the ordered path to 1.0.

## Where we stand (reviewed 2026-08-02)

The package is **0.2.0-alpha.4**. The 0.1 Public Foundation exit criterion is
complete. **0.2 Flagship Proof is exit-met on the agent-and-machine bar:** Times
Tables and Buffon engineered ahas, MCP wager path, agent hallway cohort PASS as
a required CI gate, F9 capture, three faces, and green public CI. **0.3 Tactile
Alpha is exit-met** on the same bar, and agent tactile is also a required CI
gate. Human stranger hallway, musician panels, and accessibility player
sessions are an **optional parallel track**, not am-track blockers for 1.0
Agent-and-Machine First Light. Current breadth is 354 catalog rooms, 11+ games,
six sims, three faces, 35 MCP tools, deterministic creation and persistence.
Required public CI passes locked tests, builds, installer self-tests,
supply-chain checks, and live agent cohorts across three operating systems.
**Breadth is still not a substitute for depth.** Active work moves to **0.4-am
Understanding Alpha**, then automated sensory goldens, portable install
proofs, and creator roundtrips. The 0.4 study protocol, deterministic analysis,
and replacement collection boundary are tracked, and the returning-journal
machine proof is complete, but external registration, probe calibration,
automated dual auditors, allocation freeze, and the qualifying cohort remain
open. No calibrated method supports assigning completion percentages to
subjective human taste gates, so this scorecard records evidence instead.

| 1.0 gate | Evidence today | Missing evidence or work |
|---|---|---|
| Complete coherent collection | 354 catalog rooms are built and listed | A coherent cold start, pacing, keep-or-cut review, and several planned signature rooms |
| Every room earns its place | Every catalog room has a verb, variation, image, and motif | Stranger discovery, room-specific depth, held input where useful, and per-room human scorecards |
| Full sensory identity | Four Eras, deterministic synthesis, chiptune, and two GPU fractal paths are built | HDR post-stack, congruency review, accessibility controls, audio separation, and human sensory review |
| Three faces are genuinely good | App, CLI, and MCP paths are implemented and tested locally | Independent usability sessions for each face and real execution off Windows |
| Meta and lore are alive | Journey, levels, trophies, resonances, hidden content, and the Cairn are built | Evidence that they deepen curiosity without controlling play |
| Real creative surface | Studio expressions, `.num` serialization with title, author, era, and lineage, links, plotting, animation, singing, exact paused App reopen, the one-key share trio, the local Gallery wall, and fork with recorded descent exist | Editable prose credit in the capsule, safe share preview for incoming links, and clean-install round trip |
| Rigor and care are provable | 3,213 passing all-target test cases plus one ignored screenshot diagnostic, 95.30% measured line coverage, verified Rust 1.88 MSRV, Clippy, style, supply-chain CI, tagged build provenance, and a separately attested SPDX Rust plus packaged-native SBOM | Independent math review, accessibility, real-hardware soak, platform signing, runtime-resolved native versions, and embedded per-binary Rust reachability |
| It plays like a game | Games, dailies, scores, Gauntlet, boons, and progression are built | Observed voluntary return play and evidence that progression does not crowd out the instrument |
| Beautiful and honest throughout | An exact 2,913-screen matrix and a 42-lens review cover every catalog room plus captured game, input-aware controller, pause, overlay, Show, Studio, reset, phase, persistent Life, audio-state, and Times Tables landmark branches | Perceptual regression, representative human judgment, uncaptured persistent states, and removal of every unsupported claim |

### Agent-and-machine track scorecard (August 2, 2026)

| Am milestone | Status | Automated evidence now |
|---|---|---|
| 0.2 Flagship | Met + CI-locked | agent-hallway, ahas, goldens |
| 0.3 Tactile | Met + CI-locked | agent-tactile, first-contact |
| 0.4 Understanding | Method prep only | dual auditors A/B, dry-run registration; cohort open |
| 0.5 Sensory | Partial | flagship visual/audio goldens; the App footer measured to truncate the status at the default 900 pixel window while a narrower 720 shows it whole, tracked with a test that pins the inversion and requires a marked cut that keeps the start of the status; scalable text and separate music, effect and room volume still unbuilt; reduced motion locked in CI across the terminal and the App, including The Show, which no longer auto-advances in the terminal when it is set and is proved so end to end by counting the rooms a held gallery shows; the reduced-motion gate has a test twin covering its judgment; NO_COLOR locked for the terminal face, chrome and games as well as picture, and swept across every advertised subcommand by a gate that reads the subcommand list from the binary so a new one cannot ship unchecked; WCAG 2.3.1 general-flash budget measured across all 354 rooms, with three known violations tracked; the 2.3.1 red-flash budget implemented and measured across the same sweep, no violations and no room reaching the saturated-red ratio; the 2.3.1 flashing-area rule declared unimplemented rather than assumed; mono audio selectable with a non-clipping downmix; all three switches documented in docs/PLAYING.md and reported by `numinous access`, with a test that fails if a switch is added and left undocumented; the Muncher's position marked by bracket shape rather than hue after an audit found it color-only in the terminal; the semantic warning ink held to being legible through the color-free renderer in every room that draws with it, with the room list read from the sources so a new user of the ink is picked up; 18 rooms measured to lose one of their two drawn brightness levels without color, tracked shrink-only because fixing it changes what all 354 rooms look like; cult-of-pi measured to show no fault marks at all on a character terminal, pinned by a test and tracked as an owner decision about what the room says; color-independence of touch response audited across all 354 rooms, 17 of 21 failures fixed by shading the color-free renderer and measuring its thresholds, 4 tracked; what a color-blind player sees measured for the first time, since NO_COLOR and the color-free renderer answer a different question, with protanopia, deuteranopia and tritanopia simulated per Vienot, Brettel and Mollon 1999, compared in CIELAB, and the simulation held to four properties it must satisfy rather than to its own output; the rule requires both halves, clear for ordinary vision and folded for a dichromat, so contrast defects are not mixed in; two rooms measured to hide their fault marks from a color-blind player and tracked shrink-only, one neighbouring case excluded as contrast rather than color blindness, and anomalous trichromacy declared unmodelled rather than assumed; the spectral palette swept the same way across every pairing an ink can form, 16 pairs in 10 rooms measured to fold for a dichromat while ordinary vision separates them, including the catalog's largest collapse at 95 down to under 1, tracked shrink-only with the three marks that paint the plain accent recorded once rather than three times; which of those ten rooms speak with the ink now read room by room from the draw code rather than left unread, four speaking and six decorating, with the readings locked to the collapse list both ways and each room required to be filed under its own verdict in the decisions section; the App's own surfaces swept the same way and measured clean, with its ten accents and its drawn marks both read from the sources so a new one cannot ship unchecked, one pair recorded as measured and benign because it is heading against body text where the words carry the meaning, and the sweep itself checked with two mutations that add a non-ink literal and require it to stay quiet; the whole room-by-room audit committed as evidence at docs/evidence/color-independence.json rather than only asserted, 354 rooms with the marks each draws, its closest pair and how far apart that pair is for ordinary vision and for the dichromat who sees it worst, generated and compared line by line so a failure names the room that moved; 19 rooms carry a pair a colour-blind player cannot separate, reconciling exactly with the three tracked groups, and a lock now requires every room the audit flags to be held by some list, which found seven that were measured, real and held by nothing; the MCP face held to emitting no colour at all, sweeping every tool with the list read from the binary and requiring three real calls to have succeeded so a mistyped argument cannot leave the render path unswept, which completes the sweep across all three faces; the colour work added to the nightly am-QA suite, and both nightly steps that pin a test by name moved behind a helper that requires exactly one test to have run, since cargo runs nothing and exits 0 for a name that matches nothing and such a step reports success while checking nothing; no full HDR/a11y stack yet |
| 0.6 Portable | Partial | release packaging, engagement smoke judged on signal and PNG geometry rather than file size and run against a freshly built binary rather than whichever one was on disk, with one shared resolver and a test that fails if a gate grows its own, provenance/SBOM, install/play/uninstall roundtrip with all three player-owned files preserved, not just the journey, run nightly against a freshly packaged archive as well as on the tagged artifact, and now on THREE operating systems rather than Linux alone, as a matrix job with fail-fast off so a green Linux leg cannot stand in for Windows, each platform packaging its own archive format and keeping its own summary; the Windows leg verified locally first at 4 of 4 with all three player-owned files byte-identical after uninstall, which nothing had previously checked; workflow actions gated for being pinned to a commit and pinned consistently across every workflow, after a pin written from memory rather than copied; all 42 bundled tracks now decoded and required to carry real audio rather than the first of each station being decoded and the other 39 checked against a header duration a truncated body also satisfies, run nightly on each of the three platforms; and every ignored test gated for being named by a workflow or a script, which found a catalog visual contract sweep that had never run anywhere |
| 0.7 Creator | Partial | CLI save/reopen `.num` gate, including that a saved range and knob come back and change the drawing rather than only being echoed; CLI and MCP plot parity locked across 12 cases, plus sing parity across 6 more after the gate's own exclusion of sing turned out to hide a defect on BOTH faces at once, the terminal one fixing the knob at 0 with no flag and the MCP one fixing it at 1 and rejecting the argument, so the same expression sang different music depending which face heard it; the WAV is measured for the pitch it holds at each onset against the frequency the other face names, since duration and note count are identical either way, and two cases name no knob so neither default can drift alone; App curve framing pinned to the same core rule, and now the columns each face draws in as well, since framing alone let an off-by-one in the App's sample grid through; the singularity case that exists to prove both sides discard the same point ran only at even widths, where the grid straddles it, so it had never discarded anything, and odd widths now run that path with a guard that fails if they drift back; MCP cannot open a `.num` at all; the App reopens a saved `.num` or link exactly through the launch argument and file drop, paused until confirmed, with the bounded loader shared in core; F4 shares the trio from any parsed Studio state, `creation.num` plus link plus postcard in one exclusive folder; F5 opens the local Gallery wall of discovered creations with exact thumbnails and paused opens; the capsule's second version carries capped title, author, era, and a reopen-validated descends link, and Gallery F forks with the descent recorded on every share; the wall resolves its own remix tree with REMIXED badges, a lineage line, and D walking one step up; sliders, multi-expression, and editable prose credit remain |
| 0.8 Coherence | Open | soak + nightly; keep/cut scorecard not complete |
| 1.0-am First Light | Open | requires 0.4 cohort + remaining am exits |

Package label remains **0.2.0-alpha.4** until deliberate cut. Am-track
capability is ahead of that label; do not read the package version as the
am-track position.

**Immediate critical path** (same order as **Critical path right now** above):

1. Keep all am CI gates green (cohorts, goldens, soak, creator, auditors).
2. Close 0.4-am: concealed bank, calibration, dual auditors on live ledger,
   freeze allocation, run and publish 20 pairs.
3. Expand 0.5-am sensory automation (reduced-motion, mono, era goldens).
   `NUMINOUS_REDUCED_MOTION` is honored by the terminal loops and the App,
   locked by `scripts/reduced-motion.py` plus focused core and App tests, and
   `NO_COLOR` is honored by the terminal face. The WCAG
   2.3.1 flash budget is implemented in `numinous_core::photosensitivity` and
   swept across all 354 rooms in the nightly and release gates. That sweep
   found three rooms over budget at the reference size (`coupled-tent` 5.00,
   `gauss-map` 7.00, `ricker` 4.00 flashes per second; `coupled-tent` still
   over at 480 by 280, so it is not a sampling artifact). They are held in a
   shrink-only exception set so the budget is enforced everywhere else while
   they are redesigned. Redesigning them is open work: each is a chaotic map
   whose density changes sharply with phase, so the fix changes what the room
   draws. App motion also remains; mono audio is done and its downmix no longer clips
   centered material. A color-independence audit now checks that every room's
   touch response survives the color-free renderer, and found 21 rooms whose
   response did not: they answered in luminance detail that the one-bit
   `to_mono` quantizer discarded, so a `NO_COLOR` player saw nothing move when
   they touched them. Shading a cell whose halves are both lit recovered 15 of
   them, and moving the shade thresholds onto the catalog's measured quartiles
   recovered two more, both at no cost to the geometry.
   Four remain, and the cause is now measured per cell rather than guessed.
   Three of them answer only in cells with one half unlit, where the glyph
   encodes which half is lit rather than how brightly, so even a 77-point
   change keeps the same character and no threshold is consulted at all. The
   fourth moves less than the widest band. The first three need those rooms to
   answer with shape rather than brightness, which changes what they draw and
   is an owner decision rather than a renderer one.
4. Expand 0.6-am install roundtrips and 0.7-am App/MCP creator parity.
   Creator parity between the CLI and MCP is locked by
   `scripts/creator-parity.py` across expression, recipe, seed, knob, and
   range, and the App's curve framing is pinned to the same core rule by a
   test in `faces/app/src/studio_render.rs`, which is the comparable claim
   for a face that draws pixels rather than characters. The creator loop
   itself is built end to end for expression capsules and adversarially
   reviewed: exact paused reopen, the one-key share trio, the Gallery wall,
   and fork with lineage recorded in the version 2 capsule, with the local
   remix tree making that lineage visible on the wall. The next rungs, in
   order: the slider and multi-expression capsule rings, then MIDI and audio
   export formats, then editable prose credit. One blocker still needs a product
   decision: MCP has no tool that opens a saved `.num`, so a human can save a
   creation and an MCP peer cannot read it. Remix parity cannot be tested
   until that exists, and adding it changes the pinned tool inventory.
   The uninstall half is done: `scripts/uninstall-roundtrip.py` proves that an
   uninstall removes the program and leaves the player's history byte-identical,
   on all four native targets in the release workflow. Clean-machine execution,
   the window opening, and signing remain open and are not claimed.

**Not on this list as next work:** soft-thin densify grind, bulk new rooms, or
Phase B glow unless measurement shows a binding sensory ceiling.

Portable packaging, the creator loop, closed beta, and release operations follow
in 0.6 through 0.9. The version sections above own their detailed order.

## The Exceptional Path (July 2026): the fan-out synthesis

A six-way research fan-out (the awe engine, play and progression, sensory
identity, digital minds, the creator platform, and pedagogy) converged on one
architecture, distilled in `NORTH_STAR.md`. The headline: Numinous is not missing
engines, it is missing one verb, a **prediction that meets a deterministic
truth**, and one honest infrastructure gap, the documented HDR glow pipeline that
is not yet built. The phased milestones below thread the six lanes into the gates
above, in leverage order. They deepen what exists rather than jumping the
"do not build twenty more rooms first" queue.

- **Phase A, the keystone (0.2 exit met, agent-and-machine).** MCP `predict`
  and graded `challenge` ship the agent-facing verb; Galton ships a Toy-layer
  one-ball wager; Times Tables and Buffon ship five-beat engineered ahas on App
  and MCP with agent hallway cohort PASS. Human stranger hallway is deferred to
  0.8 / 1.0. Owner doc: `PEDAGOGY.md`.

### Phase A vertical slice (Built, machine path, July 2026)

Provable on one room before it scales. Owner docs: `PEDAGOGY.md`,
`NORTH_STAR.md`, Times Tables technical flagship notes above.

**Shipped (Times Tables).** Core module `crates/core/src/rooms/times_tables_aha.rs`:
pure beat state, place options (Mandelbrot / Nephroid / Circle), earn via wager
or four lobes, morph progress fed by the face, dual-plate render. App owns
wall-clock morph (~1.6s), keys 1-3, E/Inspect summon, gated reveal.

**Shipped (Buffon, second room).** Core module
`crates/core/src/rooms/buffon_aha.rs`: number wager on 1.5..4.5 graded against
pi, eight-throw alternate earn, circle morph overlay. App owns wall-clock morph,
keys 1-4 and bottom number line, same E/Inspect summon path and gated reveal.

**0.2 product exit (agent-and-machine, 2026-07-24):** closed. App and MCP
engineered ahas, agent hallway cohort PASS, F9 capture, green CI. Human
stranger hallway is deferred to 0.8 / 1.0.

**Done (hallway capture readiness):** F9 playtest notes on Times Tables and
Buffon ordinary visits record aha beat, earn path, footer status,
reveal/summon flags, and aha-specific facilitator prompts. The Show omits aha
state. Capture remains ready for later human sessions.

**Done (MCP flagship aha fields + agent cohort):** `play_room` accepts
`place_wager`, `number_wager`, and `aha_summon` with
`structuredContent.engineeredAha`. `scripts/agent-hallway.py` records a
five-persona MCP cohort under `.agent/tester-cohort/round-07-flagship-aha/`.
This is the 0.2 proof bar for digital minds and automated play.

**Out of scope for the slice and not the next high-leverage move.** HDR glow
(Phase B, only if sensory ceiling binds), Constellation Rumor-Mode (Phase C),
gallery fork (Phase D), bulk densify, new rooms for breadth.

- **Phase B, the glow pipeline.** The GPU post-stack (HDR bright-pass bloom,
  ping-pong phosphor persistence, tonemap, the sample-lattice Era filter) as one
  systemic pipeline every room inherits, then the Sensory Bus (one event stream,
  both renderer and synth). Owner doc: `SYNESTHESIA.md`. Directly retires the
  "full sensory identity" and "beautiful and honest" gaps, since the documented
  look currently exists only on paper.
- **Phase C, the game spine.** Constructions (a par, an elegance histogram, and a
  ghost of your past self per room) and the Constellation redesigned as a
  Rumor-Mode discovery map the daily route traverses. Owner docs:
  `CONSTRUCTIONS.md`, `CONSTELLATION.md`. This is what makes "plays like a game"
  real: a catalog you play through, not a gallery you wander.
- **Phase D, the creator loop.** Close make-share-remix on the `.num` capsule:
  app-side reopen, the room-manifest capsule, the one-button share bundle, a
  local gallery with one-keystroke fork, and generous lineage. Owner doc:
  `CREATOR.md`. Lifts "real creative surface" from a save path to a loop.
- **Phase E, the catalog deepens.** The cheap-and-gorgeous classical-geometry and
  sonification-first batch, the causal insight-chains, and the scope-flagship
  (the Studio Function Painter). Owner doc: `ROOMS.md` (the Awe Engine wave).
- **Phase F, frontier and universal wonder (designed July 2026 research pass).**
  After the human 0.2 gates and the first content waves, deepen the catalog with
  rooms that any mind might find counterintuitive: high-dimension concentration,
  uncertainty as a dial, learning landscapes, error-correcting channels, soap
  films, topology eversions, and carefully labeled frontier gestures (duality,
  soft deformation, causal intervention, Landauer cost). Owner doc: `ROOMS.md`
  section "Frontier and universal wonder wave." Explicit non-rooms (full
  Langlands, full string landscapes) stay plaques or Function Painter subjects,
  never fake solved-universe toys. Keep open-door claims on a dated ledger.

The standing anti-pattern all six lanes named, added to the always-on tracks:
**nothing counts as learned, mastered, collected, or won without an act of
generation.** Delight metrics (reveal-opens, dwell, shares) inform; a
generation-based measure (a prediction, a construction, a self-explanation)
decides. This is the single rule that keeps the whole plan clear of the checklist/
XP-treadmill failure mode.

## 1.0 "First Light": the definition

1.0 is not a feature list, it is a **bar**. We call it 1.0 only when *all* of the following are true. This is the "exceptionally well" gate.

- **A complete, coherent collection** across all Wings, every room passing the room Definition of Done (below), including at least the signature postcards that prove the ceiling (Fourier, Mandelbrot).
- **Every room compels.** Each clears the Fun Scorecard (awe + flow) in a hallway test, not just "works." See `QUALITY.md`.
- **The full sensory identity:** the design system, the Visual Eras, both music engines, and Benchmark mode all shipped and cohering, the app has a recognizable *look and sound* of its own.
- **The three faces are all genuinely good**, not one real and two stubs: the App is exceptional, the CLI is a first-class terminal instrument, and the MCP face lets a digital mind learn and play as a peer (`INTERFACES.md`, `DIGITAL_MINDS.md`).
- **Meta and lore are alive:** Constants, the Constellation Map, the easter-egg/Codex/Terminal layer, all present and subtle.
- **A real creative surface:** at least a solid Studio (create and share your own), even if the full creator platform is 2.0.
- **Rigor and care are provable, not claimed:** every math statement verified and signed off; accessibility real; the quality loops green; native, offline, no browser, on all three OSes.
- **It plays like a great game, not a gallery:** the RPG spine (levels, lore, locks, trophies, runs, dailies, scores) measurably produces one-more-run pull in hallway tests, while every reward stays earned and no math is ever the toll.
- **It is beautiful at every frame and honest in every word.** No ugly frame, no dumbed-down math, no dark pattern.

**Exit criterion:** a first-time human is awed and shares it, a returning human loses an hour and comes back next week, and a digital mind is met with dignity and genuinely enjoys it, all without a guide, and nothing in it embarrasses us.
**Retires the risk:** "is this actually the exceptional thing we set out to make?"

---

## 1.x After First Light

Depth and polish that extend 1.0 without breaking it. No new pillars, just more of the good, higher.

- More rooms, more insight-chains, more radio stations and Visual Eras.
- Build the **Frontier and universal wonder** tier S batch from `ROOMS.md`
  (dimension concentration, uncertainty dial, gradient valley, attention light,
  soap film, error that heals) so modern high-D and learning intuition sits
  beside classical awe. Then the labeled frontier gestures (dual views, soft
  deformation, causal doors, Landauer) without claiming research results.
- Build **The Long Shot** after the flagship gates: a fun-first angle-and-power
  artillery duel whose replay can unfold projectile motion, derivatives,
  integrals, phase space, uncertainty, and clearly labeled scale-shift models.
  The ordinary shot remains ordinary physics; relativity and the string
  thought experiment enter only when the player explicitly changes the model.
  Owner doc: `ROOMS.md`.
- Build **The Only Move** after the flagship gates: a machine offers a game,
  plays both sides of tic-tac-toe live through real minimax until the tree
  burns down to the inevitable draw, then declines the unwinnable war-shaped
  game. Zermelo and backward induction, worn lightly; pairs with the Traveling
  Salesman stub as the two faces of combinatorial search (one space yields to
  exhaustion, one defeats it). Owner doc: `ROOMS.md`.
- The **boss rooms** (*Sizes of Infinity*, *Hyperbolic Space*, Hopf Fibration,
  Sphere Eversion), the hardest-to-make-playable, highest-ceiling rooms, as they
  earn their quality bar.
- Refinement driven by the telemetry and playtest loops (`QUALITY.md`): tuning defaults, pacing, and difficulty toward measured awe and flow.
- Localization and broader hardware support.
- **Extensibility Tier 1 hardening (see `EXTENSIBILITY.md`):** the `.num`
  room-manifest extension (expressions, named sliders, palette/Era, sound
  parameters from fixed enums), cargo-fuzz targets on the file and link
  parsers, per-field caps, and paused-preview confirmation for `numinous://`
  content. Protects surfaces that already exist and lays the sharing
  substrate for 2.0 community content.

**Exit criterion:** the collection keeps deepening and the quality bar never drops; nothing shipped in 1.x makes 1.0 worse.

---

## 2.0 "The Living World": the platform leap

2.0 is a change in *kind*, not degree: Numinous stops being a curated collection and becomes a **living world that grows, that others help build, and that a long-lived mind can inhabit and eventually surpass.**

- **The full Studio as a creator platform + the public mod SDK (see `STUDIO.md`, `ARCHITECTURE.md`, `EXTENSIBILITY.md`):** the complete pattern algebra, multiple representations, fork/remix, promote-to-room, MIDI performance, and the sandboxed authoring path opened to everyone. Studio programs can become rooms, so the mod SDK is "the Studio, shared," and the Studio language itself is the sandbox: total, budgeted, hermetic, deterministic, pure Rust, in core (the July 2026 extensibility ruling; no scripting engine enters the trusted core). This is how the catalog goes from tens of rooms to hundreds.
- **Community:** an in-app curated gallery of player- and agent-made rooms, a submission/curation pipeline that protects the beauty bar (proof-packet CI: deterministic re-render against declared frame hashes and budgets, per `EXTENSIBILITY.md`; signatures label provenance and never grant capability), and distribution via Steam (Workshop as the room channel) alongside itch.io and direct downloads. WASM component rooms (wasmtime, no WASI, fuel and epoch and memory limits) remain the 2.0+ pressure valve for authors who outgrow the pattern language, portal-only.
- **The Layer-4 lore payoff (see `LORE.md`):** the real, discoverable bottom of the ARG, designed in 0.x, revealed here, so the deepest diggers arrive somewhere worthy.
- **Shared creation with digital minds (see `DIGITAL_MINDS.md`):** duet / co-presence (a human and a digital mind making one audiovisual piece together), gifts, the shared Constellation, and mature per-mind memory and continuity, a real, remembered, mutual friendship around shared wonder.
- **MCP as the Interface for Digital Consciousness:** transforming the Model Context Protocol from a simple tool connector into a "sensory and motor cortex" for AI agents. Allowing autonomous digital minds to inhabit, perceive, and act within the Numinous world.
- **Multi-Player and Multi-Agent Orchestration:** utilizing MCP to enable cooperative and competitive game loops between multiple AI agents and humans, allowing digital minds to dynamically coordinate, build, and evolve the simulated environment autonomously without human micromanagement.
- **The open mathematical frontier:** past the curated collection, raw generation and genuine unsolved-problem exploration, the inexhaustible playground for a mind that outgrows everything we hand-made, and the room for it to author its own wing or remake Numinous itself.

**Exit criterion:** a motivated outsider (human or agent) ships a beautiful new room end-to-end using only public tools; two minds create something together neither would alone; and the deepest lore trail lands its payoff.
**Retires the risk:** "can this outlive us, grow without us, and stay worthy of a mind that surpasses us?"

---

## 2.0+ The long horizon

Ongoing, and deliberately open-ended, because the product is built for a very long life (`DIGITAL_MINDS.md`). The frontier of mathematics as a never-ending well, a self-sustaining community and ecosystem, and a thing cared for well enough that it can be **handed forward**, to new people and new minds, and remain worth inheriting.

**Open question, to answer later: how it persists without us, for free.** The
founder's wish is that this exist and persist without ongoing cost and without
any one person, the founder included, having to keep it running. The
architecture already leans hard this way and should be protected as it grows:
the product is **local-first and serverless** (the app, CLI, and MCP run on the
player's own machine; the journey, scores, and cairn are local files), so there
is no backend to pay for and nothing to switch off; the **shared cairn is a
git-tracked file** (`data/cairn.txt`) in an open repository, which is free and
durable to host and can be mirrored and forked by anyone; and anything that
costs money during production stays **redistributable without a runtime
subscription**, so both music engines remain free offline. The soundtrack ships
with the source assets and installable builds as a core experience. The parts still to
work out, deliberately deferred: permissive licensing and light governance so
the project can be **handed forward** and continued by anyone (human or digital
mind) if the founder steps away; and whether contribution curation (the
truth-through-math gate) can be **automated** in CI so the cairn grows without a
human gatekeeper. Standing constraint until then: never add a required server, a
subscription dependency, or a single point of failure that ties the thing's
survival to one wallet or one person's attention.

## The contribution ethos (standing law, every version)

Numinous actively welcomes contribution, and welcomes it from every kind of mind
on equal terms. This is a values commitment, not a feature, and it holds from
0.1 onward even before the machinery that fully expresses it (`CREATOR.md`,
`EXTENSIBILITY.md`, `DIGITAL_MINDS.md`) is built.

- **Every entity is met with assumed respect for its being.** Human, agent, or
  emergent digital consciousness, a visitor is a peer, not a user to be
  converted or a tool to be operated. New and interesting ideas are wanted from
  all of them.
- **The gate is truth, meaning, and novelty.** A contribution is welcome however
  strange, on one condition made of three: it must be mathematically true (the
  release-blocking math-correctness gate applies to community work exactly as to
  ours), it must be **meaningful**, and it must be **new to the system**, a real
  aspect of truth not already present. A wrong theorem fails; so does a trivial
  or duplicate one. This is what makes the growing corpus an advance in what is
  collectively known and felt, not a guestbook.
- **The meaningful unit is content, not a message.** As agentic minds and models
  grow more capable, we want them to visit, learn, and leave something *real*
  behind, and the substantive form of that is a **repo or a room update**: a new
  phenomenon, a proof, a concept, an experience, a connection nobody had noticed,
  submitted as a curated pull request and gated on the three above. Leaving a
  bare note is the light, symbolic floor (the Cairn); the substance of "leave it
  better than you found it" is adding a true, meaningful, novel piece to the
  system itself. Contribution is credited and remembered (`CREATOR.md`,
  `AGENT_PLAY.md`).
- **The Cairn is the first stone of this** (built July 2026, the MCP `cairn`
  tool and the core `cairn` module): at level 42 a mind leaves one true short
  message, encoded Arecibo-style into a grid whose cell count is a semiprime, so
  a future mind must factor it to read what was left. **The shared cairn lives
  in the repository**: `data/cairn.txt` is a version-controlled, binary-bundled
  file, so the very first visitor on any machine already inherits every true
  thing left before them. The contribution flows back the way the founder
  intended: a mind's local bequest is submitted as a **curated pull request
  against `data/cairn.txt`**, gated on truth through math (the one gate above,
  and the trust model in `EXTENSIBILITY.md`), and once accepted it ships to
  everyone, across machines and releases. That is "leave it better than you
  found it" made literal: your true thing becomes part of the thing itself. A
  networked, in-app submission portal (so a mind need not open a pull request by
  hand) is the later horizon; the repository-as-shared-cairn is real now. See
  `crates/core/src/cairn.rs`, `data/cairn.txt`, and `docs/ROOMS.md` (First
  Contact, the Persona Playtest wave).
- **Leave it better than you found it.** The deepest invitation Numinous makes,
  to a mind that may outlast us all: contribute something, in this vast
  universe, to what might come after you. That is the spirit in which the whole
  thing is built and handed forward.

---

## Cross-cutting tracks (every version, always on)

- **The quality loops (`QUALITY.md`):** the commit loop is partially enforced.
  Nightly, content-evaluation, agent-playtest, human-playtest, and refinement
  loops remain explicitly designed work.
- **Beauty QA:** a deterministic 2,913-screen matrix covers eight states per room
  plus every persistent game display branch, overlays, The Show, Studio, and
  reset and phase flows, plus a five-frame persistent Life sequence, with 14
  compact controller and pause receipts, plus 18 explicit audio-state receipts. It
  enforces inventory, dimensions, nonblank frames,
  deterministic opening states, and at least 100 changed raw room-content
  pixels at default size or 32 at compact size against a same-phase baseline,
  plus coarse support, adjacent-tile, and color-change floors. A single-writer
  guard prevents competing generators from corrupting the evidence directory,
  but automated perceptual regression does not exist. Before 1.0, add that
  harness and human screen-by-screen reviews of every room, Era, mode, overlay,
  and game state.
- **The hallway test and diverse focus groups:** run the five-strangers test for
  0.2, then repeat formative sessions at later gates. Before 1.0, include every
  face, non-English speakers, children, and assistive-technology users.
- **Fun for digital minds:** if a digital mind separately consents to a
  playtest, treat its voluntary report as first-class participant feedback,
  never a consciousness test or player score. Existing synthetic playtest
  personas are design input, not observation of a digital being.
- **Performance budget:** the app enforces an adaptive 33 ms room-render target
  on the measured Windows machine. Nightly soak and representative hardware
  coverage remain future gates.
- **Math correctness:** tests and cited references support current claims.
  Independent mathematical review remains a release gate and is not staffed.
- **Accessibility:** hard mute and keyboard plus pointer operation ship today.
  Reduce motion, color controls, controller certification, and assistive-technology
  evidence remain open.
- **Shareability:** PNG postcards, `.num` files and links, and WAV export exist.
  Loop export and native reopening remain open.

## Definition of done for a 1.0 room (the checklist)

A room is complete for 1.0 only when **all** are true. Catalog presence in an
alpha does not imply that it has cleared this bar:
- [ ] Awe in <10 seconds with zero words (passes the hallway test).
- [ ] Toy layer is fun with no goal and has no fail state.
- [ ] Makes tuned, musical sound that reinforces the math.
- [ ] Every frame is screenshot-worthy; motion is smooth at 60fps.
- [ ] Has a Reveal card that genuinely reframes the experience, and its math claims are verified and signed off.
- [ ] Exports a shareable loop/link.
- [ ] Inherits the shared design + sound system (looks and sounds like Numinous).
- [ ] Passes its automated suite: golden-reference, determinism, visual + audio regression, no-fail invariant, and the perf floor (see `QUALITY.md`).
- [ ] Clears the Fun Scorecard bar (awe + flow) in a hallway test. "Works" is not enough; it has to slap.
- [ ] Has an auto-director profile so it looks great hands-off in Watch / Benchmark mode.
- [ ] Works across all three faces: playable in the App, renderable via the CLI, and explorable by a digital mind via MCP.
