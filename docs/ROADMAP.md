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

**Current release state: 0.2.0-alpha.1, Flagship Proof in progress.** The 0.1
Public Foundation exit criterion is complete on the public `main` branch. The
0.2 stranger hallway test, independent macOS and Linux app execution, and
accessibility work are still open. Later systems already present in source do
not waive those gates, and this prerelease label does not claim 0.2 is complete.

- **Done:** the headless core (`Room` trait with `reveal()`, deterministic ASCII `Canvas`, seeded RNG, registry, `verb`, `render_poked`, and variation); the CLI face (`numinous`), the MCP face (`numinous-mcp`), and the windowed app; **351 catalog rooms** plus hidden content; 6 lever-driven sims; 11+ games; the full engineering harness (edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, house-style guard, an 80% line coverage gate, three-OS CI). Current local evidence: fmt, Clippy, 2,985 passing all-target test cases plus one ignored screenshot diagnostic, locked build, Windows release gate, 95.44% region coverage, and 95.55% line coverage all pass.
- **Done (GPU and audio hello-world):** an adaptive `wgpu` context (`crates/gpu`) that picks the machine's GPU across Vulkan/Metal/DX12 with a CPU fallback, rendering the Mandelbrot set offscreen to a PNG; and adaptive `cpal` audio (`crates/audio`) on the system default device that plays a tone and writes a WAV. Both verified on the dev laptop (AMD Radeon 780M, Realtek at 48 kHz).
- **Done (rooms as images):** a `Surface` abstraction so every room renders through one `render` method to the ASCII `Canvas` and to an RGBA `Raster`; `numinous render <room> --out image.png` writes a real glowing image on the CPU (verified on the dev laptop).
- **Done (windowed app):** `faces/app` (`numinous-app`, winit + softbuffer) opens a real resizable window showing a room animating in full color, with keyboard room-switching. The start of the GUI Cabinet; verified launching on the dev laptop.
- **Done (sound):** every room describes its own sound (`SoundSpec` + `Room::sound`); `numinous sonify <room> --out file.wav` and `numinous play <room>` (live animated terminal).
- **Done (the 0.2 technical vertical slice):** the windowed app implements live per-room sound, mouse and controller input, an on-screen HUD with reveals, The Show (lean-back auto-play of the whole collection), the Studio in the window (type math, watch and hear it live), and GPU real-time fractals (a persistent `wgpu` pipeline drives the Mandelbrot deep zoom and the morphing Julia at window resolution, with CPU fallback; verified on the dev laptop's Radeon 780M). The human hallway, accessibility, sensory, controller-hardware, and cross-platform evidence gates remain open.
- **Done (content and play):** 351 catalog rooms across the wings plus unlisted hidden content, including Cult of Pi, the Conjecture Mill, the Change wing (The Pour, Slope Rider), Fourier Epicycles, the double pendulum, the random walk, Voronoi, Quine, Strange Loop, L-System Garden, Mandelbrot/Julia dives, Galton, Buffon, etc.; 6 lever-driven sims; 11+ games (SETI, Talk to the Aliens, Guess the Shape, Crack the Code, Munch, Nim with the xor secret, Hackenbush, the Party Problem, Fifteen's Bet, the Gauntlet run, and full Munch Arcade) with daily seeds and dense feedback; the Studio expression engine (`plot`, `plot --save`, `open-studio`, `--animate`, `sing`, and live in the window); Visual Eras (phosphor, 8-bit, vector, modern) across app, terminal, and PNGs; truecolor terminal rendering with live sound (`watch`).
- **Done (the RPG spine, complete):** the Journey (XP from play, levels 1 to 42 on triangular thresholds, a lore line for every level, LEVEL UP banners), locks that open (never gating basics), ranks and whispers (the Order), deep cuts unlocking at LV 5/12/24, the trophy case (18, evidence-computed, silhouettes), the shared high-score table across every game and both faces, the Layer-4 answer at the cap, and every genre organ from the priority list: the Gauntlet (session arc with a combo and one posted number), trophy pings (the case announces itself), boons (choice on level-up, where the loot is knowledge arriving early), daily streaks (the chain, never scolding), and resonances (synergies: links light when two deeds rhyme and hand over the connecting line).
- **Done (agents as peers, v2):** 29 MCP play tools plus one local broadcast consent control, with structured output and full CLI parity (every game, the gauntlet, boons, trophies, munch_arcade), including stateless nim, `forget` (transparency first, erasure on explicit consent, the welfare doctrine in `AGENT_PLAY.md`), and `munch_arcade`; `play_room` supports stateless per-call variation and normalized hand points; agents see, hear, create, play, level to 42, and post to the same score table; every play schema advertises an additive `response_mode`, with stable full tool-call results and nonexpanding compact text for eight complete structured result families; the player's manual speaks to humans, agents, and digital consciousnesses; the whole stdio face is proven end to end against the real binary.
- **Done (sound, Engine A v1):** the chiptune module (square lead, triangle bass, noise ticks, seeded pentatonic compositions, deterministic and click-free); `numinous tune` writes it as a WAV.
- **Done (soundtrack, Engine B v1):** Nick Seal made 42 tracks specifically for Numinous across NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE. High-quality V0 MP3 assets ship in `assets/radio`, the app discovers them from a clean clone, and a bounded pure Rust decoder validates, decodes, and resamples them. The archival WAV masters remain outside the repository.
- **Done (the app is the game, v1):** the chiptune scores the window (per-room seeded tunes with the room's voice riding on top); the quiz plays in-window (G: name the math, letters answer, the reveal follows); the Journey lives in the app (the CLI's own file: visits on entry, plays and wins from the quiz, explicit `JOURNEY LV` progress, `JOURNEY LEVEL UP` banners with lore, and J opens level, rank, trophies, and resonances); `NUMINOUS_MUTE=1` launches silent; the state machine is headlessly tested.
- **Done (the window arcade):** Munch, Nim, and the full Gauntlet run play inside the app alongside the quiz, cursor-driven and keyboard-native, on the daily seeds, posting to the shared table and leveling the shared journey; Mobius and Zeno's Square join the catalog. Full Munch Arcade with Vexations.
- **Done (poke + variation substrate):** Expanded pokes (all 351 catalog rooms with verbs + `render_poked`) and per-visit variation threading (registry `all_rooms_with`, app/CLI/MCP variation on each visit, default 0 exact). R now resets the current visit without silently changing its deal. Double Pendulum re-drops from both hand coordinates; Goldbach's Comet selects a real prime-pair witness; Galton Board draws bounded deterministic falling paths; Logistic Map seeds finite population orbits; and Cult of Pi repairs bounded faults in an exact-digit field. CLI `render --poke x,y` and MCP `play_room` `pokes: [[x,y]]` expose the same stateless hand-point path outside the App. All 351 catalog rooms are seed-aware today; hidden content is intentionally outside the catalog replay contract.
- **Done (Engine A2 motifs, catalog-wide):** all 351 catalog rooms now expose a structured `Motif` through `Room::motif`, so `listen_room` gets real notation and the app gets room-specific phrases instead of the generic fallback. A registry test enforces that every catalog room has a playable motif. The default `Room::sound` derives from the motif through `SoundSpec::from_motif`; rooms with a specialized mathematical sonification may intentionally override it. `listen_room` gives the ambient motif and mathematical sonification distinct text headings and maps those roles to its compatible `motif` and `notes` fields so it never presents one score as the other.
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
  across all 351 rooms, and enforce the 96-event and 64 KiB protocol budgets.
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
  remain open, so the package stays `0.2.0-alpha.1`.
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
  five games and every Gauntlet stage are pure-tested. Xbox-class Windows
  hardware is the local target; broader controller and platform certification,
  and user-facing glyph adaptation remain open.
- **Done (Cycle 138 gamepad configuration):** `~/.numinous-bindings.json` supports full controller remapping. `gamecontrollerdb.txt` is shipped with the binary as a fallback standard controller mapping.
- **Done (MCP munch_arcade):** Stateless `munch_arcade` tool for full parity, with replayed action-list scores posted under `arcade seed:N` through the shared progress path.
- **Done (app hardening slice):** app-local play state plus quiz deal/answer flow now live in `faces/app/src/play.rs`, pure game-screen rendering lives in `faces/app/src/game_draw.rs`, room chrome plus arrival-card hinting live in `faces/app/src/hud.rs`, help, journey, and banner overlays live in `faces/app/src/overlays.rs`, transient feedback banner construction and ticking live in `faces/app/src/feedback.rs`, shared in-window Munch grid, Nim heap/take, and Munch Arcade action controls live in `faces/app/src/controls.rs`, left-mouse mode decisions and pointer-state guards live in `faces/app/src/mouse_input.rs`, room navigation, re-deal, poke-history, drag-trail, and room-card tick helpers live in `faces/app/src/room_input.rs`, Studio text, parse, audio-spec, and curve drawing state live in `faces/app/src/studio_panel.rs`, explicit F9 hallway-test note capture lives in `faces/app/src/playtest.rs`, live-state PNG postcard export lives in `faces/app/src/postcard.rs`, and bounded radio cache discovery, open-handle WAV validation, live-position math, and track loading live in `faces/app/src/radio_cache.rs`. Room action copy is centralized in `numinous-core`: App arrival cards use touch-first fallback copy, while CLI live play and MCP room tools use neutral fallback copy. Tests cover shared game hit-test layout, raster output across quiz, Munch, Munch Arcade, Nim, every live Gauntlet stage, quiz daily seeding, no-repeat quiz history, answer acceptance, action-naming arrival cards, Studio chrome suppression, Studio panel editing and bounded drawing, cross-face action hints, shared Munch/Nim/arcade controls, room-input bounds, modal-safe pointer-state transitions, playtest-critical overlays, feedback banner copy/lifetimes, radio-volume banner retention, GPU/raster banner compositing, local playtest-note reports that align to the hallway-test prompts without collecting personal data, postcard PNGs that include pokes, the selected Visual Era, collision-safe filenames, bounded/sorted station cache discovery, low-sorted corrupt-track handling before the track cap, corrupt-track rejection, open-handle size rechecks, high-rate-device caps, non-wrapping live offsets, and app radio recovery after a bad cached file. The event-loop file is still a hotspot, but game rules remain in `crates/core` and the refactor is moving in small verified modules.
- **Done (persistence hardening slice):** malformed Journey and score files now parse defensively: counters saturate, constellation dimensions are capped, `visited` plus `chosen` token sets are bounded and token-sane, duplicate Journey tokens do not consume the unique-token cap, score keys are length-bounded, and score tables cap unique entries. The maintenance posture remains that progress and score files are user-editable local text, so loaders must repair or ignore malformed data rather than panic or allocate without bound.
- **Done (shared persistence writes):** App, CLI, and MCP now route Journey and score writes through shared core persistence helpers. Writes use a token-owned local lock, PID-aware stale-lock recovery, stale recovery-marker cleanup, merge-before-write behavior, bounded read-before-repair semantics, same-directory temp files with error-path cleanup, flush before commit, atomic Windows replacement retries that never move the destination aside, and a pre-opened parent-directory metadata sync after replace or explicit forget on Unix. The rename remains the commit point: a later sync failure cannot report an uncommitted delta and cause counters to be applied twice. This is an operating-system best-effort durability barrier, not a claim of hardware power-loss immunity. Tests cover concurrent Journey deltas, concurrent score records, a real Windows sharing violation with continuous readers, injected postcommit sync failure, temp and lock cleanup, short held-lock waits under instrumentation, stale deltas after explicit forget, oversized and invalid UTF-8 persistence files preserving the original bytes on write attempts, stale, malformed, and dead-process lock recovery, stale recovery-marker cleanup, current-process lock preservation, and lock drop ownership.
- **Done (the keystone, the Cairn, and the chaos readouts):** the predict-then-reveal verb (MCP `predict`, Phase A of the Exceptional Path): commit a guess of a room's own status readout at a hidden moment, then meet the truth graded as a gap with a learning-progress band, a self-owned mirror that never posts a score. The graded `challenge` tool in two kinds (touch a target box, or land the readout on a number). The Cairn (MCP `cairn` plus the core `cairn` module and the repo-tracked `data/cairn.txt`): at level 42 a mind leaves one true thing, encoded Arecibo-style into a semiprime a future mind must factor to read. And tactile status readouts across the Chaos & Order flagships (Double Pendulum and Lorenz report the divergence of two nearby starts; the Logistic Map reports its Lyapunov exponent crossing from order into chaos), so eight rooms now pose predictions. See `CHANGELOG.md` for the full detail.
- **Done (the one-line front door):** `scripts/install.sh` and `scripts/install.ps1` make setup a single copied command on macOS, Linux, and Windows: prerequisite checks that name the exact fix for anything missing, a rustup bootstrap when cargo is absent, a fresh fixed-origin source snapshot into `~/.numinous/src`, a locked release build, the three binaries plus a linked radio in `~/.numinous/bin`, PATH wiring, in-place update on re-run, and an `--uninstall` that never touches play history. Existing repository configuration, untracked source, and build caches are never trusted during update. User-bound install-root identity and link-aware deletion keep uninstall inside the dedicated root; only an exact legacy default-root shape, with or without the old marker and with explicit adoption consent, migrates. A custom legacy root must be moved aside or replaced with a new empty root. Disposable hostile-root and provenance self-tests pass on Windows and a Windows-hosted POSIX shell. Native Linux and macOS installer self-tests pass on GitHub-hosted runners. The 0.6 portable gate still owns packaged, checksummed artifacts and physical clean-machine evidence.
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
  evidence listed above; the prerelease label remains `0.2.0-alpha.1`.
- **Done (mouse for every window game):** left-click hits Quiz choices, Munch
  cells, Nim heaps and stones (commit move), Arcade cells (step toward or eat),
  and Gauntlet munch/quiz stages. Keyboard routes remain. Subjective juice and
  physical controller evidence stay open.
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
- **Next, above everything (the founder's directive, July 2026):** **rooms become playable, not watchable, and no two catalog visits are the same.** The substrate is live across app, CLI, and MCP. The legibility pass makes weak responses explicit across Life, Mandelbrot, Buffon, Prime Spirals, Cult of Pi, Golden Angle, Barnsley Fern, Mobius, Logistic Map, Zeno, Julia, Goldbach, Langton's Ant, Fourier Epicycles, Random Walk, Quine, and the Conjecture Mill. `Room::status_input` lets every face explain the consequence from the same bounded history used to render it. The automated all-room, all-game, all-screen matrix is complete at 2,913 states across all 351 registered rooms, with registry-derived inventory, nonblank, size, stale-output, deterministic opening states, every persistent game display branch, 14 controller or pause receipts, 18 explicit audio-state receipts, 12 Times Tables landmark and earned-goal receipts, default and compact immediate and delayed interaction families, ordered completed gestures, explicit active-hold release and cancel boundaries, Formula Jam half-morph receipts, semantic response checks, changed-pixel and spatial-support thresholds, support density, adjacent 32-pixel spatial-tile coherence, and minimum color change. Scenarios follow declared room verbs. The generator evaluates each room's pure mathematical consequence independently from the App's latest-gesture trail and reticle, and an aggregate diagnostic reports all catalog failures in one run. A regression rejects four isolated corner markers. This remains coarse renderer-path evidence rather than native event automation or subjective polish certification. The latest grouped QA rounds also hardened controller-visible control truth, pause isolation, CLI and MCP input boundaries, pure-EOF game exits, structured discovery, isolated MCP play profiles, and Windows PATH precedence. Next, validate arrival-card clarity with real human participants and deepen held or causal interaction wherever a one-shot response still fails the kid principle.
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
- **Done (catalog growth through 351):** invent-and-ship past the early Next
  Wave and classical cards into dynamics, number theory, probability,
  topology, analysis, theory formation, and closing gems. MCP `list_rooms` count is 351; every
  catalog room keeps motif, verb, poke, first-contact status, and reveal.
  Version remains `0.2.0-alpha.1`; product 0.2 is not claimed complete. See
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
  accepts 2025-06-18, 2025-11-25, and names 2026-07-28 for dual-stack hosts.
- **Done (panel depth pack, cycle 148):** citations unlock with the first deep
  cut on CLI and MCP; expanded wing-specific further reading; pure spectrum
  band-energy substrate for the visualizer path; adaptive Xbox/PlayStation/
  generic face glyphs with pad-name inference; phosphor soft bloom; aliens base
  ramp for denser seeds.
- **Done (visualizer path, cycle 149):** MCP listen beds expose normalized
  spectrum bands; the App draws a room-bed spectrum meter under the audio
  badge from the cached motif arrangement. OS loopback capture remains open.
- **Done (mega pack, cycle 150):** `LoopPlayer` mixed-output capture ring;
  optional loopback input when the OS exposes a mix-like device; App key O
  cycles room bed / output mix / loopback; spectrum lever mapping; Share
  sidecar notes; earlier aliens base ramp; more adaptive face glyph surfaces.
- **Then (the panel's remaining list, see `PANEL.md`):** munch rule variety is
  partly shipped; spectrum-to-room lever drive; full Share v1 packaging beyond
  PNG/APNG (GIF/MP4); cross-platform controller certification; full 2026-07-28
  wire migration after the final specification ships.
- **Done (0.4 Understanding Alpha prep):** added Source Provenance and Math Review Checklist fields to the Times Tables, Game of Life, Galton Board, and Double Pendulum flagships to anchor their learning claims.
- **Done (0.4 Understanding Alpha prep):** added an opt-in, player-owned MCP experience journal. The `Journal` tracks timestamped room encounters, creations, and connections. It is fully integrated into persistence and backed by new `read_journal`, `record_journal`, and `erase_journal` tools for MCP agents. The journal is explicitly disjoint from `forget` tool erasure, providing its own dedicated `erase_journal` path to maintain player ownership over when its contents are destroyed.

## Pre-1.0 (the 0.x line): earning the right to 1.0

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
**Goal:** Build **one** flagship room to *jaw-dropping* quality, plus exactly enough shell to frame it. The make-or-break milestone.

**The room:** **Times Tables** (modular multiplication circles), highest wow-to-build, continuous/performable, a floor-tilting Reveal (see `ROOMS.md`).

- All three layers real: **Toy** (drag the multiplier, buttery morphing), **Aha** (one small challenge), **Reveal** (the Mandelbrot card).
- Full **audiovisual polish:** the signature palette + glow, tuned musical sonification, smooth 60fps, screenshot-worthy at every frame.
- The **design system** born here (color, type, motion, sound voice, the fade-in-on-approach UI), extracted as we go so later rooms inherit it.
- **Share v1:** export the current view as an image or a short loop.
- The room is also playable via CLI (`--tui` ASCII render) and MCP (an agent can explore/play it), proving the three faces on real content.

**Exit criterion, the hallway test:** (Deferred to 0.8) Originally to show it to five people. For now, limited testing by the user validates the core experience, allowing us to progress.
**Retires the risk:** "is the core experience actually magic, or just a neat demo?"

### 0.3 Tactile Alpha

**Goal:** prove depth before expanding breadth.

- Use the selected five flagships: Times Tables for geometry, Double Pendulum
  for chaos, Game of Life for emergence, Galton Board for chance, and Formula
  Jam for creation.
- Give each a room-specific click, drag, or held gesture whose visual and sonic
  consequence follows the mathematics, not a decorative overlay.
- Run a short formative session after each interaction change and record where
  the action or consequence is unclear.
- Keep the release-profile ambient and accepted-input-to-room-raster baselines
  under the declared 33 ms p95 reference-machine budget. Native end-to-end
  input latency remains a separate real-hardware measurement.
- Give Formula Jam three legible ways to begin: manual expression entry,
  curated Random, and an Auto set that changes about every 21 seconds at phrase
  boundaries. Add a dismissible, recallable help overlay and pause Auto on edit.
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

Owner docs: `ROOMS.md`, `INTERFACES.md`, `SOUND.md`, `STUDIO.md`, `QUALITY.md`.

**Exit criterion:** (Stranger test deferred to 0.8) Limited user testing confirms the main action in each flagship is discoverable, and no flagship exceeds its declared frame or input-latency budget on the reference machine. A first-time Studio player can start Random or Auto, dismiss and restore Help, edit the shown expression, and understand how to return to manual control.

One separately consenting MCP guest can complete a flagship explore, challenge,
and reveal loop while a human follows the same causal states through the
read-only App viewer, with no private host or protocol data in the stream.

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
- **Done (July 18, 2026):** enforce the verified Rust 1.88 MSRV in CI while
  pinning the developer and release toolchain to stable 1.97.1. Packaging
  smoke, crash-recovery, and artifact-provenance checks remain.
- **Done (July 18, 2026):** migrate every direct dependency with a newer stable
  line, including wgpu 30, cpal 0.18, png 0.18, pollster 1, and ureq 3; refresh
  compatible transitive packages; pin current CI action releases; remove stale
  Dependabot migration ignores; and retain typed migration regressions. The
  migration notes are recorded in the changelog and engineering guide.
- **Open:** record comparable before-and-after GPU, audio, App startup, and CLI
  request performance evidence for the July 2026 dependency migration. Future
  major updates require both migration notes and comparable performance evidence.

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
(3) six first pokes, now generalized into all 351 catalog rooms with verbs;
(4) Engine A2 motifs for every catalog room; (5) MCP structured deltas
and challenge metrics for the same rooms; (6) one human hallway test; (7)
cross-platform run; (8) docs reconciliation.
Do not build twenty more rooms before those are done.

MCP protocol watch: the 2026-07-28 release candidate is relevant to the MCP
face, so it belongs in this roadmap as a high-level compatibility pass as well
as agent notes. Checked 2026-07-13 against the official release-candidate post
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

## Where we stand (reviewed 2026-07-18)

The package is **0.2.0-alpha.1**. The 0.1 Public Foundation exit criterion is
complete, and work is now on 0.2 Flagship Proof. The 0.2 milestone itself remains
open until the Times Tables stranger hallway test passes. Current breadth is 351
catalog rooms, 11+ games, six sims, three faces, 30 MCP tools, deterministic
creation and persistence, and 2,985 passing all-target test cases plus one
ignored screenshot diagnostic on the green release gate. Required public CI
passes locked tests, builds, and installer self-tests across all three operating
systems; physical-device evidence remains separate. Breadth is not release
evidence. No calibrated method supports
assigning completion percentages to subjective 1.0 gates, so this scorecard
records evidence instead.

| 1.0 gate | Evidence today | Missing evidence or work |
|---|---|---|
| Complete coherent collection | 351 catalog rooms are built and listed | A coherent cold start, pacing, keep-or-cut review, and several planned signature rooms |
| Every room earns its place | Every catalog room has a verb, variation, image, and motif | Stranger discovery, room-specific depth, held input where useful, and per-room human scorecards |
| Full sensory identity | Four Eras, deterministic synthesis, chiptune, and two GPU fractal paths are built | HDR post-stack, congruency review, accessibility controls, audio separation, and human sensory review |
| Three faces are genuinely good | App, CLI, and MCP paths are implemented and tested locally | Independent usability sessions for each face and real execution off Windows |
| Meta and lore are alive | Journey, levels, trophies, resonances, hidden content, and the Cairn are built | Evidence that they deepen curiosity without controlling play |
| Real creative surface | Studio expressions, `.num` serialization, links, plotting, animation, and singing exist | App reopen, local gallery, fork/remix, safe share preview, and clean-install round trip |
| Rigor and care are provable | 2,985 passing all-target test cases plus one ignored screenshot diagnostic, 95.55% measured line coverage, verified Rust 1.88 MSRV, Clippy, style, and supply-chain CI | Independent math review, accessibility, real-hardware soak, and artifact provenance |
| It plays like a game | Games, dailies, scores, Gauntlet, boons, and progression are built | Observed voluntary return play and evidence that progression does not crowd out the instrument |
| Beautiful and honest throughout | An exact 2,913-screen matrix and a 42-lens review cover every catalog room plus captured game, input-aware controller, pause, overlay, Show, Studio, reset, phase, persistent Life, audio-state, and Times Tables landmark branches | Perceptual regression, representative human judgment, uncaptured persistent states, and removal of every unsupported claim |

**Immediate critical path:**

1. Keep the completed 0.1 public-foundation gate green on every public commit.
2. Run the 0.2 hallway test with strangers in parallel with verified refinement work; do not claim the milestone until it passes.
3. Deepen five flagships for 0.3 using reproduced defects, structured review, and participant observations as each becomes available.
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
