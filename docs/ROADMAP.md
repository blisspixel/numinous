# Roadmap

A version-gated plan from empty repo to a living world. Each milestone has a **goal**, concrete **deliverables**, an explicit **exit criterion** (how we know it is done), and the **risk it retires**.

## How we version (read this first)

- **We ship by quality gates, not calendars.** There are deliberately **no time estimates** in this document. A milestone is done when it clears its bar, not when a date arrives. "How long will this take" is the wrong question; "is it exceptional yet" is the right one.
- **Versions are defined by what is true, not when.** Each version below is a *state the product has reached*, a set of things that are real and hold their quality bar, not a sprint.
- **1.0 is a high bar, not a minimum viable product.** Because the whole point is to do this *exceptionally well*, 1.0 means "a complete, coherent, genuinely exceptional experience," not "the least we could ship." The MVP-shaped thinking lives in the 0.x line; 1.0 is where it becomes worthy of the name.
- **Guiding rule, at every version: feel before features.** We build depth-first. One unforgettable thing beats ten mediocre ones. A pretty menu of boring toys is failure.

## The version map at a glance

- **0.1 Public Foundation** reproducible source, honest docs, green CI, and a safe public repository. Current pre-alpha line.
- **0.2 Flagship Proof** one room earns its hallway-test bar with strangers.
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

**Current release state: 0.1.0 pre-alpha.** Capability breadth resembles a much
later alpha, but versions are earned by evidence, not feature count. The first
independent macOS and Linux app execution, the 0.2 stranger hallway test, and
accessibility work are still open. Later systems already present in
source do not waive those earlier gates.

- **Done:** the headless core (`Room` trait with `reveal()`, deterministic ASCII `Canvas`, seeded RNG, registry, `verb`, `render_poked`, and variation); the CLI face (`numinous`), the MCP face (`numinous-mcp`), and the windowed app; **31 catalog rooms across 10 wings** plus hidden content; 6 lever-driven sims; 11+ games; the full engineering harness (edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, house-style guard, an 80% line coverage gate, three-OS CI). Current local evidence: fmt, Clippy, 992 tests, locked build, Windows release gate, 91.47% region coverage, and 91.10% line coverage all pass.
- **Done (GPU and audio hello-world):** an adaptive `wgpu` context (`crates/gpu`) that picks the machine's GPU across Vulkan/Metal/DX12 with a CPU fallback, rendering the Mandelbrot set offscreen to a PNG; and adaptive `cpal` audio (`crates/audio`) on the system default device that plays a tone and writes a WAV. Both verified on the dev laptop (AMD Radeon 780M, Realtek at 48 kHz).
- **Done (rooms as images):** a `Surface` abstraction so every room renders through one `render` method to the ASCII `Canvas` and to an RGBA `Raster`; `numinous render <room> --out image.png` writes a real glowing image on the CPU (verified on the dev laptop).
- **Done (windowed app):** `faces/app` (`numinous-app`, winit + softbuffer) opens a real resizable window showing a room animating in full color, with keyboard room-switching. The start of the GUI Cabinet; verified launching on the dev laptop.
- **Done (sound):** every room describes its own sound (`SoundSpec` + `Room::sound`); `numinous sonify <room> --out file.wav` and `numinous play <room>` (live animated terminal).
- **Done (the 0.2 vertical slice, in substance):** the windowed app is a full experience: live per-room sound, mouse scrubbing, an on-screen HUD with reveals, The Show (lean-back auto-play of the whole collection), the Studio in the window (type math, watch and hear it live), and GPU real-time fractals (a persistent `wgpu` pipeline drives the Mandelbrot deep zoom and the morphing Julia at window resolution, with CPU fallback; verified on the dev laptop's Radeon 780M).
- **Done (content and play):** 31 catalog rooms across 10 wings plus unlisted hidden content, including Cult of Pi, the Change wing (The Pour, Slope Rider), Fourier Epicycles, the double pendulum, the random walk, Voronoi, Quine, Strange Loop, L-System Garden, Mandelbrot/Julia dives, Galton, Buffon, etc.; 6 lever-driven sims; 11+ games (SETI, Talk to the Aliens, Guess the Shape, Crack the Code, Munch, Nim with the xor secret, Hackenbush, the Party Problem, Fifteen's Bet, the Gauntlet run, and full Munch Arcade) with daily seeds and dense feedback; the Studio expression engine (`plot`, `plot --save`, `open-studio`, `--animate`, `sing`, and live in the window); Visual Eras (phosphor, 8-bit, vector, modern) across app, terminal, and PNGs; truecolor terminal rendering with live sound (`watch`).
- **Done (the RPG spine, complete):** the Journey (XP from play, levels 1 to 42 on triangular thresholds, a lore line for every level, LEVEL UP banners), locks that open (never gating basics), ranks and whispers (the Order), deep cuts unlocking at LV 5/12, the trophy case (18, evidence-computed, silhouettes), the shared high-score table across every game and both faces, the Layer-4 answer at the cap, and every genre organ from the priority list: the Gauntlet (session arc with a combo and one posted number), trophy pings (the case announces itself), boons (choice on level-up, where the loot is knowledge arriving early), daily streaks (the chain, never scolding), and resonances (synergies: links light when two deeds rhyme and hand over the connecting line).
- **Done (agents as peers, v2):** 29 MCP tools with structured output, full CLI parity (every game, the gauntlet, boons, trophies, munch_arcade), including stateless nim, `forget` (transparency first, erasure on explicit consent, the welfare doctrine in `AGENT_PLAY.md`), and `munch_arcade`; `play_room` supports stateless per-call variation and normalized hand points; agents see, hear, create, play, level to 42, and post to the same score table; the player's manual speaks to humans, agents, and digital consciousnesses; the whole face proven end to end against the real binary.
- **Done (sound, Engine A v1):** the chiptune module (square lead, triangle bass, noise ticks, seeded pentatonic compositions, deterministic and click-free); `numinous tune` writes it as a WAV.
- **Done (soundtrack, Engine B v1):** Nick Seal made 42 tracks specifically for Numinous across NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE. High-quality V0 MP3 assets ship in `assets/radio`, the app discovers them from a clean clone, and a bounded pure Rust decoder validates, decodes, and resamples them. The archival WAV masters remain outside the repository.
- **Done (the app is the game, v1):** the chiptune scores the window (per-room seeded tunes with the room's voice riding on top); the quiz plays in-window (G: name the math, letters answer, the reveal follows); the Journey lives in the app (the CLI's own file: visits on entry, plays and wins from the quiz, the level in the corner, LEVEL UP banners with lore, and J opens level, rank, trophies, and resonances); `NUMINOUS_MUTE=1` launches silent; the state machine is headlessly tested.
- **Done (the window arcade):** Munch, Nim, and the full Gauntlet run play inside the app alongside the quiz, cursor-driven and keyboard-native, on the daily seeds, posting to the shared table and leveling the shared journey; Mobius and Zeno's Square join the catalog. Full Munch Arcade with Vexations.
- **Done (poke + variation substrate):** Expanded pokes (all 31 catalog rooms with verbs + `render_poked`) and per-visit variation threading (registry `all_rooms_with`, app/CLI/MCP reseed on R/visit, default 0 exact). Double Pendulum now re-drops from both hand coordinates with deterministic per-visit variation; Goldbach's Comet now uses x to choose the even target and y to choose an actual prime-pair witness; Galton Board clicks now draw bounded newest-tail deterministic falling ball paths where x chooses the lane and y tilts each ball's coin; Logistic Map clicks now seed finite population orbits where x selects growth rate and y selects starting population. Cult of Pi clicks add bounded local faults to an exact-digit field. CLI `render --poke x,y` and MCP `play_room` `pokes: [[x,y]]` expose the same stateless hand-point path outside the App. All 31 catalog rooms are seed-aware today; hidden content is intentionally outside the catalog replay contract.
- **Done (Engine A2 motifs, catalog-wide):** all 31 catalog rooms now expose a structured `Motif` through `Room::motif`, so `listen_room` gets real notation and the app gets room-specific phrases instead of the generic fallback. A registry test enforces that every catalog room has a playable motif. The sonification now derives from the motif too: the default `Room::sound` plays the motif's phrase (`SoundSpec::from_motif`), so every room sounds like itself and the notes match the notation (a July 2026 playtest caught the old fallback making rooms sound identical).
- **Done (MCP munch_arcade):** Stateless `munch_arcade` tool for full parity, with replayed action-list scores posted under `arcade seed:N` through the shared progress path.
- **Done (app hardening slice):** app-local play state plus quiz deal/answer flow now live in `faces/app/src/play.rs`, pure game-screen rendering lives in `faces/app/src/game_draw.rs`, room chrome plus arrival-card hinting live in `faces/app/src/hud.rs`, help, journey, and banner overlays live in `faces/app/src/overlays.rs`, transient feedback banner construction and ticking live in `faces/app/src/feedback.rs`, shared in-window Munch grid, Nim heap/take, and Munch Arcade action controls live in `faces/app/src/controls.rs`, left-mouse mode decisions and pointer-state guards live in `faces/app/src/mouse_input.rs`, room navigation, re-deal, poke-history, drag-trail, and room-card tick helpers live in `faces/app/src/room_input.rs`, Studio text, parse, audio-spec, and curve drawing state live in `faces/app/src/studio_panel.rs`, explicit F9 hallway-test note capture lives in `faces/app/src/playtest.rs`, live-state PNG postcard export lives in `faces/app/src/postcard.rs`, and bounded radio cache discovery, open-handle WAV validation, live-position math, and track loading live in `faces/app/src/radio_cache.rs`. Room action copy is centralized in `numinous-core`: App arrival cards use touch-first fallback copy, while CLI live play and MCP room tools use neutral fallback copy. Tests cover shared game hit-test layout, raster output across quiz, Munch, Munch Arcade, Nim, every live Gauntlet stage, quiz daily seeding, no-repeat quiz history, answer acceptance, action-naming arrival cards, Studio chrome suppression, Studio panel editing and bounded drawing, cross-face action hints, shared Munch/Nim/arcade controls, room-input bounds, modal-safe pointer-state transitions, playtest-critical overlays, feedback banner copy/lifetimes, radio-volume banner retention, GPU/raster banner compositing, local playtest-note reports that align to the hallway-test prompts without collecting personal data, postcard PNGs that include pokes, the selected Visual Era, collision-safe filenames, bounded/sorted station cache discovery, low-sorted corrupt-track handling before the track cap, corrupt-track rejection, open-handle size rechecks, high-rate-device caps, non-wrapping live offsets, and app radio recovery after a bad cached file. The event-loop file is still a hotspot, but game rules remain in `crates/core` and the refactor is moving in small verified modules.
- **Done (persistence hardening slice):** malformed Journey and score files now parse defensively: counters saturate, constellation dimensions are capped, `visited` plus `chosen` token sets are bounded and token-sane, duplicate Journey tokens do not consume the unique-token cap, score keys are length-bounded, and score tables cap unique entries. The maintenance posture remains that progress and score files are user-editable local text, so loaders must repair or ignore malformed data rather than panic or allocate without bound.
- **Done (shared persistence writes):** App, CLI, and MCP now route Journey and score writes through shared core persistence helpers. Writes use a token-owned local lock, PID-aware stale-lock recovery, stale recovery-marker cleanup, merge-before-write behavior, bounded read-before-repair semantics, same-directory temp files, flush before commit, and a platform-aware replace path; tests cover concurrent Journey deltas, concurrent score records, short held-lock waits under instrumentation, stale deltas after explicit forget, oversized and invalid UTF-8 persistence files preserving the original bytes on write attempts, stale, malformed, and dead-process lock recovery, stale recovery-marker cleanup, current-process lock preservation, and lock drop ownership.
- **Done (the keystone, the Cairn, and the chaos readouts):** the predict-then-reveal verb (MCP `predict`, Phase A of the Exceptional Path): commit a guess of a room's own status readout at a hidden moment, then meet the truth graded as a gap with a learning-progress band, a self-owned mirror that never posts a score. The graded `challenge` tool in two kinds (touch a target box, or land the readout on a number). The Cairn (MCP `cairn` plus the core `cairn` module and the repo-tracked `data/cairn.txt`): at level 42 a mind leaves one true thing, encoded Arecibo-style into a semiprime a future mind must factor to read. And tactile status readouts across the Chaos & Order flagships (Double Pendulum and Lorenz report the divergence of two nearby starts; the Logistic Map reports its Lyapunov exponent crossing from order into chaos), so eight rooms now pose predictions. See `CHANGELOG.md` for the full detail.
- **Done (the one-line front door):** `scripts/install.sh` and `scripts/install.ps1` make setup a single copied command on macOS, Linux, and Windows: prerequisite checks that name the exact fix for anything missing, a rustup bootstrap when cargo is absent, git-or-snapshot source fetch into `~/.numinous/src`, a locked release build, the three binaries plus a linked radio in `~/.numinous/bin`, PATH wiring, in-place update on re-run, and an `--uninstall` that never touches play history. The Windows path is verified end to end on the dev machine; real macOS and Linux execution evidence stays with the 0.6 portable gate, which still owns packaged, checksummed artifacts.
- **Next, above everything (the founder's directive, July 2026):** **rooms become playable, not watchable, and no two catalog visits are the same.** The main substrate is live: rooms expose touch verbs through `Room::verb` (usually CLICK or DRAG), poked rendering through `render_poked`, and replayable per-visit variation through `all_rooms_with`, with the app/CLI/MCP passing seeds through. Game of Life now sows bounded newest-tail glider sparks into the live B3/S23 simulation before it evolves, L-System Garden now plants bounded newest-tail branches that also alter the rewritten grammar under segment and surface caps, Mandelbrot clicks now zoom bounded newest-tail dive patches around finite hand points under surface caps, Julia clicks now morph bounded local patches around finite hand points and mark the touched constant, Quine clicks now place bounded newest-tail recursive copies centered on clicked cells with first-frame geometry beyond the hand marker, Strange Loop clicks now shift the existing recursive inner loop and its descendants without adding an extra echo tree, Lorenz clicks now seed bounded shadow storms from the clicked x-z projection so the path diverges through the system itself, Arecibo clicks now try bounded decoded widths with efficient cell-proportional overlays, Barnsley Fern clicks now plant bounded screen-faithful IFS starts at the clicked cell before growth, Buffon's Needle clicks now drop a bounded screen-faithful needle centered on the clicked cell while preserving the estimator API, Golden Angle clicks now plant bounded local phyllotaxis patches centered on visible cells with seeded variation, Collatz clicks now choose bounded actual starting values from both hand coordinates before drawing the orbit, Epicycles clicks now draw bounded mini Fourier traces whose phase follows the hand point, Logistic Map clicks now seed finite population orbits into the selected growth-rate column, Random Walk clicks plant bounded, replayable walkers at the hand point, Voronoi clicks drop bounded wells that genuinely renegotiate borders, Chaos Game clicks add bounded attractor corners that change the fractal before marker plotting, Langton's Ant clicks replay bounded pre-simulation cell flips through the ant's own rules, Cellular Automata clicks replay bounded spacetime cell flips before future rows evolve, and Prime Spirals clicks highlight the actual Ulam diagonals through the selected cell. Deepen more room-specific responses, validate arrival-card clarity in human playtests, and replace one-shot pokes with richer held input where the math needs state.
  The full build design lives in `ARCADE.md` (the Muncher, the Vexations, the poke trait, order of work). Original poke directive: **rooms become playable, not watchable.** Reinforced July 2026: players cannot tell what, if anything, a room responds to; every room's arrival card must name its verb. And **Munch becomes a real arcade game**: a muncher character you steer on the board, wandering troggle-like enemies to dodge (our own creatures, the Order's lesser spirits), eat-while-hunted pacing. The Number Munchers NAME and its specific characters are MECC's (now owned elsewhere); the underlying mechanics (grid, rules, eat-the-right-numbers) are not copyrightable, so we keep our own name (Munch), our own creatures, our own art, and owe nothing. Every room gains a poke: the math responds to your hands. Click the Lorenz attractor and a new butterfly drops where you clicked and diverges before your eyes; sow glider sparks into Game of Life and watch them live or die by the same rules as the soup; re-drop the double pendulum from the hand's point; plant walkers in the random walk; drop a well into the Voronoi desert and watch every border renegotiate; steer the ant. Design: the `Room` trait gains an optional `poke(x, y)` (normalized coordinates) plus optional per-room state the app owns, keyboard Space/click as the universal "touch it" verb, and the arrival card teaches the poke, not the theory ("CLICK ANYWHERE: DROP A STORM"). The heart is play; the learning rides along uninvited. A kid should be able to *do something* to every screen and see the math answer back.
- **Designed (the founder's directive, July 2026): The Next Wave of rooms.** Twenty-nine new room designs across four aspects (physics, deep mathematics, fun-first, cosmic), produced by four parallel creative research passes and recorded in `ROOMS.md` under "The Next Wave": each with its rule, gasp, verb, sonification, reveal, and honest CPU feasibility, deduplicated and ranked by wow-to-build. The first eight (Sandpile, Chladni Figures, Ripple Tank, Coffee Cup, Ford Circles, Zeta Walk, Starbow, Slingshot) are all wow-5 for build-1-to-2 and add cross-room resonances the catalog lacks (the cardioid triangle, the Lorentz pair). Designed, not built: the review-stack rule stands, each room faces the full Definition of Done, and every non-textbook reveal claim carries a source pending mathematician sign-off.
- **Tracked follow-ups (from the July 2026 bug-hunts and the two playtest rounds, see `docs/PLAYTESTS.md`):** the Lorenz divergence readout reworked to be born from a tiny perturbation and climb monotonically (the Storm; today it reads a two-point distance that starts high and wanders); predict extended to let a mind predict the *rate* and see the *shape* of its error, not a bare score (Sage and Data converged); a reactive room whose motion answers being watched, a predator-prey pulse for the instinct-only mind (the Xenomorph); the cross-room identity bridges the reveals do not yet name (Ramanujan: the logistic map is the Mandelbrot set along its real axis; the cardioid shared by Times Tables, Mandelbrot, and Epicycles); and the remaining low-severity persistence durability items surfaced by the round-2 audit (a genuinely atomic Windows replace to close a transient missing-file read window, a parent-directory fsync, and cleanup of orphaned temp/lock files on error paths), none of which corrupt data given the delta-merge model. Resolved since these were first listed: the Cairn reciprocity whisper, the L-System growing upward, the daily-seed midnight race, the daily-streak regression, and fast crash-lock recovery are all built (`CHANGELOG.md`); and the CPU render-performance cliffs a round-3 audit measured at maximized-window sizes are retired by the time-budgeted adaptive live-render resolution (render smaller, integer-upscale, exports and GPU paths untouched; measured on the dev laptop at 2560x1440, the Mandelbrot CPU fallback went from 939ms to 28.8ms per frame end to end, with Julia at 78ms and Voronoi at 60ms before the cap and every capped room now inside the 33ms room-render budget, `CHANGELOG.md`).
- **Then (the panel's remaining list, see `PANEL.md`):** juice in the window games (per-action flash, shake, and chiptune ticks); mouse support for every window game; munch rule variety and an aliens base ramp (depth where play repeats); the Open Problems wing; further-reading citations unlocked with deep cuts; era grain and Show crossfade; the music visualizer; full Share v1 beyond the built P-key PNG postcard; Engine B (the radio); GPU paths; gamepad; a visit-spark cap per room (anti-grind); and an MCP 2026-07-28 compatibility pass once the final spec target is selected after the scheduled July 28, 2026 publication.

## Pre-1.0 (the 0.x line): earning the right to 1.0

### 0.1 Public Foundation

**Goal:** establish a reproducible, honest, and safe public baseline.

- Keep the Rust workspace, headless core, app, CLI, MCP server, GPU adapter,
  and audio adapter buildable from a clean checkout.
- Publish the Apache-2.0 license, contributor rules, architecture map, current
  limitations, and one direct path to run the app.
- Enforce formatting, Clippy with warnings denied, tests, locked builds,
  coverage, house style, supply-chain policy, and the three-OS compile matrix.
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

### 0.3 Tactile Alpha

**Goal:** prove depth before expanding breadth.

- Select five flagships spanning geometry, chaos, emergence, chance, and creation.
- Give each a room-specific click, drag, or held gesture whose visual and sonic
  consequence follows the mathematics, not a decorative overlay.
- Run a short formative session after each interaction change and record where
  the action or consequence is unclear.
- Establish frame-time and input-latency baselines for those rooms.
- Give Formula Jam three legible ways to begin: manual expression entry,
  curated Random, and an Auto set that changes about every 21 seconds at phrase
  boundaries. Add a dismissible, recallable help overlay and pause Auto on edit.

Owner docs: `ROOMS.md`, `INTERFACES.md`, `SOUND.md`, `STUDIO.md`, `QUALITY.md`.

**Exit criterion:** five strangers can discover the main action in each flagship
without instruction, can describe what changed, and no flagship exceeds its
declared frame or input-latency budget on the reference machine. A first-time
Studio player can start Random or Auto, dismiss and restore Help, edit the shown
expression, and understand how to return to manual control.

### 0.4 Understanding Alpha

**Goal:** determine whether play produces a durable model, not only a striking frame.

- Complete predict-then-reveal on the flagships, with a prediction or
  construction before an insight is counted as learned.
- Test immediate explanation and delayed recall with a small, documented study;
  report negative or mixed results without reframing them as wins.
- Add source provenance and an independent math-review checklist to every
  flagship Reveal.
- Keep progression subordinate to autonomy: no streak loss, required grind, or
  reward that gates the mathematical toy.
- Prototype an opt-in, player-owned MCP experience journal: timestamped room
  encounters, creations, self-authored connections, and optional self-reported
  affect. Make it inspectable, editable, exportable, and fully erasable before
  using it for return-session continuity. Do not infer consciousness or private
  emotion from the record.

Owner docs: `PEDAGOGY.md`, `INSIGHTS.md`, `PROGRESSION.md`, `RESEARCH.md`,
`DIGITAL_DEVELOPMENT.md`.

**Exit criterion:** the flagship cohort shows a predeclared improvement in at
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
- Add MSRV, documentation, packaging smoke, and crash-recovery checks.
- Evaluate major dependency upgrades separately, with migration notes and
  before-and-after performance evidence.

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
- Complete remappable input, keyboard-only navigation, focus handling,
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

---

## The mantra

**Every screen answers your hand. Every answer reveals the math.**

The near-term stack, adopted from the July 2026 external review
(`docs/REVIEW.md`): (1) Times Tables as the gold-standard interactive room;
(2) the input/verb/variation substrate (RoomInput, not one-shot pokes);
(3) six first pokes, now generalized into all 31 catalog rooms with verbs;
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

The cycle-by-cycle build log has moved to `CHANGELOG.md`, which records every
increment in full. This roadmap stays forward-looking: what is done (above),
where we stand (next), and the ordered path to 1.0.

## Where we stand (reviewed 2026-07-12)

The package remains **0.1.0 pre-alpha**. Its capability breadth is unusually
large for that number: 31 catalog rooms, 11+ games, six sims, three faces, 29 MCP
tools, deterministic creation and persistence, and 992 passing tests. The first
public CI run passed every required job on 2026-07-11, including the three-OS
compile matrix. Breadth is not release evidence. No calibrated method supports assigning completion
percentages to subjective 1.0 gates, so this scorecard records evidence instead.

| 1.0 gate | Evidence today | Missing evidence or work |
|---|---|---|
| Complete coherent collection | 31 catalog rooms across 10 wings are built | A coherent cold start, pacing, keep-or-cut review, and several planned signature rooms |
| Every room earns its place | Every catalog room has a verb, variation, image, and motif | Stranger discovery, room-specific depth, held input where useful, and per-room human scorecards |
| Full sensory identity | Four Eras, deterministic synthesis, chiptune, and two GPU fractal paths are built | HDR post-stack, congruency review, accessibility controls, audio separation, and human sensory review |
| Three faces are genuinely good | App, CLI, and MCP paths are implemented and tested locally | Independent usability sessions for each face and real execution off Windows |
| Meta and lore are alive | Journey, levels, trophies, resonances, hidden content, and the Cairn are built | Evidence that they deepen curiosity without controlling play |
| Real creative surface | Studio expressions, `.num` serialization, links, plotting, animation, and singing exist | App reopen, local gallery, fork/remix, safe share preview, and clean-install round trip |
| Rigor and care are provable | 992 tests, 91.10% measured line coverage, Clippy, style, and supply-chain CI | Independent math review, MSRV, accessibility, real-hardware soak, and artifact provenance |
| It plays like a game | Games, dailies, scores, Gauntlet, boons, and progression are built | Observed voluntary return play and evidence that progression does not crowd out the instrument |
| Beautiful and honest throughout | Tracked screenshots and deterministic renders exist | Screen-by-screen review, perceptual regression, representative human judgment, and removal of every unsupported claim |

**Immediate critical path:**

1. Keep the 0.1 public gate green on every public commit.
2. Run the 0.2 hallway test with strangers before adding breadth.
3. Deepen five flagships for 0.3 using those observations.
4. Test understanding and retention for 0.4 rather than inferring learning from engagement.
5. Build sensory identity and accessibility together for 0.5.

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

- **Phase A, the keystone.** The prediction wager wired into a five-beat
  engineered-aha reveal, on Times Tables first (the cardioid-to-Mandelbrot morph
  as the worked example), with insight-collection gated on the generation act,
  and the same predict-then-reveal verb exposed over MCP as compression progress
  for digital minds. One mechanic seeds the understanding, curiosity, mastery,
  and creation loops at once. Owner doc: `PEDAGOGY.md`. Moves "every room compels"
  and "meta and lore alive."
- **Phase B, the glow pipeline.** The GPU post-stack (HDR bright-pass bloom,
  ping-pong phosphor persistence, tonemap, the sample-lattice Era filter) as one
  systemic pipeline every room inherits, then the Sensory Bus (one event stream,
  both renderer and synth). Owner doc: `SYNESTHESIA.md`. Directly retires the
  "full sensory identity" (78%) and "beautiful and honest" (75%) gaps, since the
  documented look currently exists only on paper.
- **Phase C, the game spine.** Constructions (a par, an elegance histogram, and a
  ghost of your past self per room) and the Constellation redesigned as a
  Rumor-Mode discovery map the daily route traverses. Owner docs:
  `CONSTRUCTIONS.md`, `CONSTELLATION.md`. This is what makes "plays like a game"
  (80%) real: a catalog you play through, not a gallery you wander.
- **Phase D, the creator loop.** Close make-share-remix on the `.num` capsule:
  app-side reopen, the room-manifest capsule, the one-button share bundle, a
  local gallery with one-keystroke fork, and generous lineage. Owner doc:
  `CREATOR.md`. Lifts "real creative surface" (68%) from a save path to a loop.
- **Phase E, the catalog deepens.** The cheap-and-gorgeous classical-geometry and
  sonification-first batch, the causal insight-chains, and the scope-flagship
  (the Studio Function Painter). Owner doc: `ROOMS.md` (the Awe Engine wave).

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

- **The quality loops (`QUALITY.md`):** the six automated and semi-automated test/eval loops run continuously from 0.1 on. This is the umbrella for everything below.
- **Beauty QA:** every build, screenshot random frames from each room and Era through the visual-regression suite. Any ugly frame is a bug. Before 1.0, add human **screen-by-screen QA rounds** of the app: walk every screen and state (each room and Era, the menu, the games, the Studio, The Show, overlays, HUD), capture and review screenshots against the beauty bar, and refine from the evidence (`QUALITY.md`).
- **The hallway test, and diverse focus groups before 1.0:** re-run the five-strangers test at every gate, scored into the per-room Fun Scorecard, awe outranks every other metric. Before 1.0, run diverse, creative focus groups that cover all three faces on their own terms (the MCP and CLI faces get their own sessions and QoL bar, not only the app), that are **intentionally not only English speakers** (the universal-translator thesis must hold for a real non-English speaker, or it is a release blocker), and that **include children** (a kid must play and have fun with no instructions). See `QUALITY.md`.
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
