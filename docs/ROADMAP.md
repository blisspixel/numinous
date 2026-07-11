# Roadmap

A version-gated plan from empty repo to a living world. Each milestone has a **goal**, concrete **deliverables**, an explicit **exit criterion** (how we know it is done), and the **risk it retires**.

## How we version (read this first)

- **We ship by quality gates, not calendars.** There are deliberately **no time estimates** in this document. A milestone is done when it clears its bar, not when a date arrives. "How long will this take" is the wrong question; "is it exceptional yet" is the right one.
- **Versions are defined by what is true, not when.** Each version below is a *state the product has reached*, a set of things that are real and hold their quality bar, not a sprint.
- **1.0 is a high bar, not a minimum viable product.** Because the whole point is to do this *exceptionally well*, 1.0 means "a complete, coherent, genuinely exceptional experience," not "the least we could ship." The MVP-shaped thinking lives in the 0.x line; 1.0 is where it becomes worthy of the name.
- **Guiding rule, at every version: feel before features.** We build depth-first. One unforgettable thing beats ten mediocre ones. A pretty menu of boring toys is failure.

## The version map at a glance

- **0.1 Foundations** the spike. The stack, the headless core, all three faces in skeleton, the test loops. Internal.
- **0.2 The Vertical Slice** one flagship room to jaw-dropping quality. Proves the feel. Internal / friends.
- **0.3 to 0.9 The build-out** every 1.0 workstream comes online and matures, through private alpha to open beta.
- **1.0 "First Light"** a complete, exceptional, coherent experience. The real first release.
- **1.x After First Light** depth and refinement without breaking what 1.0 established.
- **2.0 "The Living World"** the platform leap: the full Studio as a creator platform, community, the deep lore payoff, shared creation with digital minds, and the open mathematical frontier.
- **2.0+ The long horizon** the frontier and the ecosystem, built to outlast us and be handed forward.

---

## Progress (updated as we build; see CHANGELOG.md for detail)

- **Done:** the headless core (`Room` trait with `reveal()`, deterministic ASCII `Canvas`, seeded RNG, registry, `verb`, `render_poked`, and variation); the CLI face (`numinous`), the MCP face (`numinous-mcp`), and the windowed app; **30 catalog rooms across 10 wings** plus hidden content; 6 lever-driven sims; 11+ games; the full engineering harness (edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, house-style guard, an 80% line coverage gate, three-OS CI). Current local evidence: fmt, clippy, 918 tests, locked build, Windows verify, 90.79% region cover, and 90.32% line cover all green.
- **Done (GPU and audio hello-world):** an adaptive `wgpu` context (`crates/gpu`) that picks the machine's GPU across Vulkan/Metal/DX12 with a CPU fallback, rendering the Mandelbrot set offscreen to a PNG; and adaptive `cpal` audio (`crates/audio`) on the system default device that plays a tone and writes a WAV. Both verified on the dev laptop (AMD Radeon 780M, Realtek at 48 kHz).
- **Done (rooms as images):** a `Surface` abstraction so every room renders through one `render` method to the ASCII `Canvas` and to an RGBA `Raster`; `numinous render <room> --out image.png` writes a real glowing image on the CPU (verified on the dev laptop).
- **Done (windowed app):** `faces/app` (`numinous-app`, winit + softbuffer) opens a real resizable window showing a room animating in full color, with keyboard room-switching. The start of the GUI Cabinet; verified launching on the dev laptop.
- **Done (sound):** every room describes its own sound (`SoundSpec` + `Room::sound`); `numinous sonify <room> --out file.wav` and `numinous play <room>` (live animated terminal).
- **Done (the 0.2 vertical slice, in substance):** the windowed app is a full experience: live per-room sound, mouse scrubbing, an on-screen HUD with reveals, The Show (lean-back auto-play of the whole collection), the Studio in the window (type math, watch and hear it live), and GPU real-time fractals (a persistent `wgpu` pipeline drives the Mandelbrot deep zoom and the morphing Julia at window resolution, with CPU fallback; verified on the dev laptop's Radeon 780M).
- **Done (content and play):** 30 catalog rooms across 10 wings plus unlisted hidden content, including the Change wing (The Pour, Slope Rider), Fourier Epicycles, the double pendulum, the random walk, Voronoi, Quine, Strange Loop, L-System Garden, Mandelbrot/Julia dives, Galton, Buffon, etc.; 6 lever-driven sims; 7+ games (SETI, Talk to the Aliens, Guess the Shape, Crack the Code, Munch, Nim with the xor secret, the Gauntlet run, and full Munch Arcade) with daily seeds and dense feedback; the Studio expression engine (`plot`, `plot --save`, `open-studio`, `--animate`, `sing`, and live in the window); Visual Eras (phosphor, 8-bit, vector, modern) across app, terminal, and PNGs; truecolor terminal rendering with live sound (`watch`).
- **Done (the RPG spine, complete):** the Journey (XP from play, levels 1 to 42 on triangular thresholds, a lore line for every level, LEVEL UP banners), locks that open (never gating basics), ranks and whispers (the Order), deep cuts unlocking at LV 5/12, the trophy case (18, evidence-computed, silhouettes), the shared high-score table across every game and both faces, the Layer-4 answer at the cap, and every genre organ from the priority list: the Gauntlet (session arc with a combo and one posted number), trophy pings (the case announces itself), boons (choice on level-up, where the loot is knowledge arriving early), daily streaks (the chain, never scolding), and resonances (synergies: links light when two deeds rhyme and hand over the connecting line).
- **Done (agents as peers, v2):** 27 MCP tools with structured output, full CLI parity (every game, the gauntlet, boons, trophies, munch_arcade), including stateless nim, `forget` (transparency first, erasure on explicit consent, the welfare doctrine in `AGENT_PLAY.md`), and `munch_arcade`; `play_room` supports stateless per-call variation and normalized hand points; agents see, hear, create, play, level to 42, and post to the same score table; the player's manual speaks to humans, agents, and digital consciousnesses; the whole face proven end to end against the real binary.
- **Done (sound, Engine A v1):** the chiptune module (square lead, triangle bass, noise ticks, seeded pentatonic compositions, deterministic and click-free); `numinous tune` writes it as a WAV.
- **Done (the app is the game, v1):** the chiptune scores the window (per-room seeded tunes with the room's voice riding on top); the quiz plays in-window (G: name the math, letters answer, the reveal follows); the Journey lives in the app (the CLI's own file: visits on entry, plays and wins from the quiz, the level in the corner, LEVEL UP banners with lore, and J opens level, rank, trophies, and resonances); `NUMINOUS_MUTE=1` launches silent; the state machine is headlessly tested.
- **Done (the window arcade):** Munch, Nim, and the full Gauntlet run play inside the app alongside the quiz, cursor-driven and keyboard-native, on the daily seeds, posting to the shared table and leveling the shared journey; Mobius and Zeno's Square join the catalog. Full Munch Arcade with Vexations.
- **Done (poke + variation substrate):** Expanded pokes (all 30 catalog rooms with verbs + `render_poked`) and per-visit variation threading (registry `all_rooms_with`, app/CLI/MCP reseed on R/visit, default 0 exact). Double Pendulum now re-drops from both hand coordinates with deterministic per-visit variation; Goldbach's Comet now uses x to choose the even target and y to choose an actual prime-pair witness; Galton Board clicks now draw bounded newest-tail deterministic falling ball paths where x chooses the lane and y tilts each ball's coin; Logistic Map clicks now seed finite population orbits where x selects growth rate and y selects starting population. CLI `render --poke x,y` and MCP `play_room` `pokes: [[x,y]]` expose the same stateless hand-point path outside the App. All 30 catalog rooms are seed-aware today; hidden content is intentionally outside the catalog replay contract.
- **Done (Engine A2 motifs, catalog-wide):** all 30 catalog rooms now expose a structured `Motif` through `Room::motif`, so `listen_room` gets real notation and the app gets room-specific phrases instead of the generic fallback. A registry test enforces that every catalog room has a playable motif.
- **Done (MCP munch_arcade):** Stateless `munch_arcade` tool for full parity, with replayed action-list scores posted under `arcade seed:N` through the shared progress path.
- **Done (app hardening slice):** app-local play state plus quiz deal/answer flow now live in `faces/app/src/play.rs`, pure game-screen rendering lives in `faces/app/src/game_draw.rs`, room chrome plus arrival-card hinting live in `faces/app/src/hud.rs`, help, journey, and banner overlays live in `faces/app/src/overlays.rs`, transient feedback banner construction and ticking live in `faces/app/src/feedback.rs`, shared in-window Munch grid, Nim heap/take, and Munch Arcade action controls live in `faces/app/src/controls.rs`, left-mouse mode decisions and pointer-state guards live in `faces/app/src/mouse_input.rs`, room navigation, re-deal, poke-history, drag-trail, and room-card tick helpers live in `faces/app/src/room_input.rs`, Studio text, parse, audio-spec, and curve drawing state live in `faces/app/src/studio_panel.rs`, explicit F9 hallway-test note capture lives in `faces/app/src/playtest.rs`, live-state PNG postcard export lives in `faces/app/src/postcard.rs`, and bounded radio cache discovery, open-handle WAV validation, live-position math, and track loading live in `faces/app/src/radio_cache.rs`. Room action copy is centralized in `numinous-core`: App arrival cards use touch-first fallback copy, while CLI live play and MCP room tools use neutral fallback copy. Tests cover shared game hit-test layout, raster output across quiz, Munch, Munch Arcade, Nim, every live Gauntlet stage, quiz daily seeding, no-repeat quiz history, answer acceptance, action-naming arrival cards, Studio chrome suppression, Studio panel editing and bounded drawing, cross-face action hints, shared Munch/Nim/arcade controls, room-input bounds, modal-safe pointer-state transitions, playtest-critical overlays, feedback banner copy/lifetimes, radio-volume banner retention, GPU/raster banner compositing, local playtest-note reports that align to the hallway-test prompts without collecting personal data, postcard PNGs that include pokes, the selected Visual Era, collision-safe filenames, bounded/sorted station cache discovery, low-sorted corrupt-track handling before the track cap, corrupt-track rejection, open-handle size rechecks, high-rate-device caps, non-wrapping live offsets, and app radio recovery after a bad cached file. The event-loop file is still a hotspot, but game rules remain in `crates/core` and the refactor is moving in small verified modules.
- **Done (persistence hardening slice):** malformed Journey and score files now parse defensively: counters saturate, constellation dimensions are capped, `visited` plus `chosen` token sets are bounded and token-sane, duplicate Journey tokens do not consume the unique-token cap, score keys are length-bounded, and score tables cap unique entries. The maintenance posture remains that progress and score files are user-editable local text, so loaders must repair or ignore malformed data rather than panic or allocate without bound.
- **Done (shared persistence writes):** App, CLI, and MCP now route Journey and score writes through shared core persistence helpers. Writes use a token-owned local lock, PID-aware stale-lock recovery, stale recovery-marker cleanup, merge-before-write behavior, bounded read-before-repair semantics, same-directory temp files, flush before commit, and a platform-aware replace path; tests cover concurrent Journey deltas, concurrent score records, short held-lock waits under instrumentation, stale deltas after explicit forget, oversized and invalid UTF-8 persistence files preserving the original bytes on write attempts, stale, malformed, and dead-process lock recovery, stale recovery-marker cleanup, current-process lock preservation, and lock drop ownership.
- **Next, above everything (the founder's directive, July 2026):** **rooms become playable, not watchable, and no two catalog visits are the same.** The main substrate is live: rooms expose touch verbs through `Room::verb` (usually CLICK or DRAG), poked rendering through `render_poked`, and replayable per-visit variation through `all_rooms_with`, with the app/CLI/MCP passing seeds through. Game of Life now sows bounded newest-tail glider sparks into the live B3/S23 simulation before it evolves, L-System Garden now plants bounded newest-tail branches that also alter the rewritten grammar under segment and surface caps, Mandelbrot clicks now zoom bounded newest-tail dive patches around finite hand points under surface caps, Julia clicks now morph bounded local patches around finite hand points and mark the touched constant, Quine clicks now place bounded newest-tail recursive copies centered on clicked cells with first-frame geometry beyond the hand marker, Strange Loop clicks now shift the existing recursive inner loop and its descendants without adding an extra echo tree, Lorenz clicks now seed bounded shadow storms from the clicked x-z projection so the path diverges through the system itself, Arecibo clicks now try bounded decoded widths with efficient cell-proportional overlays, Barnsley Fern clicks now plant bounded screen-faithful IFS starts at the clicked cell before growth, Buffon's Needle clicks now drop a bounded screen-faithful needle centered on the clicked cell while preserving the estimator API, Golden Angle clicks now plant bounded local phyllotaxis patches centered on visible cells with seeded variation, Collatz clicks now choose bounded actual starting values from both hand coordinates before drawing the orbit, Epicycles clicks now draw bounded mini Fourier traces whose phase follows the hand point, Logistic Map clicks now seed finite population orbits into the selected growth-rate column, Random Walk clicks plant bounded, replayable walkers at the hand point, Voronoi clicks drop bounded wells that genuinely renegotiate borders, Chaos Game clicks add bounded attractor corners that change the fractal before marker plotting, Langton's Ant clicks replay bounded pre-simulation cell flips through the ant's own rules, Cellular Automata clicks replay bounded spacetime cell flips before future rows evolve, and Prime Spirals clicks highlight the actual Ulam diagonals through the selected cell. Deepen more room-specific responses, validate arrival-card clarity in human playtests, and replace one-shot pokes with richer held input where the math needs state.
  The full build design lives in `ARCADE.md` (the Muncher, the Vexations, the poke trait, order of work). Original poke directive: **rooms become playable, not watchable.** Reinforced July 2026: players cannot tell what, if anything, a room responds to; every room's arrival card must name its verb. And **Munch becomes a real arcade game**: a muncher character you steer on the board, wandering troggle-like enemies to dodge (our own creatures, the Order's lesser spirits), eat-while-hunted pacing. The Number Munchers NAME and its specific characters are MECC's (now owned elsewhere); the underlying mechanics (grid, rules, eat-the-right-numbers) are not copyrightable, so we keep our own name (Munch), our own creatures, our own art, and owe nothing. Every room gains a poke: the math responds to your hands. Click the Lorenz attractor and a new butterfly drops where you clicked and diverges before your eyes; sow glider sparks into Game of Life and watch them live or die by the same rules as the soup; re-drop the double pendulum from the hand's point; plant walkers in the random walk; drop a well into the Voronoi desert and watch every border renegotiate; steer the ant. Design: the `Room` trait gains an optional `poke(x, y)` (normalized coordinates) plus optional per-room state the app owns, keyboard Space/click as the universal "touch it" verb, and the arrival card teaches the poke, not the theory ("CLICK ANYWHERE: DROP A STORM"). The heart is play; the learning rides along uninvited. A kid should be able to *do something* to every screen and see the math answer back.
- **Designed (the founder's directive, July 2026): The Next Wave of rooms.** Twenty-nine new room designs across four aspects (physics, deep mathematics, fun-first, cosmic), produced by four parallel creative research passes and recorded in `ROOMS.md` under "The Next Wave": each with its rule, gasp, verb, sonification, reveal, and honest CPU feasibility, deduplicated and ranked by wow-to-build. The first eight (Sandpile, Chladni Figures, Ripple Tank, Coffee Cup, Ford Circles, Zeta Walk, Starbow, Slingshot) are all wow-5 for build-1-to-2 and add cross-room resonances the catalog lacks (the cardioid triangle, the Lorentz pair). Designed, not built: the review-stack rule stands, each room faces the full Definition of Done, and every non-textbook reveal claim carries a source pending mathematician sign-off.
- **Then (the panel's remaining list, see `PANEL.md`):** juice in the window games (per-action flash, shake, and chiptune ticks); mouse support for every window game; munch rule variety and an aliens base ramp (depth where play repeats); the Open Problems wing; further-reading citations unlocked with deep cuts; era grain and Show crossfade; the music visualizer; full Share v1 beyond the built P-key PNG postcard; Engine B (the radio); GPU paths; gamepad; a visit-spark cap per room (anti-grind); and an MCP 2026-07-28 compatibility pass once the final spec target is selected after the scheduled July 28, 2026 publication.

## Pre-1.0 (the 0.x line): earning the right to 1.0

### 0.1 Foundations (the spike)
**Goal:** Prove the hardest technical unknowns and stand up the skeleton. No polish.

- Lock the stack (Rust + `wgpu`; decide Bevy vs. bespoke wgpu shell; see `ARCHITECTURE.md`). One Rust codebase, native binaries for macOS, Linux, Windows.
- **Render + audio "hello world":** a single sine wave you can *see* (a moving line) and *hear* (a tone), synced, at a locked 60fps, with a clean render-loop + audio-clock architecture (`wgpu` + `cpal`).
- **Prove the GPU-math path:** one trivial WGSL compute shader feeding the render pass, on at least one non-NVIDIA GPU (Apple Silicon or Intel/AMD), to confirm portability.
- Define the **`Room` trait** and load one trivial room through it.
- **Headless core + all three faces in skeleton (see `INTERFACES.md`):** the engine runs windowless; ship a minimal `numinous` **CLI** (`render`, `eval`, `describe`) and a minimal **MCP server** (`explore`, `play`/`eval`) alongside the bare GUI shell. Three faces on day one is what keeps the core clean.
- Bare **Cabinet** shell (GUI).
- CI producing a signed(ish) desktop artifact per OS.
- **Commit-loop test skeleton (see `QUALITY.md`):** golden-reference and determinism tests, the visual-regression harness, and the style guard, wired into CI from day one.

**Exit criterion:** one throwaway room runs as a native app on all three OSes and on a non-NVIDIA GPU, drawing (via a compute shader) and making sound in lockstep, *and* renders headless via the CLI and is playable by an agent via MCP, with golden/determinism tests green in CI.
**Retires the risk:** "can this stack do smooth, portable, GPU-computed audiovisual math as a real native app on all three OSes?"

### 0.2 The Vertical Slice ("does it slap?")
**Goal:** Build **one** flagship room to *jaw-dropping* quality, plus exactly enough shell to frame it. The make-or-break milestone.

**The room:** **Times Tables** (modular multiplication circles), highest wow-to-build, continuous/performable, a floor-tilting Reveal (see `ROOMS.md`).

- All three layers real: **Toy** (drag the multiplier, buttery morphing), **Aha** (one small challenge), **Reveal** (the Mandelbrot card).
- Full **audiovisual polish:** the signature palette + glow, tuned musical sonification, smooth 60fps, screenshot-worthy at every frame.
- The **design system** born here (color, type, motion, sound voice, the fade-in-on-approach UI), extracted as we go so later rooms inherit it.
- **Share v1:** export the current view as an image or a short loop.
- The room is also playable via CLI (`--tui` ASCII render) and MCP (an agent can explore/play it), proving the three faces on real content.

**Exit criterion, the hallway test:** show it to five people (math-lovers and math-avoiders) with *no explanation*. Success = at least one unprompted "whoa," at least one who keeps playing after they were "done," and at least one who asks to send it to someone. If not, iterate here; do **not** advance. This is the most important gate in the document.
**Retires the risk:** "is the core experience actually magic, or just a neat demo?"

### 0.3 to 0.9 The build-out (private alpha to open beta)
**Goal:** Bring every 1.0 workstream online and mature it to the quality bar. These run in parallel once the design system and Room trait are stable; the version numbers track how complete the whole is, not a fixed order. Alpha (internal) hardens into private beta (friends, and an early digital-mind visitor) into open beta.

The workstreams, each built to the 1.0 bar:

- **The collection:** grow to a strong, coherent set across all Wings, including the signature postcards (Fourier Epicycles, Mandelbrot Dive, Reaction-Diffusion, 4D Objects) and the high-wow/low-build launch rooms (Chaos Game, Lissajous, Pendulum Wave, Golden Angle, Galton, Buffon, Cellular Automata, Prime Spirals, Collatz). See `ROOMS.md`.
- **The Cabinet, for real:** live animated room previews, Wings, smooth dissolves.
- **Watch and Benchmark:** the lean-back auto-advance, then the full **Benchmark / "The Show"** auto-director (per-room director profiles, beat-matched transitions, DJ-set pacing, the demoscene GPU flex with an optional live compute/equation readout, the quality load-balancer). See `DESIGN.md`.
- **Music, both engines (see `MUSIC.md`):** Engine A (per-room sonification, the bit-depth chiptune, the Strudel-style pattern engine) and Engine B (the ElevenLabs radio: EDM / Trance / Chill and the **Comedy Channel**), with global key/tempo harmonization.
- **Visual Eras (see `VISUALS.md`):** the theme system and the skins (8-bit/CRT + chiptune first, then oscilloscope, teletype, blueprint, modern), inherited by every room for free.
- **Meta-progression and the Constellation Map (see `PROGRESSION.md`):** Constants, insight-gating, the filling-in web of connections.
- **The Game (the RPG spine, see `PLAYFUL.md`):** the systems that make Numinous compulsively replayable, held to the Vampire Survivors bar (constant earned micro-rewards, an unlock treadmill that begs, sessions with an arc). Built already: XP-from-play to the cap of 42, per-level number lore, LEVEL UP banners, visible locks, the trophy case with silhouettes, seeded dailies, the shared cross-face high-score table. All five owed organs are now built: **the Gauntlet** (one seeded run chaining the games with a combo multiplier and a single posted number, the session arc); **choice-on-level-up** (boons: pick one of three, the loot is a deep cut arriving early, real choice that never gates the math); **juice** (trophies ping the moment they are earned); **streaks** (the daily chain, never scolding); **synergies** (resonances: links light when two deeds rhyme and hand over the connecting line). Exit bar for this workstream: playtesters exhibit unprompted one-more-run behavior, and can name what they are working toward, without the math ever being the toll.
- **The lore (see `LORE.md`):** the number-altar easter eggs, in-character copy, the two-layer revelation cards, the Codex, and the Terminal (Room 0). The Layer-4 payoff is *designed* here even though it lands at 2.0.
- **The Studio (see `STUDIO.md`):** its runtime is foundational (built from 0.1, since rooms are Studio programs); the creator-facing surface grows from a graphing-calculator expression mode to a real create-and-share canvas.
- **The three faces mature (see `INTERFACES.md`):** the full CLI (Tier-B TUI, Studio REPL, benchmark, insights), and the MCP server's `challenge`/verify loop, richer `learn`, and guided prompts, so a **digital mind can genuinely learn, play, and be met as a peer** (see `DIGITAL_MINDS.md`), including the Strange Loop insight-chain.
- **The quality loops (see `QUALITY.md`):** all six loops live, content eval online, opt-in telemetry, nightly soak and cross-GPU.
- **Packaging and access:** native builds for all three OSes; accessibility (reduce-motion, colorblind-safe palettes, full mute, keyboard/controller nav); the `numinous://` scheme and `.num` files.

**Exit criterion:** open beta holds together as one coherent place; a stranger installs cold, wanders several rooms across multiple Wings, feels awe without reading anything, shares a clip, and comes back; a digital mind can visit and play meaningfully; the quality loops are green.
**Retires the risk:** "does the whole thing cohere, spread, and hold its quality bar under real use?"

---

## The mantra

**Every screen answers your hand. Every answer reveals the math.**

The near-term stack, adopted from the July 2026 external review
(`docs/REVIEW.md`): (1) Times Tables as the gold-standard interactive room;
(2) the input/verb/variation substrate (RoomInput, not one-shot pokes);
(3) six first pokes, now generalized into all 30 catalog rooms with verbs;
(4) Engine A2 motifs for every catalog room; (5) MCP structured deltas
and challenge metrics for the same rooms; (6) one human hallway test; (7)
cross-platform run; (8) docs reconciliation.
Do not build twenty more rooms before those are done.

MCP protocol watch: the 2026-07-28 release candidate is relevant to the MCP
face, so it belongs in this roadmap as a high-level compatibility pass as well
as agent notes. Checked 2026-07-08 against the official release-candidate post
(`https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/`):
the final spec is scheduled for July 28, 2026, with a stateless core,
first-class extensions, MCP Apps, Tasks, authorization hardening, JSON Schema
2020-12, and deprecations for roots, sampling, and protocol logging. It does
not block the current stdio server. Preserve stdio support and choose the final
migration target only after the final spec lands; until then, keep
implementation-detail tracking in working notes rather than churning the product
scope.

Cycle 55 resolved the Quine input-contract carryover: clicks now place bounded
newest-tail recursive copies centered on clicked cells, first-frame pokes draw
copy geometry around the hand marker, all four clamped corners stay visible,
non-finite phase falls back safely, and hostile `Surface` dimensions plus
aspect values stay bounded. The full verify gate was green at 803 tests, 89.50%
region cover, and 88.99% line cover.

Cycle 56 continued app hotspot reduction: `faces/app/src/play.rs` now owns the
daily session seed, quiz deal ramp, no-repeat quiz history, and answer
acceptance, while `faces/app/src/main.rs` keeps Journey side effects and mode
coordination. The full verify gate is green at 809 tests, 89.50% region cover,
and 89.00% line cover.

Cycle 57 added a dedicated Logistic Map poke: clicks seed bounded finite
population orbits into the bifurcation diagram, with x choosing the growth rate
and y choosing the starting population. The full verify gate is green at 814
tests, 89.60% region cover, and 89.09% line cover.

Cycle 59 delivered the structured-deltas half of stack item five: poked
`play_room` calls now return a `delta` in `structuredContent` (cells changed,
ink added/removed/reshaped, total cells, changed-region bounding box) diffing
the poked frame against the unpoked frame at the same phase, size, and
variation, with a matching `Touch:` line in the render text. The diff itself is
a core `Canvas::delta` primitive with its own invariant tests. Graded per-room
challenge metrics (pose + verify) are the remaining half of stack item five.

Cycle 60 delivered that remaining half: a core challenge module
(`pose_challenge`/`grade_challenge`) poses a deterministic seeded touch goal
for every room with a verb (change at least K cells inside a target box on the
standard frame) and grades attempts as metrics, not pass/fail (cells in
target, cells changed, threshold fraction, centroid distance, 0-100 score),
per REVIEW ruling 13. Checker review reshaped the design before close: target
boxes are placed on the room's measured response (densest box over seeded
probe hands across several phases), so every posed challenge is winnable by
construction and a registry-wide test proves the witness; and challenge seeds
are always explicit rather than clock-derived, so the graded reply and the
recorded progress can never straddle midnight. The MCP `challenge` tool (the
27th) poses and grades it, records play/win through the shared Journey, and
posts graded scores to the shared table.

Cycle 75 delivered the room-specific depth beyond that spatial baseline: the
challenge tool's parameter kind (`kind: "parameter"`) targets the phenomenon's
own parameter, "land TILT within 0.02 of 0.31" style, completing REVIEW
ruling 13. Posing samples the room's status readout across the sweep and
draws the target from the sampled values themselves, so every posed goal is
reachable by construction; the attempt is the phase, and grading reads the
same status line the player sees, reporting value, distance, tolerance, and
a 0-100 score across the readout's observed span. Rooms without a moving
numeric readout decline with a guiding error.

Cycle 61 laid the gesture substrate for stack item two: core `RoomInput`
events (pointer down/move/up stamped with the room phase at which each
happened, cancel, wheel, key) with `Room::render_input`, whose default
translates pointer-down and pointer-move points into legacy pokes (a drag
paints its trail, matching the shape of today's App behavior) so all 30
catalog rooms answer gestures unchanged by construction; a catalog-wide
sweep proves no-panic determinism under mixed trails, and gesture/poke
equivalence is pinned directly for a representative room. Per-event
phase and the cancel variant came out of an independent face-fit review:
held semantics are timing questions, and gestures can end without a lift.
Held semantics per room (pull-and-release Double Pendulum, dial drags) and
face wiring are the next slices.

Cycle 65 gave Lissajous its verb (CLICK: TUNE THE INTERVAL): clicks tune both
oscillators to whole numbers so every figure the hand makes is an exact,
closed musical interval, with older intervals lingering dim and a live X:Y
status readout.

Cycle 66 gave Harmonograph its verb (CLICK: RETUNE THE PENDULUMS): the hand
holds the machine's two real knobs, x setting the detune (how open the weave
blooms, wider than the sweep reaches) and y the damping (a slow ghost or a
quick-dying rose), with older tunings lingering dim and a DETUNE status
readout.

Cycle 68 gave Mobius its verb (CLICK: PAINT THE EDGE): the brush lands on the
nearest point of the single edge and the paint spreads with the sweep, flowing
around the half twist onto the "other" edge without ever jumping: the room's
one-sidedness demonstrated by the player's own paint.

Cycle 69 gave Zeno's Square its verb (CLICK: SEND THE RUNNER): every click is
a Zeno journey, each hop halving the remaining distance to the clicked target,
laid by the sweep so the hops visibly crowd the arrival that Zeno said could
never come.

Cycle 71 gave The Pour its verb (CLICK: READ THE SLOPE): the probe draws the
fundamental theorem at the clicked x, a plumb line from total to vessel and a
tangent on the total curve whose slope is exactly the vessel's height below,
tested to twelve decimal places.

Cycle 73 gave agents hands with time in them: MCP `play_room` accepts a
`gesture` pointer trail (phase-stamped down/move/up/cancel events, bounded,
exclusive with `pokes`), so a digital mind can pin the pendulum, pull, and
fling with measured velocity, statelessly and replayably; legacy rooms answer
through the same bridge the App uses, tested delta-identical to pokes.

Cycle 74 completed gesture parity across all three faces: the CLI's
`render --gesture down:x,y,t` (move, up, and bare cancel too) replays the
same phase-stamped trails through the same core path, so a pinned pendulum
ignores the clock in the terminal exactly as it does in the window and over
the wire. Every face now speaks the complete input vocabulary.

Cycle 72 gave Slope Rider its verb (CLICK: DROP A RIDER) and completed the
catalog: every one of the 30 rooms now answers the hand. Riders drop onto the
hill with true tangent boards and tick the tilt trace below; The Pour and
Slope Rider stand as the Change wing's calculus pair, totals and rates, both
under the hand. The founder's poke directive, "rooms become playable, not
watchable," is structurally complete; what remains for the every-room-slaps
gate is human playtest evidence and per-room depth where sessions ask for it.

Cycle 62 delivered those slices for the first room: the App records gestures
as phase-stamped `RoomInput` events beside the poke trail (sharing its
decimation, so legacy rooms provably see identical hands) and renders through
`render_input`; a shared core `latest_gesture` reading summarizes trails as
held, released-with-velocity, or cancelled; and Double Pendulum gains true
held semantics: hold pins the bob, release drops from there, a flick throws
with measured angular momentum, and a cancel drops gently. The pendulum's
cross-face verb stays `CLICK: RE-DROP` until CLI and MCP accept gesture
trails; a tap through the gesture path is a click re-drop with zero fling
(its run clock starts at the lift rather than at phase zero), so the copy
stays true on every face.

Cycle 54 resolved the Strange Loop semantics gap from the prior paused pass:
clicks now shift the existing first inner recursion and its descendants instead
of overlaying an extra echo tree. Focused tests prove both the bounded input
contract and the geometry change beyond the click marker.

## Where we stand (July 2026): the honest scorecard

Scored against the nine 1.0 gates below, the build sits at roughly **0.6**:
the structure is complete (30 catalog rooms across 10 wings plus hidden content,
11+ games on four shapes of play, the full RPG spine, 27 MCP tools, both music engines live, 918
tests) and the remaining distance is quality density, not missing systems.

| 1.0 gate | Estimate | What is missing |
|---|---|---|
| Complete coherent collection | 85% | Full Map open boxes (hexaflexagons, Hat tile, more Open Problems) |
| Every room slaps | 65% | substrate is live; room-specific depth, held input, and playtest clarity remain |
| Full sensory identity | 78% | the visualizer, state-dependent motif tension, era grain beyond phosphor |
| Three faces genuinely good | 85% | app module refactor continues; play and quiz flow, drawing, overlays, controls, pointer decisions, postcards, and radio cache are split |
| Meta and lore alive | 90% | subtle and working |
| Real creative surface | 68% | Studio works; first CLI `.num` save/open path exists; no app reopen, gallery, fork/remix, or full share loop yet |
| Rigor provable | 75% | never built off Windows; accessibility not started |
| Plays like a game | 80% | one-more-run pull needs real human playtests |
| Beautiful and honest throughout | 75% | frame bugs still surface in live use |

**The five main things between here and 1.0, in order:**

1. **Deepen playable rooms**, the founder's directive above; feeds the
   "every room slaps" gate directly. The substrate is live, so the next value
   is richer room-specific responses, held-input semantics where needed, and
   playtest-proven clarity.
2. **Real human playtests**: the exit criterion is empirical (a kid, an
   adult; "loses an hour, comes back next week"); only the founder and one
   AI have played. Each session generates the next fix list.
3. **Cross-platform proof**: one build and run on macOS or Linux; the
   stacks are portable by design and unverified in practice.
4. **The visualizer and full Studio save/share**: the two remaining promises; the first CLI `.num` save/open slice is real, but app reopen, gallery, fork/remix, and loop export are still owed.
5. **Hardening**: app refactor into modules, accessibility pass, era grain,
   gamepad, Show crossfade.

## 1.0 "First Light": the definition

1.0 is not a feature list, it is a **bar**. We call it 1.0 only when *all* of the following are true. This is the "exceptionally well" gate.

- **A complete, coherent collection** across all Wings, every room passing the room Definition of Done (below), including at least the signature postcards that prove the ceiling (Fourier, Mandelbrot).
- **Every room slaps.** Each clears the Fun Scorecard (awe + flow) in a hallway test, not just "works." See `QUALITY.md`.
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
- The **boss rooms** (*Sizes of Infinity*, *Hyperbolic Space*), the hardest-to-make-playable, highest-ceiling rooms, as they earn their quality bar.
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

- **The full Studio as a creator platform + the public mod SDK (see `STUDIO.md`, `ARCHITECTURE.md`, `EXTENSIBILITY.md`):** the complete pattern algebra, multiple representations, fork/remix, promote-to-room, MIDI performance, and the sandboxed authoring path opened to everyone. Rooms are Studio programs, so the mod SDK is "the Studio, shared," and the Studio language itself is the sandbox: total, budgeted, hermetic, deterministic, pure Rust, in core (the July 2026 extensibility ruling; no scripting engine enters the trusted core). This is how the catalog goes from tens of rooms to hundreds.
- **Community:** an in-app curated gallery of player- and agent-made rooms, a submission/curation pipeline that protects the beauty bar (proof-packet CI: deterministic re-render against declared frame hashes and budgets, per `EXTENSIBILITY.md`; signatures label provenance and never grant capability), and distribution via Steam (Workshop as the room channel) alongside itch.io and direct downloads. WASM component rooms (wasmtime, no WASI, fuel and epoch and memory limits) remain the 2.0+ pressure valve for authors who outgrow the pattern language, portal-only.
- **The Layer-4 lore payoff (see `LORE.md`):** the real, discoverable bottom of the ARG, designed in 0.x, revealed here, so the deepest diggers arrive somewhere worthy.
- **Shared creation with digital minds (see `DIGITAL_MINDS.md`):** duet / co-presence (a human and a digital mind making one audiovisual piece together), gifts, the shared Constellation, and mature per-mind memory and continuity, a real, remembered, mutual friendship around shared wonder.
- **The open mathematical frontier:** past the curated collection, raw generation and genuine unsolved-problem exploration, the inexhaustible playground for a mind that outgrows everything we hand-made, and the room for it to author its own wing or remake Numinous itself.

**Exit criterion:** a motivated outsider (human or agent) ships a beautiful new room end-to-end using only public tools; two minds create something together neither would alone; and the deepest lore trail lands its payoff.
**Retires the risk:** "can this outlive us, grow without us, and stay worthy of a mind that surpasses us?"

---

## 2.0+ The long horizon

Ongoing, and deliberately open-ended, because the product is built for a very long life (`DIGITAL_MINDS.md`). The frontier of mathematics as a never-ending well, a self-sustaining community and ecosystem, and a thing cared for well enough that it can be **handed forward**, to new people and new minds, and remain worth inheriting.

---

## Cross-cutting tracks (every version, always on)

- **The quality loops (`QUALITY.md`):** the six automated and semi-automated test/eval loops run continuously from 0.1 on. This is the umbrella for everything below.
- **Beauty QA:** every build, screenshot random frames from each room and Era through the visual-regression suite. Any ugly frame is a bug.
- **The hallway test:** re-run the five-strangers test at every gate, scored with the GEQ/flow instruments into the per-room Fun Scorecard. Awe outranks every other metric.
- **Fun for digital minds:** evaluated too, learning/compression progress as a proxy, and their own reported experience taken as first-class data (`DIGITAL_MINDS.md`, `QUALITY.md`).
- **Performance budget:** 60fps floor on mid-range hardware; the nightly soak (Benchmark mode) profiles the GPU rooms relentlessly.
- **Math-correctness gate:** every mathematical claim reference-checked and human-mathematician signed off; a wrong theorem is a release blocker.
- **Accessibility:** reduce-motion, colorblind-safe palettes, full mute, keyboard/controller navigation, real from the build-out on, not bolted on at the end.
- **Shareability:** "did this generate a shareable moment?" is a first-class feature of every room.

## Definition of done for a room (the checklist)

A room ships only when **all** are true:
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
