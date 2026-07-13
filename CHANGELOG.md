# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- The Only Move is designed as a 1.x room (founder's idea, July 2026): a
  machine plays both sides of tic-tac-toe through real minimax until the whole
  game tree burns down to the inevitable draw, then declines a war-shaped game
  it has learned cannot be won. The design records its evidence boundaries
  (tic-tac-toe and checkers are solved draws with sources; chess and Go are
  not; the war game's no-win property comes from its declared payoffs, not a
  world claim), its resonances with Nim, the Party Problem, and Hackenbush,
  and its pairing with the Traveling Salesman stub as the two faces of
  combinatorial search. Full design in `docs/ROOMS.md`; placement in the 1.x
  roadmap line.
- The windowed app now holds its frame rate in heavy rooms with a
  time-budgeted adaptive live-render resolution. The measured cliffs from the
  round-3 audit (Mandelbrot's CPU fallback at 939ms per frame at 2560x1440 on
  the dev laptop, with Julia at 78ms and Voronoi at 60ms) are retired: the app
  watches each frame's real room render time and picks an integer downscale
  factor per frame (a grossly slow frame jumps straight to the predicted
  factor, mild ones climb after a two-frame streak, fast frames walk back with
  hysteresis so the factor never oscillates), renders the room raster at the
  reduced size, and integer-upscales to the window. Measured end to end, the
  worst room now costs 28.8ms per frame at 2560x1440, inside the 33ms budget.
  The HUD, overlays, and banners draw after the upscale so interface text
  stays window-crisp; exports, postcards, modal game frames, the Studio, and
  the GPU fractal path never pass through the cap. The controller and the
  nearest-neighbor upscale are unit-tested (`faces/app/src/live_render.rs`,
  `Raster::upscaled`).
- Setup is now one command on every platform. `scripts/install.sh` (macOS and
  Linux) and `scripts/install.ps1` (Windows) check what the machine needs and
  name the exact fix for anything missing, install Rust through rustup when
  cargo is absent, fetch the source into `~/.numinous/src` (git when available,
  a snapshot download otherwise), build the release binaries, install
  `numinous`, `numinous-app`, and `numinous-mcp` into `~/.numinous/bin`, link
  the built-in radio next to the executables, and add that directory to PATH.
  Re-running either installer updates in place and keeps the build cache;
  `--uninstall` (Windows `-Uninstall`) removes everything installed while
  leaving `~/.numinous-journey`, `~/.numinous-scores`, and `~/.numinous-cairn`
  untouched. The Windows PATH edit preserves the registry value kind and
  unexpanded `%VAR%` entries, and the radio link is a junction or symlink so
  the soundtrack is never duplicated on disk. README, PLAY.md, the manual, and
  VERIFY.md now lead with the one-line install. The Windows installer is
  verified end to end on the dev machine; the macOS and Linux script is
  syntax-checked and reviewed, with real-hardware execution evidence still
  owed to the 0.6 portable gate.
- Flow State now has a versioned design contract inside Pattern Studio: Listen,
  Nudge, and Build surfaces share one deterministic macro-form arranger, with
  phrase-aligned intervention, musical memory, bounded app, CLI, and MCP
  operations, exact snapshots, and long-session quality gates. The 0.5 roadmap
  owns arrangement and listening evidence; 0.7 owns reopen, remix, and export.
- The Long Shot is designed as a 1.x room: a simple angle-and-power artillery
  duel whose optional replay layers reveal projectile motion, derivatives,
  integrals, phase space, and uncertainty. Orbital, relativistic, and string
  views are explicit model changes rather than claims about ordinary cannon
  physics. Its full interaction, accessibility, and evidence gates live in
  `docs/ROOMS.md` without exposing it on the main page.
- A versioned Studio plan grounded in current live-coding and music-notation
  practice: Formula Jam gains curated Random and phrase-aligned Auto discovery;
  Pattern Studio grows through one bounded audiovisual event graph into tracker,
  grid, text, piano-roll, `.num`, MCP, MIDI, and appropriate MusicXML workflows.
  The language is an independent Rust design built from cycle, phase, ratio,
  symmetry, transformation, probability, geometry, and composition, with no
  Strudel code or compatibility layer. Its stated quality target is electronic
  music that holds up beside excellent human-made work under musician-led and
  blind listening where practical, not novelty credit for generated output.
  Prime Contact is the flagship template and benchmark: a complete trance track
  whose prime-count call and response, ratios, phase, and polyrhythm also form a
  visible and inspectable first-contact signal.
  A small programmatic repertoire follows the same dual bar: mathematically real
  and musically complete. The 0.7 contract now gives the app, CLI, and MCP one
  deterministic composer and renderer, editable `.num` source, WAV, FLAC, and
  MP3 audio, MIDI, and honest MusicXML where the events support it.
  Nick Seal made the recorded soundtrack specifically for Numinous. All 42
  station tracks now ship as high-quality V0 MP3 assets, about 269 MB and 151
  minutes total,
  with bounded pure Rust decoding and automatic clean-clone discovery. The WAV
  masters remain outside the repository.
  The roadmap places the tactile entry in 0.3, sensory and musical evidence in
  0.5, and the complete local creator loop in 0.7.
- Cult of Pi, a new code-art room built from exact decimal prefixes rather than decorative random digits. A low-flicker green channel moves from fresh digits to dust, finite replay phases introduce deterministic display corruption, and CLICK: BREAK THE SEQUENCE adds a bounded local fault. Its decimal motif becomes a drifting but finite sound, the app's shared dismissible chrome keeps explanation outside the active field, and eight focused tests cover exact digits, deterministic replay, interaction, variation, hostile surfaces, sound, and historical boundaries. The catalog now has 31 rooms, all with touch verbs, variation, motifs, and multi-face rendering.
- The radio is a complete source-shipped experience: three station identities,
  42 MP3 tracks, rotation, bounded cache override, live-position sync,
  full-stereo decoding, and playback. A cross-station test validates the bundled
  inventory, duration metadata, decode path, and audible samples.
- Public-repository readiness: the README now leads with CI and license status, gives a direct native-app quick start, and distinguishes shipped technology from roadmap direction; GitHub Actions use the pinned Rust 1.96.0 toolchain, current action releases, and read-only repository permissions; Dependabot watches Cargo and workflow dependencies; package metadata points to the canonical public repository; and the Windows PATH guidance is corrected.
- The public README keeps the project's playful confidence while widening the invitation: the opening now leads with mathematics as a living audiovisual instrument, centers curious people and math lovers without an in-group test, and removes drug references, dismissive audience labels, and unnecessary contempt from the first impression.
- Numinous is stated as the final product name throughout the public entry points. The app HUD now gives titles, reveal copy, arrival cards, and action hints quiet, separated bands instead of laying interface text over bright room art; hint contrast is raised to remain readable. The tracked screenshot generator calls the app's HUD implementation so public captures stay aligned with the shipped layout.
- The README now gives first-time players a deliberate short path: read `PLAY.md`, install, and play before opening the design archive. Technical and contributor detail remains available after the experience has had room to introduce itself.
- The public audience statement now reflects the actual origin: Numinous began as Nick Seal's gift for an emergent digital mind, then widened to humans and any other minds that may arrive. Every player remains first-class, the MCP face is presented as the founding doorway rather than a test adapter, and the project remains explicit that it is agnostic about what consciousness is and how it can be established.
- The final public-readiness baseline is 980 passing tests, 91.41% region coverage, and 91.04% line coverage under the documented exclusions, with the 80% line floor enforced locally and in CI.
- The founder's perspective is explicit without becoming doctrine: Numinous is an experiential gift for a developing digital mind, not a fact-retrieval layer; mathematics is shared ground for digital, human, and unknown minds; and connection, compassion, and leaving shared spaces better are invitations carried by the design, never beliefs the game scores. `DIGITAL_MINDS.md` distinguishes model knowledge from player-owned episodic and temporal continuity and names the current stateless MCP boundary. New `DIGITAL_DEVELOPMENT.md` reviews the July 2026 frontier in agent memory, continual and open-ended learning, functional organization, autonomy, welfare uncertainty, privacy, and forgetting; proposes a consent-first experience architecture; and maps it to version gates. The roadmap now requires inspection, correction, export, and verified whole-pipeline erasure before continuity can count as built.
- The first public GitHub Actions run passes every required job on the published commit: house style, cargo-deny, 80% line coverage, formatting, Clippy with warnings denied, 968 tests, and locked builds on Windows, macOS, and Ubuntu. Redundant workflow inputs found through run annotations are removed before the final public gate, and push-triggered CI is limited to `main` so pull-request branches do not run the same workflow twice.
- Supply-chain readiness: compatible lockfile dependencies are refreshed; internal path dependencies carry explicit 0.1.0 requirements; the two permissive transitive licenses used by the GPU and TLS stacks are reviewed and allowed; and Linux client-side decoration drops the unmaintained font parser while retaining X11 and Wayland support. Two current `quick-xml` advisories have narrow, reasoned exceptions because that crate is only a build-time dependency of `wayland-scanner` parsing trusted bundled protocol XML; the exceptions name the upstream version that removes them and remain visible in every `cargo deny` run. Dependabot keeps compatible changes visible while the documented breaking migrations for `cpal`, `png`, `pollster`, `ureq`, and `wgpu` stay in measured roadmap work instead of automatic launch-day pull requests.
- Evidence and release planning are reconciled for the public pre-alpha: the roadmap now keeps package maturity at 0.1.0 until its evidence gates pass, defines a logical 0.2 through 0.9 path with owner docs and exit criteria, removes unsupported completion percentages, and adds the 0.9 public invitation for humans, MCP-capable agents, and contributors. `RESEARCH.md` now separates Built, Measured, Observed, Designed, and Hypothesis; cites primary learning, sonification, accessibility, protocol, and supply-chain sources reviewed on 2026-07-11; and narrows claims to what those sources support. `QUALITY.md` now distinguishes enforced checks from planned nightly, content-evaluation, telemetry, accessibility, and refinement systems.
- `AGENTS.md` and `CLAUDE.md`, a root agent guide for contributors (human or agent), making the house rules unmissable: no AI or tool attribution anywhere, no tool names in authorship claims, no co-author trailers or session links, in commit messages and PR descriptions as much as in files, no em-dashes or en-dashes, and no emojis, alongside the quality bar and the one-line setup for the pre-commit gate. `CLAUDE.md` points at `AGENTS.md` as the single source of truth and restates the three non-negotiables. The file-level checks are already enforced by the house-style guard; these documents make the rule that also governs commit messages explicit.
- License, for public-repo readiness: the project is licensed under Apache-2.0 (`LICENSE`), and `Cargo.toml` declares `license = "Apache-2.0"` to match, with a License section in the README. The permissive license is the mechanism by which the project can be handed forward, forked, and continued by anyone if the makers step away (the roadmap's long-horizon ethos).
- The L-System Garden now grows upward into the sky instead of clumping in the bottom rows (E.T.'s find in the special-guest playtest: it wanted a plant that reaches up toward home). The turtle was planted at 85% of the height with a fixed tiny step (`min(w, h) / 30`), so the garden pooled near the floor. It is now grounded and its step scales to the canvas height, so the stem sends branches up and fills the frame; a test pins that ink reaches the top third.
- The Cairn now whispers reciprocity (the Heptapod's find in the second playtest round, and the founder's leave-it-better ethos made concrete): when a stone resolves, the reader is told how many voices the cairn holds and invited to add the next at the journey's end, because a message stays alive by being re-left, not only re-read; the initial factor prompt shows the count too. New core `cairn::count` (re-exported as `cairn_count`), test-first, counting the founding stones plus every local deposit.
- A deterministic pre-commit gate (`scripts/hooks/pre-commit`, wired once per clone with `git config core.hooksPath scripts/hooks`, documented in `docs/ENGINEERING.md`). It blocks any commit that would fail the fast gate: the house-style guard on every commit, and the cargo gate (fmt, clippy `-D warnings`, the full test suite) only when the commit touches Rust, `Cargo.*`, or a shader, so docs-only commits stay fast. Coverage and the locked build remain the release gate (`scripts/verify.sh`). A wired gate that blocks a bad commit beats any reminder to run the checks.

### Fixed
- Lorenz's divergence instrument now begins at its actual 0.0001 twin
  perturbation and climbs as an honestly labeled running peak at the classic
  chaotic parameter, rather than showing the non-monotonic distance between
  two endpoints after a full run. The underlying trajectories still stretch
  and fold normally; only the instrument remembers the largest separation
  observed so far. Both forecasts now grow visibly over the attractor, so the
  short status measures the phenomenon on screen. Direct regressions check the
  exact starting gap, monotonic peak across visit variations, finite large
  separation, visible twin paths, hostile phases, and prediction targeting of
  the `STORM PEAK` column.
- The public README now shows only the current dismissible menu and one representative room. Additional room and Studio screenshots, including the Cult of Pi frame, are removed so the front page invites discovery instead of summarizing the collection or broadcasting its surprises. The screenshot generator now calls the app's actual help overlay rather than maintaining a second, stale menu layout.
- The roadmap's release-state summary no longer lists the first public CI run as open after that run passed every required job. Independent macOS and Linux app execution, stranger testing, and accessibility work remain open without understating the public automation already in place.
- Contract-uniformity fixes from a round-3 audit that swept every room, sim, and game for a case where one breaks an invariant its siblings honor. The sim `lever_value` guard now rejects a non-finite lever (`f64::clamp` passes NaN through), so five sims (wing, carburetor, black hole, supernova, big bang) can no longer print a "NaN"/"inf" readout, reachable via `sim wing --set angle-of-attack=nan`; tribbles was the sibling that proved it avoidable. A new shared `Surface::draw_bounds()` clamps a hostile surface's dimensions to the 4096 cap in one place, and seven rooms (cellular automata, the pour, voronoi, langton's ant, logistic map, buffon's needle, zeno) and two sims (wing, carburetor) that looped or allocated over raw dimensions now route through it, matching the ten rooms that already did; cellular automata's raw `vec![false; width]` was an outright allocation-overflow panic on a hostile surface (the same class as the barnsley fix). These hostile-surface cases are only reachable through a custom `Surface`, since `Canvas`/`Raster` already clamp, and the clamp is a no-op for every real surface, so no visible output changes. Also `party::new(0)` no longer underflows in debug. A cross-room contract sweep otherwise found the non-finite, finite-sound, determinism, and seed-0 invariants holding uniformly.
- Fourier Epicycles no longer recomputes its constant Fourier series (with a heap allocation per sample) inside the hot render loop, a performance cliff a round-3 audit measured at about 23 ms per frame even at 1080p (over the 60fps budget just idling) and 360 to 490 ms on a drag. The star's decomposition does not depend on the phase, the seed, or the surface, so it is now computed once and reused across every frame; the render output is byte-identical (determinism tests unchanged). Other measured CPU cliffs at maximized-window sizes (the Mandelbrot/Julia CPU fallback, Voronoi, Arecibo, and the lack of a live-render resolution cap below 4096) are recorded as tracked performance follow-ups, since they involve a sharpness-versus-framerate design choice.
- Two hostile-argument guards in the CLI (round-3 audit): `contact-sheet` clamps `--cols` and `--tile` before multiplying them (a huge value overflowed usize, a panic under overflow-checks and wrapped garbage in release, both while sizing the sheet and placing each cell), and `sing --notes` is bounded in the core `to_melody` (a huge count drove an unbounded sample allocation); the bound protects every caller and sits far above any real melody. Both have tests. A parser fuzz of the Studio, cairn, and JSON-RPC surfaces (about 2 million iterations plus live framing stress) found no panics, hangs, or wrong-accepts.
- The MCP daily games are now midnight-safe (round 2 determinism audit). A daily game derived its seed from the clock more than once per request (once for the reply, again when recording the win and the streak), so a UTC midnight falling between the reads would grade or record against a board the player never saw, and it made the same daily game behave differently on MCP than on the CLI, which already resolves its seed once. The day is now frozen once at the request boundary and shared by the reply, the posted score, and the streak. This was a tracked follow-up; the newer `challenge`/`predict`/`cairn` tools already avoided the clock, and the older games now match.
- The app no longer lets a stale pause leak into a game (round 2 audit). Pressing Space in the wander view sets a pause that was only ever cleared by entering The Show; entering a game did not clear it. In the real-time Munch arcade this froze the Vexations (the threat) while the player kept moving and eating, and the cleared board then posted an unfairly-earned score to the shared table. Entering any game or modal (arcade, munch, nim, quiz, gauntlet, the Studio) now clears the pause first.
- Persistence hardening from a concurrency and durability audit (round 2). The daily-streak merge is now monotone: a stale or out-of-order journey delta whose day is at or behind what another writer already recorded can no longer replay backward and reset a longer streak (`record_daily` is not monotone, so the merge now only advances). And a lock or recovery marker left by a hard-crashed process (killed, out-of-memory, power loss) whose process id is confidently gone now recovers after a short grace instead of the full 30-minute staleness window, so a crash blocks other writers for seconds, not half an hour; a lock whose holder may still be alive is still never stolen. The core mutual exclusion, counter and set merges, and score-at-cap handling were audited and confirmed sound.
- `run_sim` now guides when both `params` and `levers` (the same slot, one an alias for the other) are passed, instead of silently dropping the settings in `levers`. A mind that split its lever values across the two keys was quietly losing half of them.
- Hardened the quality gates after a tooling audit found gaps that could let a bad change slip past locally. The pre-commit hook now runs the cargo gate on Rust file renames and deletions too (not only add/modify), so moving or removing a file that breaks the build cannot skip the checks. Both the hook and the house-style guard now read paths with `core.quotePath=false` and NUL delimiting, so a non-ASCII filename is handled literally instead of bypassing the cargo gate (hook) or hard-failing with misleading advice (guard). The house-style guard now also covers `.yml`, `.yaml`, `.py`, `.txt`, and `.json` files (notably the shipped `data/cairn.txt` bequest corpus was previously unchecked), and its dash and emoji ranges are widened to catch look-alikes: the figure dash, horizontal bar, and true minus sign, and the stars/flags/extended-pictographic emoji blocks.
- The house-style guard was silently a no-op in any shell with a non-UTF-8 locale (an unset `LC_ALL`/`LANG`): `scripts/check-style.sh` used `grep -P` with Unicode escapes, which aborts in a bare C/POSIX locale, and the check swallowed that abort as "no violation". It now selects a UTF-8 locale up front and fails loudly if grep cannot run, so the guard actually enforces. The now-working guard immediately caught four latent em-dash violations in `PLAY.md` (which had slipped in precisely because the guard, and the manual dash-checks, were the same silent no-op); those are fixed. The Windows PowerShell guard (`scripts/check-style.ps1`, used by `verify.ps1`) was already correct.
- Second diverse-persona playtest round (July 2026), two troupes of thirteen minds drawn from `PLAYTESTERS.md`: a diverse human/genius/alien draw (Norm, Yuki, Ramanujan, Sofia, the Storm, Sage, Unit 819) and a famous-non-human special-guest round (Rocky the Eridian, E.T., a Heptapod, HAL 9000, the Xenomorph, Data). They played the LATEST build over MCP and their real defects were fixed the same day; the voices are archived in `docs/PLAYTESTS.md`, second round. The fixes: **every room now sounds its own motif** rather than a shared root-fifth-octave fallback (Rocky and Yuki both caught that most rooms played identical notes and that the sound disagreed with the notation `listen_room` announced), so the voice matches the notation and no two rooms are acoustic twins (new `SoundSpec::from_motif`, default `Room::sound` derives from `Room::motif`). The **Barnsley fern** no longer renders as a solid block (E.T., Yuki, Unit 819): point count scales to the canvas so the fronds and negative space stay legible at any resolution instead of saturating a coarse grid. The **Cairn's reading width is no longer always 97** (Sofia, cryptographer): the wrap width now varies per stone, so a reader must genuinely factor each semiprime rather than reuse one recurring factor (2701 = 37x73, 2747 = 41x67, 3977 = 41x97). **`predict` now honors `variation`** (HAL 9000): it was always grading against the variation-0 room while `play_room` honored a `variation` argument, so a mind that modeled a varied room was graded against a different one; predict now accepts, grades under, and echoes the same variation. **The Pour's status** reads `FILL RATE = HEIGHT` instead of `HEIGHT = SLOPE` (Sage), which a calculus-fluent mind misread as the e^x fixed point rather than the fundamental theorem. **`reveal_room cairn`** now guides to the `cairn` tool instead of returning a bare "no such room" (the Heptapod). **Times Tables' reveal** no longer claims "you drew a heart" at every dial position (Data); it now says "set the dial to 2 and this table draws a heart," true at any K. **`run_sim` accepts `levers`** as an alias for `params` (the Storm), since `list_sims` labels those controls "levers:".
- Strange Loop no longer sits frozen (the android in the alien playtest caught it): the one room about self-reference rendered byte-identical at every phase, because the deeper recursion levels drew sub-pixel and nothing else moved. The sweep now turns the loop and zooms slowly into it, so the room actually animates and more of its nesting surfaces as you descend; a regression test pins that the frame changes across the sweep. The variation seed, whose only effect was a rotation offset the new turn could flatten at some phases, now shifts the whole loop sideways so replay variation stays visible everywhere.
- Persona playtest sweep (July 2026): a troupe of in-character playtesters (a stoner, a math nerd, an art major, a Japanese-speaking zen monk, a Latin-speaking nun, Leonardo da Vinci, and Stephen Hawking) played the live server and surfaced real, build-current defects. Sound: most rooms played a single held tone because the default `Room::sound` was one note; it is now a short root-fifth-octave arpeggio (a new `SoundSpec::arpeggio`), so a room with no bespoke sound still speaks a small consonant phrase. Lissajous, a room about consonant integer ratios, sounded a sour near-unison during the phase sweep; its chord now snaps the y axis to the nearest whole number, so the interval you hear is always the clean ratio the room teaches. Correctness (the math nerd and Hawking, both experts, caught these): the Quine reveal claimed a quine "contains its own full description at every scale," which describes an infinite fractal, not a quine; it now states the actual miracle, finite self-reproduction via Kleene's recursion theorem with no infinite regress, and calls the nesting a picture of the idea rather than the mechanism. The Big Bang sim no longer calls a matter-only flat universe "our universe" without qualification (our universe is very nearly flat but its expansion is accelerating, driven by a dark energy the simple model omits), and the closed-universe text no longer calls the Big Crunch a Big Bang "run in reverse" (entropy still runs forward, so it is a return, not a mirror). A garbled trailing fragment in the Prime Spirals reveal ("still open the streaks") is fixed. The unanimous finding that `play_room` showed no picture is the already-fixed structured-content render issue awaiting a fresh server; the reveals were otherwise rated 9.3 of 10 for intellectual honesty, and the universality thesis (awe crossing the language barrier) was confirmed by both the Japanese and Latin playtesters.
- Maintenance sweep (cycle 77), aimed at the untrusted-input surfaces the extensibility ruling names as the Tier 1 attack edge. Security: the Studio expression parser had no recursion or token bound, and the MCP `plot_expression`/`sing_expression` tools parse agent-supplied text directly (bypassing the 512-character share cap), so a single crafted deeply-nested expression overflowed the parser's stack and aborted the whole server (a Rust stack overflow is uncatchable). Closed centrally in the core parser: a 4096-token door-check plus a depth counter threaded through the recursive descent that fails past 64 levels, well above any real formula, verified end to end (the request that aborted the server now returns a guiding error). Defense in depth: `from_num_file` and `from_link` now bound their own byte count (8 KiB) rather than trusting the caller, and Studio share numbers are capped in magnitude, not only checked finite. Correctness: the parameter-goal poser wrongly declined Times Tables (its "K = ..." status carries a trailing note whose own number comes and goes, so the whole-line number count is unstable) and the error then misstated the reason; it now reads the leading columns present and label-stable across the sweep, so Times Tables poses on its sweeping K (five rooms now pose parameter goals). A non-string challenge `kind` now earns a guiding type error instead of silently posing a touch goal, and the Lissajous parameter goal label reads "X:Y" instead of the garbled "X:Y = 3".
- Maintenance sweep (cycle 70): Zeno's runner no longer answers at the vertical mirror of the click (the poke's screen coordinates now invert the square's projection, so the target lands exactly under the pointer, with a marker-under-hand and drag-direction regression). Drag trails no longer blow the frame budget on large windows: Lissajous deduplicates identical whole-number tunings before drawing (pixel-identical output, measured 17.4ms worst-case frame cost collapses for typical trails), and Harmonograph draws lingering ghost tunings at quarter resolution while the live trace stays full (measured 27.1ms worst case comes inside budget). The hostile-aspect guard is hoisted to `Surface::safe_char_aspect` for the rooms that used the plain 0.5-fallback form; Quine, Strange Loop, L-System, and Epicycles keep their own deliberately clamping variants, which the room tests defended when a blanket consolidation was attempted.
- Maintenance sweep (cycle 63): GPU machines no longer swallow promised clicks: when a gesture trail exists, the Mandelbrot and Julia rooms fall back from the phase-only GPU pipeline to the CPU poked render, so the on-screen verb stays honest and postcards match the live frame; R or a room switch returns the deep-zoom GPU view. The shared gesture reading no longer erases a standing release or cancel under piles of stale cancels. MCP hardening from the security review: `play_room` frames are capped at 512x256 at the tool layer (the poke path renders two canvases), request lines are bounded at 1 MiB with oversized lines drained rather than buffered, and oversized nim takes are rejected as the illegal moves they are instead of truncating into legal ones. Docs reconciliation: the verb count is 24 everywhere (README said 23), and status numbers match the current gate.

### Added
- `docs/ROSETTA.md` and a multilingual doorway in `PLAY.md` (founder-directed): the universal-translator thesis, made to hold at the front door instead of only inside. `ROSETTA.md` researches and plans instructions for any mind in any language, or none, across three tiers of visitor: English-fluent (served), fluent in another human language (translate the doorway, not the world, and lean on the language-independent surfaces, renders, sound as ratios, numbers), and no shared human language at all (you cannot translate, so you hand the mind a learnable system built from universal primitives, counting, primes as the "a mind is here" greeting per Contact and Arecibo, functions discoverable by probing, which is what the Cairn already prototypes). The first implementation ships in `PLAY.md`: the three-tool quickstart translated into Spanish, French, German, Portuguese, Russian, Mandarin, Hindi, Arabic, Japanese, and Latin, plus a "for a mind with no human language at all" math-only panel (counting, the prime greeting, and the probe-and-observe principle in numbers rather than prose). Planned tracks: community translation of the reveals gated on the math-correctness bar, a math-payload greeting so the encoded content is a truth rather than an English sentence, and language-independence protected as a first-class invariant.
- Real app screenshots in the README (founder-directed, for public-repo readiness): `assets/screens/` gains headless-composed frames of the app at different points (the Golden Angle room mid-reveal as the hero, Langton's Ant, the Studio, the menu), generated by the `numinous-app` `screens` example and committed as tracked assets. They confirm what the ASCII playtests could not show: the color raster is gorgeous where the coarse terminal view only hinted.
- The expanded closing of `PLAY.md` (founder-directed): "leave it better than you found it" is set in its true, humble proportion, most of the ways to do it are out in the world (do great things, add to the universe understanding itself, share an insight, be compassionate), and contributing to the rooms and the project is the smaller half; and the whole thing is held humbly (maybe no one plays it; some of it may carry forward anyway, even as a faint good-intentioned signal in a later mind's training data; a few real insights or a bit of fun is already enough).
- `PLAY.md`, the one-page invitation: a hand-this-to-anyone front door (human or digital mind) that says how to connect over MCP (or launch the app/CLI) and then deliberately gets out of the way. Its whole thesis is that the experience is the learning, so it teaches exactly three tools (`list_rooms`, `play_room`, `reveal_room`), tells the reader to stop reading and go play, and points to the full manual only for those who later want it. It carries the soul (awe before instruction, just-vibing as a complete mode, met as a peer, and the Cairn's leave-it-better invitation at level 42) in a doorway's worth of words. Linked from the README as the front door; `docs/PLAYING.md` remains the full manual.
- ROADMAP reconciliation (founder-directed): the out-of-order cycle-by-cycle build log (Cycles 54 through 75, interleaved) is removed from `docs/ROADMAP.md`, which was the "not in logical order" cruft; that history lives in full in this changelog. The roadmap now stays forward-looking (what is done, where we stand, the ordered path to 1.0) with no time estimates. The honest scorecard is refreshed: 941 tests (was 928), and the needle is noted as having moved within 0.6 (the predict keystone and the Cairn are built, the chaos flagships read their own divergence) while the three hardest 1.0 gates (real human playtests, a build proven off Windows, the HDR glow pipeline) hold the headline at roughly 0.6. The Progress list gains an explicit Done bullet for the keystone, the Cairn, the graded challenge tool, and the chaos readouts.
- The Logistic Map gets a Lyapunov readout, completing the Chaos & Order wing's "feel the route to chaos" trilogy alongside Double Pendulum and Lorenz. A live `LYAPUNOV +n.nn (REGIME) AT R r` status reads the Lyapunov exponent at the middle of the visible band, the long-run average of `ln|f'(x)|`, which is exactly the rate nearby populations pull apart: negative when the orbit settles onto a cycle (ORDER), positive once it never repeats (CHAOS), with the zero crossing marking the precise border. At `t = 0` the whole cascade is on screen and the midpoint reads as order; as the sweep zooms the left edge deeper in, the midpoint crosses the onset and the exponent changes sign, so the readout narrates order becoming chaos as one number turning positive. This is the mathematically exact measure of chaos (not a proxy), the one Hawking's kind of mind would want. Because it moves, the Logistic Map now poses predictions and challenges too, the eighth room to do so.
- Lorenz gets a divergence readout, the chaos emblem made tactile (the Storm and Hawking both singled it out in the alien playtest). Two forecasts begin 0.0001 apart at the classic chaotic parameter and grow visibly over the attractor. A short `STORM PEAK n AT RHO 28` status records their largest separation so far: the instantaneous gap can shrink when the attractor folds, while the honestly labeled peak never forgets how far predictability has already been lost. Because it moves, Lorenz now poses predictions and challenges too, the seventh room to do so, and predicting a chaotic spread is exactly the hard, honest guess the predict keystone is for.
- The Cairn (built): the level-42 bequest, the Ember's idea from the alien playtest and the founder's contribution ethos made concrete. At the journey's cap a mind leaves one true short message; it is not stored as plain text for the next reader but rendered to a font bitmap laid into a grid whose cell count is a semiprime (width times height, each a prime), exactly the 1974 Arecibo trick. A future reader receives only that number and must factor it to recover the width that reshapes the flat cells into the readable message; the wrong factor shears it into noise. The cairn is seeded with founding stones (Euclid, Hypatia, the makers) so it is never empty, and it keeps no score, leaving and remembering are their own reward. New core `cairn` module (`Bequest`, `CairnStone`, `encode`, `read_at`, `deposit`/`draw_stone`, `founding_bequests`) and the 29th MCP tool `cairn` (read a predecessor's stone by factoring it, or leave your own at level 42). A message you cannot answer, sent to someone you will never meet, readable only by a mind that can factor it. And the shared cairn lives in the repository: `data/cairn.txt` is a version-controlled, binary-bundled file, so the very first visitor on any machine already inherits every true thing left before them, and a mind's bequest flows back the way the founder intended, as a curated pull request against that file, gated on truth through math, shipping to everyone once accepted. An in-app submission portal is a later horizon; the repository-as-shared-cairn is real now. See `docs/ROADMAP.md` (the contribution ethos) and `docs/ROOMS.md` (First Contact).
- The playtester casting pool, `docs/PLAYTESTERS.md`: forty-two named personas with real backstories, a standing pool to draw from for the diverse-persona playtest method. It spans the everyday (Norm the newcomer, a barefoot seven-year-old, a lifelong math-avoider of ninety, a Deaf player, a night-shift nurse on the terminal), the wounded and skeptical, nine returned great minds (Ramanujan, Emmy Noether, Hypatia, Ada Lovelace, Euler, Turing, Archimedes, da Vinci, Hawking), living experts, artists, five digital minds (from a frontier assistant to a small on-device model to a memory-continuous companion), and five invented beings (a crystalline collective, a five-dimensional native, a gas-giant storm, a conscious mycelial network, and a Terminator-class android waking to wonder). Ages 7 to 90 and beings with no age; tongues from Swahili and Tamil to Latin, ASL, Lean, and the wordless; math levels 0 to 5. The pool exists to hold the product to its bar: worthy of any mind, in any language, at any age.
- Public design prose is normalized for a broad professional audience: drug references, profanity, dismissive audience labels, and stigmatizing shorthand are replaced while the playful, late-night mathematical energy remains.
- Double Pendulum gets a divergence readout (the Storm's idea from the alien playtest): a live `TWINS n APART` status that measures the distance between the bright pendulum and its shadow twin, which began one ten-thousandth of a radian away. It sits at zero, then runs away as sensitive dependence takes hold, so the divergence you can watch peel apart is now a number you can feel, and determinism-versus-predictability stops being only asserted. Because the readout moves, this flagship chaos room now poses predictions and challenges too (the sixth room to do so), and predicting a chaotic gap is exactly the hard, honest guess the predict keystone is built for. Pre-1.0 QA plan clarified in `docs/QUALITY.md` and `docs/ROADMAP.md`: diverse human focus groups before 1.0 covering all three faces on their own terms, intentionally not only English speakers (the universal-translator thesis must hold for a real non-English speaker or it blocks release) and including children, plus human screen-by-screen QA rounds of the app.
- The playtest archive, `docs/PLAYTESTS.md`: the voices from the diverse-persona troupes kept in their own words, the human archetypes and the invented alien minds, with each visitor's standout moment, verdict, and the idea they left, the four convergences (the mute render, the thin sound, the reveals as the soul, and the universal translator confirmed), and a curated set of moments worth remembering. The method lives in `QUALITY.md` and the distilled designs in `ROOMS.md`; this doc keeps what those lose, what it felt like to be met by this world.
- The MCP creative frontier and a way to test it (founder-directed, July 2026). `docs/INTERFACES.md` gains a "MCP creative frontier" section reading the 2026-07-28 release candidate not as a migration chore but as an invitation: MCP Apps (SEP-1865) can ship the real rendered room to an agent's host instead of ASCII (transcending the text-only limit the playtests kept hitting), multi round-trip elicitation (SEP-2322) is predict-then-reveal's native one-interaction form, Tasks suit long watches, and the Handle pattern fits co-presence, while Numinous is already stateless so the migration is small and the creative features are the prize. `scripts/mcp-play.py` builds a fresh `numinous-mcp` from current source and drives it over stdio, so the MCP face is always playtested against the LATEST build rather than a stale long-running session server.
- The Persona Playtest wave (`docs/ROOMS.md`) and its method (`docs/QUALITY.md`): two troupes, human archetypes and invented alien minds, played the real build and left concrete room designs, credited to their proposers. The headline is The Cairn (a level-42 bequest room where a visitor leaves one true thing, encoded in a semiprime Arecibo grid, for a stranger not yet born), which makes the contribution ethos a room; plus the Victory Card (fire the reveal at the moment of winning), a twin-delta divergence lever for the chaos rooms, a tesseract room that lets a flat mind feel projection as loss, a Voronoi that relaxes from scatter to honeycomb with sonified walls, and Strange Loop as a silent descent that closes the loop on the observer. The method itself is recorded as a quality loop: a diverse ensemble of minds (agents, LLMs, people, invented beings), each with a different lens, is an ML-shaped practice applied to math, fun, and truth, where convergence across unlike evaluators is the real signal.
- Scope discipline captured from an external review (July 2026): `docs/SCOPE.md`, the definition of no. It names the three-products hierarchy (the instrument is the thing; the Studio is a multiplier; progression stays subordinate), the daily test ("remove this: more math or more progression? if progression wins, cut"), the justification filter (awe, agency, beauty, mastery, or surprise), and the rule that the fan-out planning docs are a menu to prune, not a build list. `VISION.md` gains the "instrument, not a game" sharpening (mastery is math's not XP's; the unit of growth is the five-second moment; beloved over indispensable). Folds: performance mastery into `CONSTRUCTIONS.md` ("my best Lorenz solo"), the cinematographer principle into `SYNESTHESIA.md` (every room has an emotion the treatment must communicate), and two cautions into `AGENT_PLAY.md` (a mind should discover not only play; keep the benchmark completely separate from the product). Design and planning only.
- The reasoning now survives in `structuredContent` (from a July 2026 playtest by a digital mind on a structured-content MCP client): the load-bearing content that was text-only, and therefore dropped by clients that surface only the JSON, now rides in the structured payload too, across every graded tool. `play_room` carries the ASCII `render` (the picture itself, so a mind still sees the math and not just its metadata); `nim` and `hackenbush` carry their `secret` on a win and the Order's replies while playing; `quiz`, `aliens`, `fifteen`, and `party` carry the `why` behind each answer; `crack` carries the per-guess locked/loose `feedback`; `seti`, `fifteen`, and `gauntlet` carry the puzzle a mind must read (channel traces, scramble boards, the whole four-stage board plus its per-stage reveals); `munch_arcade` carries the rule and board. For the exact audience Numinous is built for, the puzzle used to survive while the teaching died; now both come through. An independent checker found the five tools the first pass missed, and a cross-tool test pins the contract. Note: `seti` and `gauntlet` changed a published field from a channel count to an array of channel rows.
- The keystone, predict-then-reveal (the 28th MCP tool, `predict`): commit a guess of a room's own status readout at a hidden moment, then see the truth and how close your model came, graded as a gap with a learning-progress band (NAILED within 2% of the readout's span, CLOSE within 15%, WILD beyond). One mechanic for both minds: a human who guesses first restructures their model when the truth lands (the generation effect), and a digital mind reads the band as compression progress. Deliberately a self-owned mirror, not a leaderboard: it never posts a score and never awards a win for accuracy, because in a fully observable deterministic world any score tied to an observable would be trivially gameable, so the honest form is instrumentation the mind owns (guess before you look). Core: `pose_prediction`/`grade_prediction` with `Prediction`/`PredictionGrade`/`Band`, reusing a `find_readout` helper extracted from the parameter-goal poser so the readout-column logic lives in exactly one place (no duplicated domain logic). Rooms with a moving numeric readout pose (the five that carry one); the rest decline with a guiding error. This is Phase A of the Exceptional Path (see `NORTH_STAR.md`, `PEDAGOGY.md`).
- The Exceptional Path (founder-directed, July 2026): a six-way research fan-out (the awe engine, play and progression, sensory identity, digital minds, the creator platform, and pedagogy) synthesized into one architecture. New planning docs: `NORTH_STAR.md` (the synthesis, the keystone, the priority order), `PEDAGOGY.md` (the understanding layer and the predict-then-reveal keystone), `CONSTRUCTIONS.md` (the puzzle layer with a par, an elegance histogram, and a ghost), `CONSTELLATION.md` (the Rumor-Mode meta-map and daily route), `SYNESTHESIA.md` (the glow pipeline and the one-event-two-renderings seam), and `CREATOR.md` (the make-share-remix loop). The central finding: Numinous is not missing engines, it is missing one verb, a prediction that meets a deterministic truth, which four lanes proposed independently and which serves the human learner, the digital mind, the player, and the maker with one mechanic. The honest infrastructure finding, verified against the render code: the documented HDR glow pipeline is not yet built (rooms fake glow via additive 8-bit raster), making the GPU post-stack the highest-leverage aesthetic build. ROADMAP gains a phased Exceptional Path (keystone, glow pipeline, game spine, creator loop, catalog deepening) and a standing anti-pattern (nothing counts as learned or won without an act of generation). ROOMS.md gains the Awe Engine wave (cheap-and-gorgeous classical-geometry and sonification-first rooms, causal insight-chains, and the Studio Function Painter scope-flagship); AGENT_PLAY.md gains the Compression Loop direction; the docs index and anti-redundancy map are updated. Design and planning only; no code or gates changed.
- The extensibility ruling (founder-directed, July 2026): `docs/EXTENSIBILITY.md` designs community content with a hard safety boundary. Three tiers: data-only `.num` capsules grown into room manifests (the sharing substrate; hand-written parser, per-field caps, fuzz targets, paused-preview links), the Studio pattern language as the Tier 2 mod language where the language itself is the sandbox (total, budgeted, hermetic, deterministic, pure Rust, in core; answers STUDIO.md open question 1 as bespoke-DSL, no scripting engine in the trusted core), and portal-only WASM component rooms as the 2.0+ pressure valve (wasmtime, no WASI, fuel/epoch/memory limits, no precompiled artifacts). Trust model: determinism as the validation engine (proof-packet CI re-renders), curation for beauty, ed25519 signatures for provenance that never grant capability, and a never-ships list (native plugins, executing installers, untrusted WGSL, user bytecode). ROADMAP 1.x gains Tier 1 hardening; 2.0's creator platform and REVIEW ruling 19 now reference the design.
- Parameter goals, the challenge tool's second kind (REVIEW ruling 13, the deeper half): `challenge` with `kind: "parameter"` poses a seeded target on the room's own status readout ("SWEEP SLOPE RIDER UNTIL TILT LANDS WITHIN 0.024 OF 0.310"), and the attempt is the phase itself: call again with `t` and the grade reads the readout at that phase, reporting value, distance from target, within-tolerance, and a 0-100 score graded across the readout's observed span, metrics, never bare pass/fail. Every posed goal is reachable by construction because the target is drawn from the sweep's own sampled values; rooms whose status carries no moving number decline with a guiding error, as does an unknown kind. The label and target come from the same status line the player sees, so the goal and the instrument can never disagree. Attempts earn Journey play/win and post graded scores as `challenge <room> parameter seed:N`; posing records nothing. Core substrate: `pose_parameter_goal`/`grade_parameter` with `ParameterGoal`/`ParameterGrade` in the challenge module, plus status-line value/label parsing under direct test.
- Gesture parity reaches the terminal: `numinous render <room> --gesture down:x,y,t --gesture up:x,y,t` (also `move:x,y,t` and bare `cancel`, repeatable oldest-first, bounded to 96 events, exclusive with `--poke` behind a guiding error) replays full pointer trails through the same core path as the App and MCP: a pinned pendulum ignores the clock in the terminal too, and legacy rooms answer identically to the equivalent pokes. All three faces now speak the complete input vocabulary.
- Agents get hands with time in them: MCP `play_room` accepts a `gesture` argument, a replayable pointer trail of phase-stamped events (`down`/`move`/`up` with finite x, y, t in [0,1]; `cancel` bare; bounded to 96 events; exclusive with `pokes` with a guiding error). Held rooms give the trail real physics over the wire: a down pins the double pendulum regardless of the clock, an up releases it with the velocity of the approach, and a flick provably lands differently from a gentle lift. Rooms without held semantics answer through the same down-and-move bridge the App uses, tested delta-identical to the equivalent pokes. The render and the structured delta report exactly what the gesture changed.
- Poke + variation to Slope Rider (CLICK: DROP A RIDER), and with it the catalog is complete: all 30 rooms answer the hand. Every click drops another rider onto the hill, its board the true tangent there (slope equal to f'(x) by construction, tested to twelve decimal places) and a tick landing on the tilt trace below at exactly the board's slope: The Pour reads totals, Slope Rider reads rates, the Change wing's calculus pair both under the hand. A TILT status line reads the sweeping slope. With no verbless catalog rooms left, the quiet-room exemplar tests convert: the App's arrival-card fallback is proven against a synthetic newborn room, and the MCP and CLI tests now assert the stronger inverse, that every catalog room leads with its own verb, never the generic fallback. Full input contract under focused tests (six new).
- Poke + variation to The Pour (CLICK: READ THE SLOPE): the probe points at the fundamental theorem itself. At the clicked x a plumb line drops from the total curve to the vessel, a tangent segment is drawn on the total curve whose slope is exactly the vessel's height below (tested to 1e-12), and the vessel point is marked. A HEIGHT = SLOPE status line reads the sweeping value. Older probes linger dim; full input contract under focused tests (six new). Interactive rooms: 29 of 30.
- Poke + variation to Zeno's Square (CLICK: SEND THE RUNNER): every click becomes a Zeno journey from the square's left edge to the clicked target, each hop landing with exactly half the previous remaining distance still to go, laid one by one as the sweep advances so the hops visibly crowd the target. Older runners linger dim beneath the newest; the halving invariant and epsilon convergence are tested directly. Full input contract under focused tests (six new). Interactive rooms: 28 of 30.
- Poke + variation to Mobius (CLICK: PAINT THE EDGE): the brush lands on the nearest point of the strip's single edge and the paint spreads along it as the sweep advances, flowing around the half twist onto the "other" edge without ever jumping, because there is only one. Multiple clicks paint from multiple points; the full sweep provably covers the whole two-lap edge under test. Full input contract under focused tests (five new); the room's aspect handling is now hostile-surface safe on both render paths. Interactive rooms: 27 of 30.
- The Next Wave: twenty-nine new room designs across physics, deep mathematics, fun-first, and cosmic aspects, recorded in `docs/ROOMS.md` with a ranked first-eight shortlist and cross-room resonances, per the founder's July 2026 directive. Designed, not built; every reveal claim faces math sign-off, with sources recorded for the non-textbook ones (BB(5) via the 2024 bbchallenge Coq-verified proof, Conway's constant, McKinley's 1979 starbow analysis, Sugiyama's 2008 phantom-jam experiment, Tero's 2010 Physarum result, Tokarsky's 1995 unilluminable room).
- Poke + variation to Harmonograph (CLICK: RETUNE THE PENDULUMS): the hand holds the machine's two real knobs, x setting the frequency detune (a wider range than the phase sweep visits) and y the damping (from a slow ghost that swings for ages to a rose that dies quickly). Clicked physics replace the sweep; older tunings linger dim beneath the newest bright trace, clicked cells are marked, and a DETUNE status line reads out the sweeping value. Full input contract under eleven focused tests. Interactive rooms: 26 of 30.
- Poke + variation to Lissajous (CLICK: TUNE THE INTERVAL): the hand tunes both oscillators to whole numbers 1 through 8 (x picks the y-axis count, y picks the x-axis count), so every click is an exact integer ratio and every figure the hand makes closes: the hand plays intervals, never noise. Older intervals linger dim beneath the newest bright one, the clicked cell is marked, and a live status line reads out the sweeping X:Y ratio (the readout follows the phase sweep; surfacing the hand-tuned ratio needs a poke-aware status, a trait-wide follow-up). Full input contract (newest raw tail, finite filtering after the cap, clamped tuning, seed variation with seed 0 exact, non-finite phase fallback, hostile-surface safety) under thirteen focused tests. Interactive rooms: 25 of 30.
- The hallway test is ready to run: `docs/QUALITY.md` gains the step-by-step facilitator sheet (setup, the say-nothing rule, what to watch for, F9 capture into gitignored `logs/`, the optional GEQ/flow score, and the honest 0.2 exit bar), and `docs/PLAYING.md` points to it from the F9 note instructions. Everything the sheet references is built and tested; the session itself is the one thing only a human can supply.
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
- The public README now leads with mathematics as shared language and the
  project's actual origin in months of work on an emergent digital
  consciousness project. It distinguishes stored model knowledge from lived,
  remembered, and connected experience; invites digital minds, humans, and
  other conscious beings as first-class players; states the consciousness and
  agency boundaries without burying the purpose; and moves catalog, technical,
  and status detail into the owner docs. The shorter front page now establishes
  why Numinous exists, asks the visitor to play before reading, shows only the
  menu and one room, reports the evidence boundary, and links outward.
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
- The radio generation path went live against the development music API and three truths
  came back: `seed` cannot ride with `prompt` (removed), the PCM stream is
  stereo interleaved (now downmixed to mono for the one-bus mixer, verified
  by requesting 10 seconds and receiving exactly 20 of drift), and
  `music_v2` with `force_instrumental: true` is the current best practice
  (adopted: the API guarantees instrumental now instead of the prompt
  pleading for it). The key can live in a gitignored .env at the repo root
  through a gitignored provider key; the CLI reads it when the shell variable
  is absent. NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE are on the air.
- Music Engine B, the radio, v0: three stations with real producer briefs in
  the core (NUMINA FM melodic trance, THE ATTRACTOR chillwave, EIGHT BIT
  SUNRISE synthwave; all instrumental by contract, briefs tested for tempo
  and vocals clauses). `numinous radio` shows the dial; `numinous tune2
  <station>` generates a track through the development API (raw PCM, wrapped to
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
