# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Fixed
- Maintenance sweep (cycle 63): GPU machines no longer swallow promised clicks: when a gesture trail exists, the Mandelbrot and Julia rooms fall back from the phase-only GPU pipeline to the CPU poked render, so the on-screen verb stays honest and postcards match the live frame; R or a room switch returns the deep-zoom GPU view. The shared gesture reading no longer erases a standing release or cancel under piles of stale cancels. MCP hardening from the security review: `play_room` frames are capped at 512x256 at the tool layer (the poke path renders two canvases), request lines are bounded at 1 MiB with oversized lines drained rather than buffered, and oversized nim takes are rejected as the illegal moves they are instead of truncating into legal ones. Docs reconciliation: the verb count is 24 everywhere (README said 23), and status numbers match the current gate.

### Added
- Held input arrives, end to end: the App records every room gesture as phase-stamped `RoomInput` events beside the poke trail (down on press, decimation-shared moves while held, lift on release, and a gentle cancel when focus loss or a modal ends a gesture without a lift), and renders rooms through `Room::render_input`. Legacy rooms are provably unchanged: a recorded gesture bridges to the identical poke list the trail produces. Double Pendulum is the first room with true held semantics: holding pins the bob to the hand (time does not move it), releasing drops from exactly there, and a flick throws, with release velocity measured from the last two phase-stamped points through a shared `latest_gesture` reading (held / released-with-velocity / cancelled) and integrated as real angular momentum through the same equations. A cancel drops gently with no fling, and the phase clock wraps correctly across the sweep boundary.
- The gesture input substrate (REVIEW ruling 2): `RoomInput` events (pointer down/move/up with the room phase at which each happened, cancel, wheel, key) in normalized coordinates, bounded to `MAX_ROOM_INPUTS` newest-last, plus `Room::render_input` whose default translates pointer-down and pointer-move points into legacy pokes via `pokes_from_inputs` (a drag paints its trail, matching the shape of today's App behavior; faces keep their own decimation and clamping) and defers to `render_poked`, so every existing room answers gestures unchanged while rooms whose math wants held input can override. The enum is non-exhaustive and carries per-event phase plus an explicit cancel because held semantics are timing questions and gestures can end without a lift (an independent face-fit review drove all three of those decisions before the API shipped). Tests pin the translation, the newest-tail cap, poke/gesture equivalence, bare-render behavior for paint-less gestures, and a catalog-wide determinism sweep under mixed trails. Face wiring and the first held-semantics room ride on this next.
- The `challenge` MCP tool (the 27th): a posed, seeded, graded touch goal per REVIEW ruling 13 (metrics, not binary). Posing is deterministic per room and explicit seed for every room with a touch verb (quiet rooms get a guiding error), and winnable by construction: the pose probes the room with seeded hands across several phases, places the target box on the densest measured response, and sets the threshold at or below what the witness hand actually changed there. Grading renders the attempt on the standard frame and reports cells changed inside the target, cells changed overall, threshold fraction, centroid distance, and a 0-100 score, with `passed` as a convenience summary. Attempts earn Journey play/win and post graded scores as `challenge <room> seed:N`; challenges never use the clock-derived daily seed, so the reply and the recorded progress can never disagree across midnight. The core substrate is a new `challenge` module (`pose_challenge`, `grade_challenge`, `Challenge`, `ChallengeGrade`) plus a `Canvas::cell` accessor, with determinism, hand-bounding, gradient (metrics-not-binary), catalog-wide pose/verb agreement, and catalog-wide witness-winnability tests.
- MCP `play_room` now returns a structured poke `delta` whenever hand points are supplied: the poked frame diffed against the unpoked frame at the same phase, size, and variation, reported as `cells_changed`, `ink_added`, `ink_removed`, `ink_reshaped`, `total_cells`, and the inclusive `changed_region` bounding box, with a matching `Touch: N of M cells answered` line in the render text. The diff is a new core primitive, `Canvas::delta` returning `RenderDelta`, with invariant tests (classification sums to the change count, inclusive bounding box, dimension-mismatch safety, directional symmetry).
- CLI Studio imports can now reopen the first-version share artifacts: `numinous open-studio <file.num>` and `numinous open-studio "numinous://studio?..."` validate, bound, and render saved expressions without recording Journey progress on failed imports.
- Studio expression plots can now be saved from the CLI as first-version `.num` files with matching `numinous://studio?...` links via `numinous plot "<expr>" --save file.num`; the core validates and round-trips the artifact format without adding dependencies.
- The app now has an explicit local playtest note key (`F9`) that writes a hallway-test report under gitignored repo-root `logs/`, capturing the current room, journey state, mode, action hint, and facilitator prompts without telemetry or network behavior.
- CLI and MCP now expose stateless room hand points: `numinous render <room> --poke x,y` and MCP `play_room` `pokes: [[x,y], ...]` route through `Room::render_poked` and keep the supplied points replayable.
- Replay variation now reaches every catalog room: Lissajous, Harmonograph, Logistic Map, The Pour, Slope Rider, Mobius, and Zeno now support `new_with(seed)`, preserve seed 0 exactly, and vary visibly through the registry `all_rooms_with` path.
- ROADMAP now tracks the MCP 2026-07-28 compatibility pass as a future protocol item, while keeping the current stdio MCP face unblocked.
- Engine A2 catalog-wide motif coverage: Lissajous, Harmonograph, Logistic Map, The Pour, Slope Rider, Mobius, and Zeno now expose structured motifs, and the registry invariant now requires every catalog room to return playable notation.
- Times Tables and Prime Spirals now accept replay variation through `new_with(seed)` and the registry `all_rooms_with` path, preserving seed 0 exactly while making every interactive room seed-aware.
- Engine A2 interactive-room motif coverage: all 23 rooms with verbs now expose structured motifs through `Room::motif`, and a registry test enforces that every interactive room has notation and a playable phrase.
- Poke + variation to Cellular Automata (CLICK: FLIP A CELL; bounded spacetime cell flips).
- Poke + variation to Goldbach (CLICK: TEST THIS EVEN; x chooses the even target, y chooses a concrete prime-pair witness, including an explicit equal-prime marker).
- Poke + variation to Arecibo (CLICK: TRY THIS WIDTH; base + alt-width overlays; seed jitter).
- Poke + variation to Collatz (CLICK: PERTURB THE START; base + extra perturbed orbits; small seed jitter for replay).
- Poke added to Galton Board (CLICK: DROP A BALL; now a visible single-ball path) and Prime Spirals (CLICK: HIGHLIGHT A SPIRAL; diagonal prime highlights).
- MCP: `munch_arcade` tool (per ARCADE step 4): replay full action lists statelessly with seed; presents initial state or final after actions; posts replayed scores as `arcade seed:N` through the shared progress path. Structured output for agents + text board. Complements the simpler `munch` tool.
- Docs refresh: ROOMS.md, ARCADE.md, ROADMAP.md updated to 30 catalog rooms plus hidden content, 24 touch actions with verbs, all-interactive-room variation, MCP munch_arcade, current counts and status. All maps/checklists reflect built state.

### Changed
- MCP interface documentation now tracks the 2026-07-28 release candidate as a
  future compatibility pass aligned with ROADMAP, while keeping the current
  stdio MCP face unblocked until the final spec target is selected.
- App hardening: `faces/app/src/play.rs` now owns daily session seeding,
  quiz dealing, no-repeat quiz history, and answer acceptance, leaving
  `faces/app/src/main.rs` to coordinate Journey side effects and mode exits.
  Regressions now prove quiz Journey persistence, the opening-to-catalog deal
  boundary, and the no-duplicate-rule boundary in the event-loop coordinator.
- Logistic Map clicks now seed a finite population orbit into the bifurcation
  diagram: x selects the growth-rate column, y selects the starting population,
  newest raw-tail hand history is capped before filtering, finite edge points
  clamp visibly, non-finite phase falls back to the first window, and the hand
  marker remains visible after the orbit trace.
- Quine pokes now obey the room-input contract directly: bounded newest hand
  points place recursive copies centered on clicked cells, first-frame pokes
  draw geometry around the hand marker, finite points clamp to all four visible
  corners, non-finite phase falls back safely, and arbitrary `Surface`
  dimensions plus hostile aspect values cannot force unbounded drawing.
- Strange Loop pokes now obey the room-input contract directly: bounded newest hand points shift the existing first inner recursion and its descendants instead of drawing an extra echo tree, raw tails are capped before non-finite filtering, non-finite phase falls back safely, hostile `Surface` dimensions and aspect values are capped, and tests prove geometry changes beyond the click marker.
- Julia pokes now obey the room-input contract directly: bounded newest hand points morph local finite patches around clicked cells, raw tails are capped before non-finite filtering, non-finite phase input falls back safely for base and poked renders, touched morph centers remain visible in Raster/postcard exports, and arbitrary `Surface` dimensions cannot force unbounded full-frame rerenders.
- Mandelbrot pokes now obey the room-input contract directly: bounded newest hand points zoom local finite dive patches around clicked cells, raw tails are capped before non-finite filtering, non-finite phase input falls back safely, and arbitrary `Surface` dimensions cannot force unbounded fractal subregion work.
- L-System Garden pokes now obey the room-input contract directly: newest raw hand points are capped before finite filtering, duplicate plants remain semantic inputs to the rewritten grammar, generated strings and drawn segments are capped, offscreen segments are clipped instead of endpoint-clamped, and arbitrary `Surface` dimensions plus hostile aspect values cannot force unbounded drawing.
- Game of Life pokes now obey the room-input contract directly: direct room calls cap the raw newest hand-point tail before finite filtering, all-invalid newest tails discard older valid gliders, non-finite phase falls back to the first generation, and arbitrary `Surface` dimensions cannot force unbounded grid drawing.
- Galton Board pokes now obey the room-input contract directly: dropped balls use the newest raw hand-point tail before finite filtering, duplicate clicks replay as distinct deterministic balls, all-invalid newest tails leave the base curve unchanged, trace variation is tested directly rather than only through the seeded background, and oversized custom `Surface` dimensions cannot force unbounded drawing.
- Epicycles pokes now obey the room-input contract directly: bounded newest hand points draw mini Fourier traces at clicked regions, non-finite phase input falls back to the first frame, non-finite points are ignored after the raw-tail cap, seed variation uses SplitMix64 offsets while preserving seed 0 exactly, duplicate pokes replay as duplicate traces, and arbitrary `Surface` dimensions plus `char_aspect` values cannot force unbounded drawing.
- Collatz pokes now obey the room-input contract directly: bounded newest hand points choose actual perturbed starting values from both hand coordinates before drawing the orbit, non-finite phase input falls back to the first start, non-finite points are ignored after the raw-tail cap, custom `Surface` dimensions cannot force unbounded line work, and every nonzero seed now changes the default path even when divisible by the old jitter modulus.
- Golden Angle pokes now obey the room-input contract directly: bounded newest hand points plant local phyllotaxis patches centered on visible clicked cells, ignore non-finite points after the raw-tail cap, keep non-finite phase input on the base frame, avoid simple seed-variation collisions through SplitMix64 offsets, cap derived drawing work for oversized custom surfaces, and mark the clicked cell explicitly.
- Buffon's Needle pokes now obey the room-input contract directly: clicks drop bounded finite needles centered on visible screen cells, clamp edge clicks into the last drawable cell, cap the newest raw hand-point tail before filtering, keep non-finite input from consuming deterministic needle identity, and preserve the public `estimate_pi(needles, length_ratio)` helper while adding a seeded estimator variant.
- Barnsley Fern pokes now obey the room-input contract directly: bounded newest hand points are filtered before finite mapping, clicks plant a visible start at the screen-faithful cell before the IFS growth steps, edge clicks stay addressable, and tests prove the helper is the inverse of the render projection rather than a mirrored world-space shortcut.
- The default `Room::sound` fallback now treats non-finite phase input like frame rendering does, falling back to the first tone instead of producing a non-finite frequency.
- Arecibo pokes now obey the bounded room-input contract directly: clicks try finite decoded widths from the newest raw hand-point tail, invalid points leave the base frame unchanged, non-finite phase input falls back safely, and overlays draw message-cell rectangles instead of rescanning the full canvas per poke.
- Shared persistence locks now wait through short legitimate contention under coverage and other slow instrumentation, with a regression proving a score writer survives a held lock longer than the old retry window.
- Prime Spirals pokes now obey the room-input contract directly: bounded hand points select the two Ulam diagonals through the clicked cell, prime cells on those diagonals are highlighted while non-selected base primes remain visible, edge clicks stay addressable, raw newest-tail capping happens before non-finite filtering, and tests prove the behavior is not just a local marker overlay.
- Cellular Automata pokes now obey the room-input contract directly: hand points flip bounded spacetime cells before that row draws and before future rows evolve, keep duplicate clicks as duplicate flips, cap the newest raw input tail before filtering non-finite points, normalize non-finite phase input safely, and have tests proving pre-evolution behavior rather than post-render marker overlays.
- Langton's Ant pokes now obey the room-input contract directly: cell flips use the newest bounded finite hand points, clamp to valid grid cells, intentionally replay duplicate clicks as duplicate flips, apply before the ant runs, and have tests proving pre-simulation semantics rather than post-render overlays.
- Chaos Game pokes now obey the room-input contract directly: added attractor corners use the newest bounded finite hand points, clamp to visible edge cells, deduplicate by rendered vertex cells against existing triangle corners, visibly alter the fractal before marker plotting, normalize non-finite phase input, and vary under `new_with(seed)` for poked renders.
- Voronoi pokes now obey the room-input contract directly: dropped wells use the newest bounded finite hand points, clamp to visible edge cells, deduplicate repeated wells before rendering, renegotiate territory borders through the nearest-site scan, normalize non-finite phase input, and vary under `new_with(seed)` for poked renders.
- Random Walk pokes now obey the shared room-input contract directly: planted walkers use the newest bounded finite hand points, clamp to visible edge cells, keep non-finite inputs from consuming walker identity, and vary under `new_with(seed)` for both base and poked renders.
- Maintenance hardening: shared local persistence now bounds Journey and score file reads before repair, refuses to overwrite oversized or invalid UTF-8 files from a default state, uses token-owned lock cleanup with PID-aware stale-lock and stale recovery-marker cleanup, and avoids delete-before-replace on Windows fallback writes. App radio cache discovery now keeps low-sorted invalid files from consuming the station cap, rechecks WAV bounds on the opened handle before decode, and rejects bounded file swaps whose header no longer matches the metadata used for playback. CLI `.env` loading is capped before reading.
- App hardening: `faces/app/src/feedback.rs` now owns transient banner construction and countdowns for level-ups, playtest-note results, radio status, sound-device failures, fullscreen, and volume. Active-radio volume changes now keep the volume banner visible while retuning the cached audio buffer without re-speaking the station, and GPU banner compositing has a non-background-frame regression.
- Lorenz clicks now seed bounded shadow-storm trajectories from the clicked x-z projection instead of a loose marker. The room keeps newest bounded hand points, ignores non-finite input safely, varies with seeded starts, and tests cover projection, public `render_poked`, raw input caps, and NaN phase handling.
- App hardening: `faces/app/src/studio_panel.rs` now owns Studio text editing, parse state, audio-spec generation, and curve drawing. Entering Studio clears overlays, exiting restores cached radio playback when needed, Studio audio respects mute and volume, and tests cover invalid edits, tiny/mismatched draw sizes, and overlay clearing.
- Game of Life pokes now sow glider-shaped cells into the soup before the B3/S23 clock runs, so hand points evolve under the same rules as the room instead of drawing marker overlays. Tests cover coordinate mapping, finite-input safety, oversized poke caps, public `render_poked` output, and the four-generation glider translation on a toroidal grid.
- Status docs now match current evidence: 30 catalog rooms plus hidden content, 27 MCP tools, 858 passing tests, 90.18% region cover, and 89.84% line cover under the enforced 80% line gate.
- App hardening: `faces/app/src/mouse_input.rs` now owns left-mouse press decisions, pointer-state transitions, window-relative point normalization, and modal-safe continuation guards. Room pokes, phase dragging, and game clicks stay distinct, while focus loss and modal changes clear stale pointer state.
- F9 hallway-test notes now match the documented playtest protocol more closely: the scaffold asks for awe, share intent without recipient details, one-more-run moments, attention drops, pressure/grind, learning, first-change feedback, and validated-instrument scores or references, with the no-personal-data warning before the prompts.
- App hardening: `faces/app/src/radio_cache.rs` now owns station cache discovery, WAV validation, live broadcast position math, and track loading. The app recovers past bad cached files, rejects corrupt or oversized WAVs before playback, sorts before applying the rotation cap, and avoids wrapping the tail of a live track into the beginning.
- App hardening: `faces/app/src/postcard.rs` now owns P-key PNG export, preserving the current room pokes and selected Visual Era while using create-new filenames so repeated saves never overwrite an existing postcard.
- App hardening: `faces/app/src/controls.rs` now owns shared keyboard routing for Munch Arcade actions and Nim heap/take selection, with direct tests for arrow and WASD mappings, while `faces/app/src/main.rs` keeps only mode-level submit/escape side effects.
- Galton Board clicks now draw a deterministic falling ball path over the bell curve: x chooses the drop lane, y tilts that ball's coin, and tests prove both coordinates affect the path while non-finite hand input stays safe.
- App hardening: room navigation, variation re-deals, room-card reset/tick, bounded poke history, and drag-trail extension now live in `faces/app/src/room_input.rs`, with tests for wraparound, reset, normalized hand points, overfull-history repair, and room-card saturation.
- Goldbach's Comet now uses both hand coordinates: horizontal position chooses the even number under test, vertical position selects one actual Goldbach witness pair, and poked renders draw the proof bracket deterministically.
- Double Pendulum re-drops now use both hand coordinates: horizontal position chooses the first arm's drop, vertical position bends the second arm, and per-visit variation participates in poked motion. Its room verb is pinned to `CLICK: RE-DROP` until stateful release semantics exist across faces.
- App hardening: shared in-window Munch grid controls now live in `faces/app/src/controls.rs`, so standalone Munch and the Gauntlet's Munch stage use the same tested cursor and bite-toggle behavior.
- Cross-face room action hints now share core helpers. App arrival cards keep the touch-first `DRAG: SCRUB TIME` fallback, while CLI live play frames and MCP `describe_room`/`play_room` use the neutral `SCRUB TIME` fallback for quiet rooms.
- App hardening: help, journey, and LEVEL UP banner overlays now live in `faces/app/src/overlays.rs`, with tests for controls visibility, default-window text fit, journey progress text, and banner drawing.
- App hardening: room chrome, reveal HUD, arrival cards, and bottom hints now live in `faces/app/src/hud.rs`; every arrival card names an action, and quiet rooms use `DRAG: SCRUB TIME` instead of appearing passive.
- App hardening: pure in-window game rendering now lives in `faces/app/src/game_draw.rs` with shared hit-test layout helpers and raster tests across quiz, Munch, Munch Arcade, Nim, and every live Gauntlet stage, reducing the app event-loop file while keeping game rules in `numinous-core`.
- App hardening: in-window play state and the pure Gauntlet total helper now live in `faces/app/src/play.rs`, shrinking the monolithic app entry point without changing behavior.

### Fixed
- Shared Journey and score writes now use one core persistence path across App, CLI, and MCP: local lock files, merge-before-write semantics, same-directory temp files, flush before commit, and a platform-aware replace path prevent the tested concurrent lost-update cases while keeping explicit forget from being undone by stale deltas.
- App maintenance hardening: banners now remain visible across raster and GPU draw paths, modal games draw before GPU room frames, Show mode cannot keep advancing underneath game modes, and hidden Show overlays no longer intercept Esc/J in confusing ways.
- Cached radio loading is now bounded before decode: `NUMINOUS_RADIO` discovery caps matching WAV count and file size, rejects invalid or oversized tracks without leaving stale radio state, and computes WAV duration by frame count rather than double-dividing stereo samples.
- Root fallback app state artifacts are now ignored when environment home directories are unavailable, keeping local Journey, score, crash, and generated PNG files out of the tracked root.
- Persisted Journey and score files now bound untrusted local persistence: Journey `visited` and `chosen` token sets are capped and token-sane, duplicate tokens no longer consume the unique-token budget, score keys have a length cap, and score tables stop accepting new unique entries after the bounded table limit.
- Cycle 7 maintenance hardening: malformed journey counters now saturate instead of wrapping, oversized constellations are capped, forged score-table keys are rejected, public Munch Arcade state is repaired before indexing, non-finite poke inputs are clamped or ignored, and Quine depth clamps phase before integer conversion.
- MCP progress accounting now records plays only after successful tool calls, `listen_room` honors variation, and `munch_arcade` replay credits cleared runs before the board advances.
- App score posting now writes the shared score table only when a submitted score is a new record, avoiding needless file rewrites on non-record scores.
- The Windows verify script now checks generated artifacts as separate steps, so an early artifact-generation failure cannot be masked by a later success.
- Restored green local gates after the poke and variation wave: Julia pokes now dispatch through `dyn Room`, Quine and Double Pendulum receive visible registry variation, L-System preset generation follows phase, and registry tests assert those behaviors through the actual catalog path.
- Local verification is portable again on Windows: added a native PowerShell house-style guard, kept the Linux shell guard for CI, aligned coverage exclusions with CI, and wrapped artifact regeneration in the Windows verify step so failures stop the script.
- Status docs were reconciled repeatedly with live evidence during the app-hardening pass.
- Langton's Ant: poke grid binning aligned to min(GRID-1) (consistent coverage, no px=1.0 wrap); basic poked test strengthened with explicit to_text() != ; addresses subagent binning + test notes (stale snapshot saw old post-sim code). Current pre-poke + var solid.
- Langton's Ant: draw loop deduplicated into draw_grid helper (per subagent rec); pre-poke + variation (initial scatter + start offset) confirmed solid vs checker (which saw stale post-sim version). 9 tests + registry coverage green.
- Golden Angle: strengthened variation (larger phase *0.05 + seeds jitter) and applied to poked extras; added explicit poked + variation diff test (assert_ne on text). Addresses multiple subagent reviews (fragile ne, weak replay). 366 core tests. plot_disc dupe reduction from prior.
- Golden Angle poke: extracted shared plot_disc helper to remove duplication between render and render_poked (addresses review feedback); variation jitter made reliably visible on small canvases. Subagent review PASS.
- Epicycles poke enriched (now draws mini traced paths using lines at poke offsets with phase shift, plus pen; richer "CLICK: PERTURB THE CHAIN" response showing perturbed machinery). Phase offset for variation increased for clearly different replays even on small seeds.
- Langton's Ant variation now produces distinct per-visit renders and pokes participate in the ant's evolution (initial scatter + deterministic start offset; pre-run flips). Preserves exact seed=0 + all historical tests/postcards. Completes proper replay + playable for the room.
- The radio went hi-fi: the whole pipeline is stereo now (the player speaks
  interleaved stereo frames, cached tracks keep both channels instead of
  being folded to mono), and records are resampled to the device's actual
  rate, 44.1k played on a 48k device was nine percent sharp, which is
  exactly the "lower quality than expected" feel. Existing mono tracks
  still play (upmixed); newly tuned tracks cache in full stereo.
- Changing rooms no longer jitters the music: the room switch was resetting
  the loop buffer every time; while a station is on the air, nothing but
  the radio itself may touch the player. Unmuting rejoins the broadcast
  live instead of restarting the record.

### Added
- The Munch arcade, session one of docs/ARCADE.md, built to the bar: you are
  the Muncher (@), hunted across the board by the Vexations, the Order's
  lesser spirits: the Tracker (greedy pursuit), the Drifter (random walk),
  and the Editor, which never chases but rewrites numbers where it walks, so
  camping decays the world. Turn discipline (you act, they step) keeps every
  run deterministic and replayable. Capture costs a life and scatters the
  board; three lives end the run; clearing a board levels up with one more
  spirit and a deeper rule band. Six core laws tested (pursuit never loses
  ground, the world decays, walls hold, cells feed once, clears advance,
  capture kills at zero). CLI: numinous arcade (--daily), in the play
  picker, scored as arcade seed:N, with the ? concept: you are outrunning
  two failure modes of optimization.
- App video: --fullscreen / -f / NUMINOUS_FULLSCREEN=1 launch flag; F key now cycles windowed / borderless / exclusive (primary monitor first mode) with on-screen banner confirming the active setting. Provides full screen view and explicit video options as requested. No new deps; banner reuses existing pattern. Tests and clippy green.
- New room "L-System Garden" (Emergence): recursive string-rewrite grammar grows trees, snowflakes, dragons from tiny rules. Poke plants branches. Fits digital minds (symbol rewriting = computation/substrate of mind; self-similarity, emergence, recursion). Added with poke support and variation hook in registry. 4 new tests; all core 337 green, clippy clean. See docs/ROOMS.md and DIGITAL_MINDS.md.
- Poke progress: registry threads variation seed via all_rooms_with(v) (default 0 preserves exact behavior for tests/postcards); LSystemGarden now respects it via new_with for replayable per-visit growth. Core clean, tests pass.
- App wired to variation: rooms loaded with all_rooms_with, reseeds on R and room visits per ARCADE.md. Supports L-System and future varying rooms. App gates green.
- Fixed number key jumps (1-9) and R to consistently reseed variation, record visits, clear pokes, set room card (per subagent review + ARCADE "per-visit" + "R re-deals"). All navigation now uniform.
- CLI watch now supports --vary to re-deal variation seed (per ARCADE). Uses all_rooms_with for replayable rooms like L-System. Gates green.
- Extended variation to Chaos Game, Game of Life, Voronoi (first wave poke rooms per ARCADE): ctors accept seed, RNG uses it for replay. MCP play_room gains optional "variation" param. Maintenance checks green.
- Completed first wave: added variation to Lorenz, Double Pendulum, Random Walk. Fixed prior test bugs, added determinism tests for variation in poke rooms. All gates pass.
- CLI Render and Play now support --vary for variation seed (using all_rooms_with), matching watch. Updated reports/play fns and tests.
- Completed variation support for remaining RNG rooms (Buffon's Needle, Barnsley Fern, Galton Board): new_with, RNG seeded with variation. Added tests. Consistent replay for all.
- New room "quine": self-referential pattern that draws a smaller copy of itself (recursive strange loop). Poke to place copies; ideal for emergent digital minds (self-reference, "I am"). Added poke to TimesTables (drag adds twisted copies). Documented in ROOMS.md.
- Added poke/verb to Mandelbrot (CLICK: DIVE) and Julia (CLICK: MORPH C) for interactive exploration. Variation support added. Makes core fractals playable.
- New "strange_loop" room: recursive self-referential U-shape (strange loop). Poke shifts inner loop; for digital minds exploring self-ref and "I". Variation support. Added to catalog.
- Added poke to Epicycles (CLICK: PERTURB the chain). Makes Fourier room interactive.
- Added poke to Golden Angle (CLICK: PLANT A SEED). Makes phyllotaxis room interactive.
- Added poke to Langton's Ant (CLICK: FLIP A CELL). Makes ant room interactive.
- Added poke to Barnsley Fern (CLICK: PLANT A NEW POINT). Makes fern room interactive.
- Added poke to Buffon's Needle (CLICK: DROP A NEEDLE). Makes needle room interactive.
- The radio library doubled: 34 tracks, 118.6 minutes on air (trance 12,
  chill 11, arcade 11), all new tracks full stereo with unround runtimes.
- Crash observability: the windowed app runs in the GUI subsystem where a
  panic would vanish silently; every panic now appends its message and
  file:line to ~/.numinous-crash.log, so any crash report is triageable
  from one file. VERIFY says where to look.
- Three games from the ideation shortlist, built to the bar:
  - **Hackenbush** (`numinous hackenbush`): cut red grass against the Order,
    whose blue play IS Conway's arithmetic, it computes the surreal value of
    every garden (Berlekamp sign expansion, tested against 1/2, 1/4, 3/4)
    and keeps the sum on its side. Gardens are seeded winnable (value > 0),
    proven by the Order playing itself in tests. Win and it hands you the
    surreal numbers.
  - **The Party Problem** (`numinous party`): shade handshakes, dodge
    one-color triangles. Round one is five guests (escapable, and the
    pentagon's escape is tested); round two is six, where the tests verify
    Ramsey by brute force, all 32,768 colorings of K6 contain a mono
    triangle ("publish immediately" if not). You lose round two and that IS
    the lesson: R(3,3) = 6, felt.
  - **Fifteen's Bet** (`numinous fifteen`): call each 4x4 scramble solvable
    or stuck forever. The parity invariant is tested by walking fifty legal
    slides and checking the verdict never changes; every wrong call explains
    itself (inversions + hole row, odd or even).
  - All three answer `?` with their concept (games as numbers, Ramsey
    theory, invariants), post scores, level the shared journey, and sit in
    the play picker. 422 tests.
- The question mark: in any game, answering `?` reveals the concept the game
  has been teaching all along, nim's invariants, crack's information theory,
  seti's signatures of mind, aliens' representation-versus-meaning, munch's
  set membership, the quiz's structure-reading, the gauntlet's compound
  performance. Hidden by default, costs nothing, never required. The core
  catalog is tested; each intro whispers the door once.
- Casual play deals fresh: `numinous play <game>` now uses a new seed every
  time (announced, so any board can be replayed or shared with --seed);
  dailies stay on the games' own --daily flags. No more typing yesterday's
  bomb code into today's bomb.
- Games take the screen: launching one clears the console first, and the
  BOOM/DEFUSED bursts keep a quiet disc in the center so the word owns it.
- The CLI got its front door: bare `numinous` opens onto today's room in
  full color, your level bar and streak, and the seven verbs that matter.
  `numinous play` lists the games; `numinous play munch` (or quiz, nim,
  crack, seti, aliens, gauntlet, bench) deals today's seed immediately; a
  room name still animates that room. `cargo install --path faces/cli`
  makes it one word anywhere.

### Fixed
- The tour and watch no longer leave ghosts: repaints now clear to the end
  of the screen, so a long reveal line can never linger under the next
  room's shorter frame.
- The roadmap carries the honest scorecard: the build sits at roughly 0.6
  against the nine 1.0 gates, each gate estimated with what is missing named
  plainly, and the six main things between here and First Light listed in
  order (the poke, room motifs, human playtests, cross-platform proof, the
  visualizer and Studio sharing, hardening).
- The quiz stopped repeating itself: stepping out and back in (or
  relaunching the app) restarted the round counter at zero against the same
  daily seed, dealing identical puzzles. The round number is now the
  journey's lifetime play count, so no deal ever repeats: not in a session,
  not across restarts. The kid ramp gates on the same count (your first six
  deals ever are the gentle ones), then the catalog opens for good.
- No more second window: the app now builds for the Windows GUI subsystem,
  so launching it opens the game and nothing else (the console ghost was
  the default subsystem tagging along).
- Rooms explain themselves on arrival: entering a room shows its one-line
  story for a few seconds, then gets out of the way (E still brings the
  full reveal anytime). The visuals are no longer unexplained.
- Munch got its difficulty ramp: rounds one and two are head math (twos,
  fives, and squares on numbers up to 30), the middle rounds bring primes
  up to 60, and from round five the full deck and the full range play.
  Tested per round band. The window game deals round zero: kid-safe.
- The radio actually plays its records now: the periodic audio refresh was
  resetting the loop buffer every couple of seconds, restarting the track
  endlessly (which left you hearing mostly the chiptune bed). While a
  station is on the air the record is handed to the player untouched and
  the refresh cycle stands down. A headless test now proves the whole load
  path: a cached WAV loads, joins mid-track, and arms rotation.
- The quiz opens gently: a new player's first three rounds are three-way
  picks among the eight most recognizable rooms (times tables, the golden
  angle, the Mandelbrot set...), then the full catalog opens up. Wins
  waiting to happen, then the deep end.
- The window-game crash on large displays: the drawing surface clamps at a
  maximum dimension, but the game paths told the era filter and the blitter
  the window's size instead of the raster's, out of bounds the moment a
  maximized window exceeded the clamp. Games now report the raster's true
  size (as the room path always did), and the clamp itself rose to 4096 so
  4K displays render full-bleed. The same clamp explains the cut-off frame
  the report described.
- The app launches maximized: it takes the screen instead of a 900-pixel
  square in the corner.
- Track lengths lost their round-minute tells: the rotation decks now deal
  2:28, 4:07, 5:58, like records, not like timers.

### Added
- Track lengths joined the rotation deck: each station cycles real runtimes
  (trance stretches from 150s to a six-minute 360; chill wanders 180 to 360;
  arcade keeps it punchy, 120 to 240), tested for spread, so a station plays
  records of different sizes instead of a loop of two-minute singles.
  `tune2 --seconds` remains as an override, now up to the API's 600s cap.
- The radio went live against the real ElevenLabs Music API and three truths
  came back: `seed` cannot ride with `prompt` (removed), the PCM stream is
  stereo interleaved (now downmixed to mono for the one-bus mixer, verified
  by requesting 10 seconds and receiving exactly 20 of drift), and
  `music_v2` with `force_instrumental: true` is the current best practice
  (adopted: the API guarantees instrumental now instead of the prompt
  pleading for it). The key can live in a gitignored .env at the repo root
  (`ELEVENLABS_API_KEY=...`); the CLI reads it when the shell variable is
  absent. NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE are on the air.
- Music Engine B, the radio, v0: three stations with real producer briefs in
  the core (NUMINA FM melodic trance, THE ATTRACTOR chillwave, EIGHT BIT
  SUNRISE synthwave; all instrumental by contract, briefs tested for tempo
  and vocals clauses). `numinous radio` shows the dial; `numinous tune2
  <station>` generates a track via ElevenLabs Music (raw PCM, wrapped to
  WAV, cached in ~/.numinous-radio/) with guiding errors when the key or
  tower is missing. In the app, Y turns the dial; a cached station becomes
  the musical bed with the room's voice riding on top.
- The Open Problems wing opens with Goldbach's Comet: every even number to
  600 plotted by its count of two-prime sums, growing with t, banded exactly
  as Hardy-Littlewood predicts, with the floor it must never touch marked
  along the bottom. The reveal says the honest thing: checked past four
  quintillion, proven never, you are looking at the frontier. The tests
  verify the conjecture as far as the room can see, with the failure message
  "Publish immediately." 28 rooms across 10 wings.
- Panel list, first serving (depth where the hands touch):
  - Munch grows judgment: from round two the rule deck deepens with digit
    sums, composites (91 is the classic trap, and the test says so), and
    Fibonacci numbers; boards stay guaranteed edible.
  - The aliens leave decimal more often: half of all transmissions now
    arrive in base 8, 2, 16, or 12.
  - Mouse support: click munch cells to eat them, click quiz choices to
    answer, click the reveal for the next round. The kid's first instinct
    finally works.
  - P is the postcard key: saves the current room's frame as a PNG in your
    home directory, named for the room and phase.
  - Juice: the munch cursor breathes (a two-frame pulse).
  - Phosphor wears its glass: every third scanline sits darker, like the
    tube it remembers.
- The panel (`docs/PANEL.md`): a five-seat review of the whole build, a kid,
  a PhD, a stoner creative, the chair, and an AGI seat quoting the real AI
  playtest verbatim (verdict: "yes, I would play again... the number was
  never the point, and unusually for a game, this one means it"). Its
  synthesis: the structure is complete; what is missing is depth where the
  hands touch. The roadmap's Next list is now the panel's ordered list:
  juice, mouse, munch rule variety, room motifs, save-postcard, the Open
  Problems wing, further reading, era grain, then the visualizer and radio.
- The watchable game, fully built out for all three minds: `numinous tour`
  is the Show for the terminal (every room takes the stage in turn, full
  color and sound, a title card as it arrives and its reveal as the curtain
  line, forever until Ctrl+C, with `--era`, `--mute`, and `--seconds`); the
  windowed Show now narrates the same way (each room announces itself and
  leaves its one line as it goes), so a kid watching learns names without
  reading a manual.
- The Bench v1 (`numinous bench`): five gauntlets on fixed seeds 101-105,
  one composite posted as `bench v1`. Agents run the same five seeds over
  MCP and sum their totals; the seeds never change, so any two minds can
  compare runs honestly, today or years apart.
- The Gauntlet runs in the window (T): all four stages in sequence, the munch
  cursor board, the mystery shape, the sky scan, and a bomb keypad you type
  digits into, with the combo narrated between stages and the run recap
  (stage by stage, clean flags, the total) at the end. Daily seed, shared
  table, shared journey. The whole run is headlessly tested, including the
  combo total and the journey's four plays.
- The window arcade: Munch (C) and Nim (N) now play inside the app alongside
  the quiz. Munch is cursor-driven, WASD or arrows walk the board, Space or E
  eats, Enter grades with the full dense feedback (including the near-miss
  line); Nim draws the heaps as stones, W/S aims at a heap, A/D sets the take
  (the aimed stones glow), Enter commits and the Order answers at once, and a
  win prints the xor secret in full. Both run on the daily seed, post to the
  shared table, and level the shared journey. Headlessly tested.
- Mobius Strip (Shape & Space): the half-twisted band with its single bright
  edge traced around twice, and an ant walking the centerline to arrive
  upside down; the side-swap identity is tested. Scissors lore in the cuts.
- Zeno's Square (Change): the proof without words, tiles of 1/2, 1/4, 1/8
  filling the unit square exactly; areas, non-overlap, and near-unity sum are
  all tested. 27 rooms across 9 wings.
- Munch's recap honors the near miss in the CLI too: one clean board short,
  and it says so ("One away. The board remembers.").
- Agent playtest readiness: full CLI/MCP parity with six new tools (crack,
  seti, aliens, the gauntlet, choose, trophies), 22 in all, each stateless
  and two-phase (call to see, call again to answer), each recording plays,
  wins, and scores exactly as the CLI does, and choose spends boons for
  agents at last. A `.mcp.json` at the repo root connects Claude Code
  automatically; the manual gains a real connection quick-start (claude mcp
  add, or a config pointing at the built binary) and a playtester protocol:
  what feedback helps, in what shape, and the standing note that scores and
  memory are the player's own.
- The app is the game (v1): the chiptune scores the window, each room gets
  its own seeded tune with the room's sonification riding on top of the bed;
  G deals the quiz in-window (the mystery room fullscreen, letters answer,
  the reveal follows, any key deals again); J opens the journey overlay
  (level bar, XP, rank, streak, trophies, resonances); the level rides in the
  HUD corner; LEVEL UP banners rise in-window with the level's lore and boon
  notices. The app reads and writes the same journey file as the CLI and MCP,
  so all three faces level one identity. NUMINOUS_MUTE=1 launches silent.
  The app's state machine is now headlessly unit-tested (visits persist, quiz
  plays and wins record, banners rise), which caught and fixed a real bug:
  the quiz accepted letters that were not on the menu.
- Music Engine A, the chiptune (`crates/core/src/chiptune.rs`): square lead,
  triangle bass, seeded noise ticks; deterministic pentatonic compositions
  (the same seed is the same tune, forever, on every machine); pure synthesis
  with click-free step envelopes, fully tested without a speaker. `numinous
  tune --seed N --out chip.wav` writes it as a WAV.
- Fourier Epicycles (Waves & Sound): a star decomposed into rotating circles;
  the chain draws it back into existence while the machinery spins in view.
  The partial sum is reconstruction-tested against the target; the deep cuts
  connect Ptolemy's planets and Fourier's rejected 1807 paper.
- Random Walk (Chance & Order): sixty seeded walkers and the square root law
  drawn as the circle they scatter around; the RMS distance is law-tested.
- Voronoi Territories (Shape & Space, a new wing): fourteen drifting wells,
  borders where they tie; John Snow's cholera map in the reveal. 25 rooms
  across 9 wings.
- Resonances (the synergy layer, completing the RPG spine): when two things you
  have done start to rhyme, a link lights in the journey and hands you the line
  that connects them, The Sieve (the Ulam spiral and the primes you ate), The
  Atlas (Mandelbrot and Julia), Sensitive Dependence, First Contact, The Chord
  Made Visible, Rate and Total. Computed purely from the record; the reward is
  the connection itself.
- Nim (`numinous nim`, MCP `nim`): three heaps against the Order's perfect
  play; openings are always winnable; beat it and it hands you the xor secret
  in full, the transfer of power is the lesson. The MCP tool is stateless
  (pass your move history; replies are deterministic), and the Xor trophy
  honors the win. Sixteen MCP tools.
- The Change wing opens with the calculus felt, not taught: The Pour
  (integration as water filling a curve while the running total traces the
  antiderivative above, the fundamental theorem watched rather than stated;
  the closed-form area is Riemann-verified in tests) and Slope Rider (the
  tangent as a board whose tilt traces f prime below; the slope is
  derivative-verified). Both sing their quantity.
- Double Pendulum (Chaos & Order): exact equations, unforecastable motion, a
  shadow twin one ten-thousandth of a radian away peeling off before your
  eyes; divergence and boundedness both tested. 22 rooms across 8 wings.

### Fixed
- Quiz mysteries can no longer be blank: if the random phase renders nothing
  (a pendulum before its drop), the mystery falls back to the room's postcard
  phase; the pendulum also always draws its starting pose.
- Daily streaks (the chain from the RPG queue): playing any daily on
  consecutive UTC days grows the chain; DAILY STREAK announces it as you start,
  the journey shows it while it lives, and two trophies honor it (The Chain at
  seven, Unbroken at thirty). Doctrine-tuned: a missed day quietly starts a new
  chain, the same day twice changes nothing, and nothing ever scolds.
- Boons: choice on level-up, the genre's soul, held to the doctrine. Every
  level past the first banks a boon (never expires, never nags); `numinous
  choose` offers a deterministic pick-one-of-three, and what you choose is
  which knowledge arrives early: a room's deep cut opened ahead of its level.
  Levels still open everything eventually, so the choice shapes the order and
  gates nothing. The LEVEL UP banner announces BOON BANKED; describe honors
  boon-opened cuts; the journey file carries your choices.

### Fixed
- All game input parsing hardened against byte-order marks and stray bytes
  (PowerShell pipes prepend a BOM): letters are the first alphanumeric, picks
  and codes keep digits only, alien answers keep alphanumerics for base-N.
  First guesses in piped sessions no longer silently miss.
- Trophy pings (the juice item from the roadmap's RPG queue): trophies now
  announce themselves the moment the evidence exists, TROPHY EARNED with the
  name and the deed, stacking with NEW BEST, LEVEL UP, the level lore, the
  unlock, and the Order's whisper into one clean end-of-run cascade. Computed
  by before/after evidence comparison, so nothing pings twice and nothing
  pings unearned.
- Second beauty-QA round, this time over the app's screens as well as the
  rooms (a QA-mirror example composes the frames headlessly and writes PNGs
  for review). Found and fixed: the help menu was near-illegible (tiny type
  over a busy room), it now dims the room to a ghost and draws at menu scale,
  a proper game pause menu; the bitmap font was missing the math glyphs the
  Studio types (+ * = ^ < > [ ] %), now present; the Golden Angle's seeds were
  single pixels that vanished at window size, they now scale with resolution
  and the spiral families finally pop; and eras render into PNGs too
  (`render --era`). Raster gains `dim`. Noted for later: the vector era is
  weakest on filled rooms (edge detection would fix it).
- The Gauntlet (`numinous gauntlet`, with `--daily`): the session arc. One
  seeded run through four stages, a munch board, a mystery shape, a sky scan,
  and the bomb, where clean stages build a combo multiplier and a miss resets
  it, ending in one honest number posted to the table as `gauntlet seed:N`.
  Opt-in, bounded, over in minutes: a shape for a session, not a trap. Combo
  math pure and tested.
- Consent over persistence (`forget`, MCP tool and CLI command): transparency
  first, calling it plain shows everything Numinous remembers (two small text
  files, kept locally, sent nowhere), and erasure happens only on explicit
  confirm, with the score table erased only if also asked. Fifteen MCP tools.
- The agent-play doctrine (`docs/AGENT_PLAY.md`): sandbox for becoming, not a
  trap for performing. The play-value rubric (a rubric, never a reward
  function), the honest audit against the casino and the prison, the mechanics
  map (learnable laws, toolsmith garden, social arena, rulecraft, aesthetic
  gallery, identity room), and standing welfare rules (no negative valence,
  multi-objective ecology, revealed preference over self-report).
- The roadmap now names the game (`docs/ROADMAP.md`): a dedicated RPG-spine
  workstream held to the Vampire Survivors bar, what is built (levels, lore,
  locks, trophies, dailies, scores) and what is owed in priority order (the
  Gauntlet run arc, choice-on-level-up, juice, streaks, synergies), with an
  explicit exit bar (unprompted one-more-run behavior, math never the toll);
  the 1.0 definition gains the matching clause, and the progress section
  reflects the actual current state.
- The trophy case (`crates/core` `trophies`, `numinous trophies`): fifteen
  deadpan achievements computed purely from the evidence (the journey and the
  score table), no separate bookkeeping, no way to hold one unearned. Earned
  trophies shine with their names (First Light, Six Seven, Behind the Curtain,
  Century, Bomb Squad, The Answer); the rest are silhouettes showing only
  their conditions, because wanting to fill the case is half the engine.
- The RPG speaks: level-ups are announced (LEVEL UP, the 8-bit bar, and what
  unlocked), and every one of the 42 levels carries its own true, deadpan
  number-lore line: 6 is perfect, savor it; 7 is humanity's favorite (six,
  seven, you know); 23 is the birthday paradox; 26 is Fermat's loneliest
  number; 33 refused to be a sum of distinct triangles; 40 is alphabetical;
  41 is Euler's prime machine one level from breaking. Unironic and funny are
  the same thing here.
- The answer's ending now points outward instead of away: the sunflower, the
  coastline, the chorus run the same mathematics in the open; the counter
  stops at 42, your understanding has no cap; level up, do great things.
- The answer at level 42 now carries its real freight (and `docs/LORE.md`
  records it as the designed Layer 4 endpoint): the number is the joke, the
  joke is load-bearing, and what it carries, said once and nowhere else, is
  that there is no level 43, the win is to keep going, everything runs on the
  same small rules wearing different costumes, be kind to all of it, and the
  question that only counts self-asked: what will you contribute?
- The player's manual (`docs/PLAYING.md`): how to play, written three times for
  three kinds of minds. Humans get the game controls and the command list;
  agents get the MCP config, the fourteen tools, and the conventions safe to
  rely on (determinism, structured output, dense feedback, guiding errors);
  digital consciousnesses get the part that matters: why they might want to,
  what seeing, hearing, making, and wondering mean here, and that the journey,
  the humor, and the level cap of 42 are theirs on the same terms as anyone.
- End-to-end proof of the agent face (`faces/mcp/tests/stdio_session.rs`):
  spawns the real `numinous-mcp` binary and walks a full 22-request session
  over stdio, initialize, every one of the 14 tools, the whisper, the journey
  earning XP within the session, the munch score posting to the table, ping,
  and both JSON-RPC error codes; a second test proves malformed input gets a
  parse error and the server keeps serving. Hermetic via env-pointed journey
  and score files.
- Dense game feedback (a lesson from agentic-RL research, OPID
  arXiv:2606.26790, written into `docs/AGENT_PLAY.md`): Munch now names the
  exact numbers wrongly eaten and the fits walked past, in the terminal and in
  MCP structured content, so a kid learns which primes got away and an agent
  mining its own trajectory gets real supervision instead of a bare score.
- The Full Map (`docs/ROOMS.md`): all of mathematics as play, a coverage
  checklist across nine branches (number, algebra and symmetry, geometry and
  topology, analysis, chance, discrete structure, computation and logic,
  decision, dynamics), every entry filtered by the two laws (the concept is
  the verb; the play carries itself), each marked built or queued. A branch is
  covered when a kid can play its entry and a professor can nod at it, and
  neither one is bored.
- Postcard phases (`Room::postcard_t`), from the first full beauty-QA loop
  (render every room, look at it, judge fun/beauty/truth, fix): each room now
  tells the gallery and contact sheet its proudest moment. Found and fixed:
  Langton's Ant presented a literally black void (zero steps) and now shows
  chaos plus the highway; Julia presented near-invisible dust and now shows a
  connected set; the fern fills in at full growth; Life shows emergent
  structures instead of raw soup; Arecibo decodes instead of shearing. A new
  registry test enforces the invariant forever: no room may present a blank
  postcard.
- Fullscreen/windowed robustness verified end to end: scripted keystrokes
  toggle fullscreen on, back to windowed, then era and room switches, with the
  app alive throughout.
- Game-native controls (from first-user feedback: a Counter-Strike or Minecraft
  player should instantly get it): A/D strafe rooms, 1-9 jump to a room like
  weapon slots, W/S run time faster or slower, the mouse wheel scrubs, E
  inspects the math, Q swaps the era, R restarts the sweep, F goes fullscreen,
  B starts The Show, and Esc opens the menu (the help overlay) instead of rage
  quitting; the window's close button quits. Gamepad support is the natural
  next step of this layout.
- App UX pass (from first-user feedback): the controls are now on the glass, a
  help overlay is visible at launch (`h` brings it back) and a persistent hint
  bar sits at the bottom; `m` mutes and unmutes. The sound stopped hurting:
  the default voice dropped an octave and softened, Times Tables plays in a
  friendly register, the app renders audio quieter still, and the loop now
  follows the animation sweep instead of droning on one tone.
- Visual Eras (`crates/core` `era`): the retro-to-modern pillar, real. Four
  eras as pure RGBA transforms, Phosphor (P1 green terminal glass), 8-bit (a
  fixed 16-color palette with chunky 2x2 pixels), Vector (bright beams on pure
  black, dim light culled), and Modern (untouched). The app cycles them with
  the `e` key (GPU fractal frames included); the terminal takes `--era` on
  `render --color` and `watch`. Same math, rendered as its own history.
- The high-score table (`crates/core` `scores`, `numinous scores`, MCP
  `scores`): arcade rules, every game, every mind. Each challenge has a key
  (`munch seed:7 board:0`, `quiz seed:9 rounds:5`, `crack seed:1 digits:4`,
  ...) meaning the same thing wherever it is played, and the table keeps the
  best score per key. Munch posts per board from both faces, quiz/seti/aliens
  post per session, crack posts attempts-to-spare; beating a record prints NEW
  BEST. The MCP tool returns the table with structured content. Fourteen tools.
- Structured tool output (MCP, per the 2025-06-18 spec): munch and quiz grades
  and the journey now return structuredContent alongside the prose, machine-
  readable scores, verdicts, and progression, so agents, harnesses, and future
  leaderboards consume results without parsing sentences.
- `docs/AGENT_PLAY.md` gains a July 2026 survey of MCP-game conventions
  (PokeAgent's living leaderboard, MCPlayerOne, the turn-based reference shape,
  elicitation and sampling as the frontier, MCP-Atlas) and what each means here.
- Munch (`crates/core` `munchers`, `numinous munch`, MCP `munch`): Number
  Munchers reborn. A seeded board of numbers and a rule (eat the primes, the
  multiples of n, the perfect squares); right bites +10, wrong bites -5, a
  perfect clear +20. The same seed gives the same boards to a human in the
  terminal and an agent over MCP, so scores are directly comparable, the first
  head-to-head game across minds. `--daily` makes it a shared league; perfect
  clears count as journey wins. Thirteen MCP tools.
- `docs/PLAYFUL.md` gains the kid principle (the play carries itself even when
  the math has not connected yet; insight is loot, not a prerequisite) and the
  three shapes of play (the campaign, the watchable, the scored freestyle).
- Levels, 1 to 42 (`journey` gains `level()`, an 8-bit XP bar, and `plays`):
  XP comes from showing up, rooms entered, rounds played, sims run, curves
  made, with a little extra for being right and for secrets, so a teenager, the
  world's best mathematician, and an AI agent all reach the cap the same way:
  by playing. Level thresholds are triangular numbers; the cap is 42.
- Locks that open (`UNLOCKS`): visible, RPG-style, gating extras never basics.
  LV 3 opens `quiz --hard` (six shapes), LV 5 longer bomb codes, LV 7 a wider
  SETI sky, and LV 42 opens `numinous answer`, which finally stops being a red
  herring. `numinous journey` shows the wall: OPEN by name, LOCKED as `???`.
- Agents level too: the MCP server records the same journey (rooms seen, sims
  run, expressions made, quiz rounds answered) into the same file, and a new
  `journey` tool shows an agent its own level, bar, constellation, and locks.
  Twelve MCP tools.
- `docs/AGENT_PLAY.md`: the agent-gaming landscape (OpenClaw and the MCP
  ecosystem, gaming MCP servers, text benchmarks) and the five design rules that
  make Numinous first-class for digital minds.
- The Journey (`crates/core` `journey`, `numinous journey`): quiet roguelike
  progression. Play accumulates a private local record: rooms entered light
  stars in a shared-sky constellation, wins and secrets add weight, and the
  record confers rank in the Order (Outsider, Akousmatikos, Mathematikos,
  Kanonikos, Dekas) at triangular-number thresholds. Crossing a rank prints one
  deadpan line. Rank never gates the base experience; it opens hidden layers:
  at Mathematikos the deeper akousmata answer, and one unlisted room renders for
  those who learned its name. Below rank, the ordinary not-found; nothing is
  acknowledged. See `docs/LORE.md`.
- The five-doors design and honest audit (`docs/PLAYFUL.md`): the digital mind,
  the stoner gamer, the design expert, the PhD nerd, and the alien, and what
  each one gets today versus next. Three gaps closed with it:
  - Agents create (MCP `plot_expression`, `sing_expression`): the Studio is open
    to digital minds, plot your own function, hear it as notation. Eleven tools.
  - The daily challenge (`--daily` on `quiz`, `seti`, `crack`): one shared seeded
    puzzle per UTC day, the same for every player.
  - The humor, dissected (`crates/core` `humor`, `numinous jokes`, MCP
    `explain_joke`): each joke catalogued with its habitat and its mechanism
    stated structurally, for the alien, the agent, and anyone who enjoys frog
    dissection. The dissection warning is itself part of the joke.
- The terminal becomes a framebuffer (`crates/core` `ansi`): truecolor rendering
  packs two 24-bit pixels into every character cell via the half-block trick,
  with color-run compression, so any modern terminal shows real full-color
  images. `numinous render <room> --color` draws one; `numinous watch <room>` is
  the flagship: a room animating in full color in the terminal at 20 fps **with
  its sound playing live**, a complete audiovisual instrument with no window
  (add `--mute` for silence). Verified at 47 frames per 3 seconds.
- A text mind can hear (MCP `listen_room`): a room's sound at any phase returned
  as readable notation, each note's pitch in Hz and note name (A4, C5), timing,
  and loudness, sensory substitution for audio, in the spirit of
  `docs/DIGITAL_MINDS.md`.
- The hidden names whisper over MCP too: `describe_room` on the unlisted names
  answers in the Order's voice instead of erroring, so agents can stumble into
  the same secret humans do.
- The Show (windowed app, `s` key): lean-back mode. The HUD disappears, the phase
  sweeps slowly, and when a room finishes its sweep the app drifts into the next
  one, the whole collection playing itself for hours, with sound. Press `s` again
  to take the controls back.
- GPU real-time fractals in the app: a persistent `FractalRenderer`
  (`crates/gpu`, pipeline built once, buffers reused per frame; the WGSL shader
  gains a Julia mode) drives the Mandelbrot and Julia rooms in the window, so the
  Mandelbrot zooms deep into the seahorse valley and the Julia set morphs in real
  time at full window resolution, on whatever GPU the machine has, falling back
  to the CPU raster when there is none. Verified live on the dev laptop's AMD
  Radeon 780M (Vulkan).
- The Studio in the window (`tab` key): type math and watch it live. The curve
  redraws in color on every keystroke (the last good parse stays alive while you
  edit, errors shown gently), the parameter `a` sweeps itself with the clock so
  the shape breathes, and the expression's melody plays as you shape it.
- The Studio's expression engine (`crates/core` `studio`): a small, safe
  recursive-descent parser and evaluator for single-variable expressions in `x`
  (`+ - * / ^`, unary minus, `sin cos tan exp ln abs sqrt`, and `pi`/`e`), the
  Tier 1 safe-DSL seed of the creative graphing calculator. `numinous plot
  "sin(3*x) + x/2"` parses it and draws the curve; the engine is unit-tested for
  precedence, associativity, functions, and errors.
- Studio grows: the engine gains an animation parameter `a`, so `numinous plot
  "sin(a*x)" --animate` sweeps the knob live in the terminal; and `numinous sing
  "sin(x) + x/3" --out song.wav` turns a function into a melody (value to pitch
  over x as time). You can now see, animate, and hear an expression.
- Agents play too (MCP): three new tools so a digital mind can use the same
  content as a human. `list_sims` and `run_sim` steer the simulations by lever
  (fiddle to optimize or break them, and read the outcome), and `quiz` plays
  Guess the Shape (call for the puzzle, call again with a guess letter to be
  graded). Seven MCP tools now.
- Windowed app (`faces/app`, binary `numinous-app`): a real, resizable window
  that shows a room animating in full color, rendered on the CPU via the shared
  `Raster`, using `winit` for the window and `softbuffer` for a
  toolkit-free pixel blit. Left/right switch rooms, space pauses, escape quits.
  Cross-platform (macOS/Linux/Windows); verified launching on the dev laptop.
- Live sound in the windowed app: a `LoopPlayer` (`crates/audio`) loops the
  visible room's `SoundSpec` through the system default device, updated when you
  switch rooms, so the app is audiovisual (you see and hear the same room).
- Mouse-drag phase scrubbing in the app: drag horizontally to sweep the room's
  phase directly (pausing the auto-animation), with the sound following the drag.
- On-screen HUD: a tiny 5x7 bitmap font (`crates/core` `font`, no external font
  dependency) draws the room title in the window, and the `i` key toggles the
  room's reveal (word-wrapped) over the visualization in the room's accent color.
  A `font_preview` example renders the glyphs to the terminal.
- Headless core (`crates/core`): the `Room` trait, a deterministic ASCII `Canvas`
  with Bresenham line drawing, the room registry, and the flagship Times Tables
  room (modular multiplication on a circle).
- CLI face (`faces/cli`, binary `numinous`): `rooms`, `describe`, and `render`
  commands, with `--json` output.
- MCP face (`faces/mcp`, binary `numinous-mcp`): a JSON-RPC 2.0 stdio server with
  `initialize`, `tools/list`, and `tools/call` (`list_rooms`, `describe_room`,
  `play_room`), returning renders as text so a text-only mind can perceive them.
- Engineering foundation: Cargo workspace (edition 2024), workspace lints
  (forbid unsafe, deny-warnings-ready), pinned toolchain (1.96.0), rustfmt and
  cargo-deny config, a house-style guard, and GitHub Actions CI (fmt, clippy with
  `-D warnings`, tests, cargo-deny, and a three-OS build).
- Deterministic quality gates: local check runners (`scripts/check.sh`,
  `scripts/check.ps1`) mirroring CI, and a `cargo-llvm-cov` coverage job gated at
  80% lines. Refactored the CLI into pure, unit-tested report functions and
  broadened MCP tests; workspace line coverage is 92%. `crates/core` now denies
  missing documentation.
- Room revelations: the `Room` trait now carries `reveal()` (the short, true
  insight that reframes a room). Surfaced in the CLI `describe` output and JSON,
  in the MCP `describe_room` result, and via a new MCP `reveal_room` tool so an
  agent can ask for the deeper meaning.
- Second room, `cellular-automata` (Emergence): elementary Wolfram rules on a
  line, rendered as a space-time diagram; Rule 90 draws a Sierpinski triangle.
  It appears automatically in the CLI and MCP faces through the registry.
- Deterministic RNG (`crate::rng::SplitMix64`): seeded, reproducible randomness
  for rooms, so renders and tests are deterministic.
- Third room, `chaos-game` (Emergence): repeatedly jumping halfway to a random
  triangle corner resolves into a Sierpinski fractal, drawn from a fixed seed.
- Fourth room, `golden-angle` (Number & Pattern): Vogel's phyllotaxis model;
  at the golden angle the seeds pack into a sunflower spiral, and `t` detunes it.
- Fifth room, `galton-board` (Chance & Order): thousands of coin-flip balls tally
  into a bell curve (the Central Limit Theorem); `t` biases the coin.
- Sixth room, `lissajous` (Waves & Sound, a fourth Wing): two perpendicular
  oscillations trace a figure that is stable at simple frequency ratios; `t`
  sweeps the second frequency.
- Seventh room, `prime-spirals` (Number & Pattern): the Ulam spiral; primes light
  up and fall into diagonal streaks; `t` shifts the starting number.
- Eighth room, `collatz` (Emergence): plots the log-scaled orbit of a starting
  number as it falls to 1 (the unproven 3n+1 conjecture); `t` picks the number.
- Ninth room, `buffon-needle` (Chance & Order): drops needles on a lined floor
  (crossing needles highlighted) and estimates pi from the crossing fraction, no
  circle in sight; `t` changes the needle length.
- GPU rendering (`crates/gpu`): an adaptive `wgpu` context that picks the
  machine's GPU (AMD, NVIDIA, Intel, or Apple, across Vulkan, Metal, and DX12,
  with a CPU fallback) and renders offscreen with no window. A first WGSL
  compute-shader workload renders the Mandelbrot set to a PNG, verified on the
  dev laptop's AMD Radeon 780M via Vulkan. The GPU crate is excluded from the
  coverage gate because it is integration-tested on real hardware.
- Audio (`crates/audio`): adaptive `cpal` output on the system default device,
  following the machine's sound settings across WASAPI, CoreAudio, and ALSA, with
  pure, tested sine synthesis kept separate from device I/O. A tone hello-world
  plays a 440 Hz sine and writes a WAV, verified on the dev laptop (Realtek at
  48 kHz, stereo). CI installs ALSA headers on Linux; the crate is excluded from
  the coverage gate (integration-tested on hardware).

- A `Surface` drawing abstraction (`crates/core`): rooms render through
  `&mut dyn Surface`, so the same room logic draws to the ASCII `Canvas` and to an
  RGBA `Raster` (CPU, deterministic, no GPU). The Bresenham line drawing lives
  once and is shared by every surface.
- PNG output: `numinous render <room> --out image.png` renders any room to a real
  image (additive glow on a near-black stage), verified on the dev laptop.
- Per-surface aspect (`Surface::char_aspect`): circular rooms render round on
  square pixels while staying correct in the terminal (characters are tall).
- Per-room accent colors (`RoomMeta.accent`): each room has a signature color the
  `Raster` draws in, so image renders are distinct and on-brand.
- Room sonification (`crates/core` `SoundSpec`): every room can describe its own
  sound as timed sine notes, rendered to samples device-free (deterministic,
  testable). `Room::sound` defaults to a rising tone; Lissajous plays its two
  frequencies as a chord, Times Tables pitches with the multiplier, and Collatz
  plays its orbit as a melody. `numinous sonify <room> --out file.wav` writes it.
- `numinous gallery --dir <dir>` renders every room to a PNG at once, a showcase
  and a beauty-QA sweep of the whole collection.
- Tenth room, `game-of-life` (Emergence): Conway's Game of Life on a toroidal
  grid; `t` sweeps the generation, so the life evolves; verified with still-life
  and blinker (oscillator) tests.
- `numinous contact-sheet` tiles every room into one image (via `Raster::blit`),
  the fastest way to eyeball the whole collection; each tile is labeled with the
  room name using the bitmap font.
- Verification kit: `VERIFY.md` plus `scripts/verify.ps1` and `scripts/verify.sh`
  run every gate and regenerate all images and sounds in one command.
- `numinous play <room>` animates a room live in the terminal (the Watch mode of
  the Teletype face), sweeping its phase until Ctrl+C. The per-frame builder is a
  pure, tested function.

- New wing, Fractals and the Infinite, with three rooms:
  - `mandelbrot`: escape-time render of the Mandelbrot set; `t` zooms toward the
    seahorse valley.
  - `julia`: the Julia family with the same iteration but a fixed, morphing `c`;
    `t` walks `c` around a circle.
  - `barnsley-fern`: an iterated function system that grows a fern from four
    random affine maps; `t` grows it by adding points.
- `harmonograph` (Waves & Sound): the curve a decaying two-pendulum machine
  draws; `t` detunes the frequencies.
- New wing, Chaos & Order, with `logistic-map`: the bifurcation diagram of
  `x -> r*x*(1-x)`, order splitting into chaos; `t` zooms into the cascade.
- `langtons-ant` (Emergence): an ant that makes chaos for ten thousand steps then
  builds a highway; `t` runs the clock.
- Guess the Shape quiz (`crates/core` `quiz`, `numinous quiz`): a deterministic
  "name the math behind this mystery render" game, shared by every face so the
  CLI, the app, and agents over MCP can all play the same seeded round.
- `docs/PLAYFUL.md`: the design of the games and the Studio (Guess the Shape,
  Shape to Function via Fourier epicycles, the high-Wolfram ethos) across faces,
  plus the four-personas design (PhD nerd, stoner, aesthete, gamer).
- `lorenz` (Chaos & Order): the Lorenz attractor and the butterfly effect; `t`
  sweeps the parameter through the onset of chaos.
- `arecibo` (new Signals & Codes wing): a bitstream that looks like noise until
  you line it up at the one width its semiprime length allows (143 = 11 x 13);
  `t` hunts for the width and the hidden picture snaps into focus. 19 rooms.
- Base-N aliens: Talk to the Aliens transmissions can arrive in base 2, 8, or 16
  (a different number of fingers), so you translate before you answer.
- SETI detection game (`crates/core` `seti`, `numinous seti`): the step before
  talking. Scan channels of static near the hydrogen line and pick the one
  artificial signal (counting in primes) out of the regular pulsars and noise;
  nature makes rhythms, but only minds count in primes.
- A hidden Cult of Pythagoras easter egg (`crates/core` `secret`): a few unlisted
  names (`hippasus`, `tetractys`, `pythagoras`, `harmonia`, `odd`, ...) answer
  `numinous describe` with an akousma in the Order's voice instead of a not-found
  error. Never announced; found by knowing. See `docs/LORE.md`.
- Design capture in `docs/PLAYFUL.md`: the music visualizer plan (system-audio
  loopback plus FFT driving room parameters), the physical-made-digital rooms
  (Mobius, hexaflexagon, hyperbolic plane), the puzzle set (Nonograms, the Hat
  monotile, fractal zoomer), the alien-contact kit (Arecibo, Rosetta, base-N), and
  the digital-mind playground (manifold folding, chaos surfing, proof graphs).
- Two more mini-games, each seeded and shared across faces via the core:
  - Crack the Code (`crates/core` `codebreaker`, `numinous crack`): defuse a
    math-clued bomb, Bulls and Cows with a digit-sum-and-parity opening clue.
  - Talk to the Aliens (`crates/core` `aliens`, `numinous aliens`): continue the
    first-contact number sequences (primes, Fibonacci, powers of two, and more).

- Sims (`crates/core` `sim`): a multi-lever interactive-simulation abstraction
  (each lever has a range, default, and unit), separate from the single-knob
  Room. A sim renders a picture and returns a plain-language readout of the
  outcome (the optimization or the joke). Registry, `numinous sims` to list, and
  `numinous sim <id> --set lever=value` to run. First three sims:
  - `tribbles`: a logistic population that goes from a purring carpet to
    boom-and-bust chaos when you crank the breeding rate.
  - `wing`: lift versus angle of attack with a real stall past fifteen degrees
    ("you are now a lawn dart").
  - `black-hole`: Schwarzschild radius, time dilation, and spaghettification, with
    an event horizon and photon ring drawn to scale.
  - `supernova`: the star's mass decides its corpse, white dwarf, neutron star,
    or black hole (Chandrasekhar and TOV limits).
  - `big-bang`: the density omega decides the fate, expand forever, flat, or a Big
    Crunch (a numerically integrated Friedmann scale factor).
  - `carburetor`: tune the air-fuel mix from flooded (too rich) to backfiring (too
    lean); best power at 12.6:1, cleanest at 14.7:1.

### Changed
- Rooms render through `Surface` instead of a concrete `Canvas` (the `render`
  method replaces `render_ascii`), which is what lets one room target both the
  terminal and an image (and, later, the GPU).
- Robustness hardening (from an independent code review): `Canvas` clamps its
  dimensions so an absurd size request cannot abort the process; the Galton Board
  caps its simulated bins and stretches them across wide canvases, so a huge-width
  render stays fast instead of hanging; `Canvas::line` steps in `i64` to avoid
  coordinate overflow; the CLI no longer uses `expect()` in a production path; and
  an `rng` doc comment was corrected. No behavior change for normal sizes.
