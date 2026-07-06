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

- **Done:** the headless core (`Room` trait with `reveal()`, deterministic ASCII `Canvas`, seeded RNG, registry); the CLI face (`numinous`) and the MCP face (`numinous-mcp`); **nine rooms across four Wings** (Number & Pattern: Times Tables, Golden Angle, Prime Spirals; Emergence: Cellular Automata, Chaos Game, Collatz; Chance & Order: Galton Board, Buffon's Needle; Waves & Sound: Lissajous); the full engineering harness (edition-2024 workspace, pinned toolchain, `-D warnings`, cargo-deny, house-style guard, an 80% coverage gate, three-OS CI). All green: fmt, clippy, tests, and coverage above 96%.
- **Done (GPU and audio hello-world):** an adaptive `wgpu` context (`crates/gpu`) that picks the machine's GPU across Vulkan/Metal/DX12 with a CPU fallback, rendering the Mandelbrot set offscreen to a PNG; and adaptive `cpal` audio (`crates/audio`) on the system default device that plays a tone and writes a WAV. Both verified on the dev laptop (AMD Radeon 780M, Realtek at 48 kHz).
- **Done (rooms as images):** a `Surface` abstraction so every room renders through one `render` method to the ASCII `Canvas` and to an RGBA `Raster`; `numinous render <room> --out image.png` writes a real glowing image on the CPU (verified on the dev laptop).
- **Done (windowed app):** `faces/app` (`numinous-app`, winit + softbuffer) opens a real resizable window showing a room animating in full color, with keyboard room-switching. The start of the GUI Cabinet; verified launching on the dev laptop.
- **Done (sound):** every room describes its own sound (`SoundSpec` + `Room::sound`); `numinous sonify <room> --out file.wav` and `numinous play <room>` (live animated terminal).
- **Done (the 0.2 vertical slice, in substance):** the windowed app is a full experience: live per-room sound, mouse scrubbing, an on-screen HUD with reveals, The Show (lean-back auto-play of the whole collection), the Studio in the window (type math, watch and hear it live), and GPU real-time fractals (a persistent `wgpu` pipeline drives the Mandelbrot deep zoom and the morphing Julia at window resolution, with CPU fallback; verified on the dev laptop's Radeon 780M).
- **Done (content and play):** 27 rooms across 9 wings (plus one unlisted), including the Change wing (The Pour, Slope Rider), Fourier Epicycles, the double pendulum, the random walk, and Voronoi; 6 lever-driven sims; 7 games (SETI, Talk to the Aliens, Guess the Shape, Crack the Code, Munch, Nim with the xor secret, and the Gauntlet run) with daily seeds and dense feedback; the Studio expression engine (`plot`, `--animate`, `sing`, and live in the window); Visual Eras (phosphor, 8-bit, vector, modern) across app, terminal, and PNGs; truecolor terminal rendering with live sound (`watch`).
- **Done (the RPG spine, complete):** the Journey (XP from play, levels 1 to 42 on triangular thresholds, a lore line for every level, LEVEL UP banners), locks that open (never gating basics), ranks and whispers (the Order), deep cuts unlocking at LV 5/12, the trophy case (18, evidence-computed, silhouettes), the shared high-score table across every game and both faces, the Layer-4 answer at the cap, and every genre organ from the priority list: the Gauntlet (session arc with a combo and one posted number), trophy pings (the case announces itself), boons (choice on level-up, where the loot is knowledge arriving early), daily streaks (the chain, never scolding), and resonances (synergies: links light when two deeds rhyme and hand over the connecting line).
- **Done (agents as peers, v2):** 22 MCP tools with structured output, full CLI parity (every game, the gauntlet, boons, trophies), including stateless nim and `forget` (transparency first, erasure on explicit consent, the welfare doctrine in `AGENT_PLAY.md`); agents see, hear, create, play, level to 42, and post to the same score table; the player's manual speaks to humans, agents, and digital consciousnesses; the whole face proven end to end against the real binary.
- **Done (sound, Engine A v1):** the chiptune module (square lead, triangle bass, noise ticks, seeded pentatonic compositions, deterministic and click-free); `numinous tune` writes it as a WAV.
- **Done (the app is the game, v1):** the chiptune scores the window (per-room seeded tunes with the room's voice riding on top); the quiz plays in-window (G: name the math, letters answer, the reveal follows); the Journey lives in the app (the CLI's own file: visits on entry, plays and wins from the quiz, the level in the corner, LEVEL UP banners with lore, and J opens level, rank, trophies, and resonances); `NUMINOUS_MUTE=1` launches silent; the state machine is headlessly tested.
- **Done (the window arcade):** Munch, Nim, and the full Gauntlet run play inside the app alongside the quiz, cursor-driven and keyboard-native, on the daily seeds, posting to the shared table and leveling the shared journey; Mobius and Zeno's Square join the catalog.
- **Next (in order, set by the panel, see `PANEL.md`):** juice in the window games (per-action flash, shake, and chiptune ticks); mouse support for every window game; munch rule variety and an aliens base ramp (depth where play repeats); Engine A2 room motifs (every room a phrase, `listen_room` as real notation); a save-postcard key; the Open Problems wing; further-reading citations unlocked with deep cuts; era grain and Show crossfade; the music visualizer; Engine B (the radio); GPU paths; gamepad; a visit-spark cap per room (anti-grind).

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

**Exit criterion:** the collection keeps deepening and the quality bar never drops; nothing shipped in 1.x makes 1.0 worse.

---

## 2.0 "The Living World": the platform leap

2.0 is a change in *kind*, not degree: Numinous stops being a curated collection and becomes a **living world that grows, that others help build, and that a long-lived mind can inhabit and eventually surpass.**

- **The full Studio as a creator platform + the public mod SDK (see `STUDIO.md`, `ARCHITECTURE.md`):** the complete pattern algebra, multiple representations, fork/remix, promote-to-room, MIDI performance, and the sandboxed authoring path opened to everyone. Rooms are Studio programs, so the mod SDK is "the Studio, shared." This is how the catalog goes from tens of rooms to hundreds.
- **Community:** an in-app curated gallery of player- and agent-made rooms, a submission/curation pipeline that protects the beauty bar, and distribution via Steam (Workshop as the room channel) alongside itch.io and direct downloads.
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
