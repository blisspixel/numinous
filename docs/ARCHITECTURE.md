# Architecture

How Numinous is built. Non-negotiables: it is a **real native application** (not a website in a costume, no browser, no Electron, no HTML), it runs beautifully on macOS, Linux, and Windows, it does **serious GPU/parallel math in real time**, it makes **serious real-time audio**, and it iterates fast enough to stay fun to build. The code-quality standards, pinned versions (as of July 2026), lint/test/unsafe/doc policy, and CI gates that hold this to a professor-proud bar live in `ENGINEERING.md`.

> **The one-line answer to "what language."** Numinous is written in **Rust**.
> The app presents CPU room rasters through `softbuffer` and accelerates its two
> live fractal paths with portable **`wgpu`** and WGSL. The same deterministic
> headless core powers the App, CLI, and MCP faces. Details below.

**Shipped stack, 2026-07-18:** the app uses a bespoke `winit` event loop,
`softbuffer` CPU presentation, `gilrs` standard-controller input, and targeted
`wgpu` paths for Mandelbrot and Julia. A disabled Sensory Lift spike now owns a
linear HDR and bloom post stack plus direct `wgpu` surface presentation, but it
does not yet replace the App's shipped presentation path. Its production
presenter exposes typed read-only adapter and frame diagnostics to one bounded
platform probe. The Windows, macOS, and Linux CI matrix drives the exact App
composition and recovery boundary, while physical release-profile pacing stays
a separate promotion gate. The headless core
renders every room through `Surface`; the CLI and MCP faces consume the same core. Audio uses
`cpal`, custom deterministic stereo synthesis with crossfaded loop sources,
`hound`, and a bounded `symphonia` MP3 decoder. Bevy, `fundsp`, `kira`, CUDA,
Triton, Wasmtime plugins, the full pattern DSL, and packaged installers are not
current dependencies. They remain options or roadmap targets where this
document names them.

## First, untangle the question

"What language is best for this" is really two questions, because a game like this has two very different layers, and the best answer for each is different:

1. **The application / engine layer.** Windowing, input, the render pipeline, audio, UI, the Cabinet, scene/room management, saving, packaging, shipping to three OSes. This wants a **mature, fast, cross-platform systems language with a real graphics stack**.
2. **The compute-kernel layer.** The actual heavy math: fractal escape-time, reaction-diffusion, N-body, cellular automata over millions of cells, FFTs for audio, 4D projections, particle systems. This wants **portable, high-throughput GPU compute**.

Most of the languages in the shortlist (Triton, Gluon, CUDA C++, SYCL, Kokkos, RAJA, Chapel, Julia-GPU, Bend, Mojo) are answers to **layer 2 only**. None of them is a good answer to layer 1. You do not ship a cross-platform consumer game *in* Triton or Chapel. So the real decision is: pick one great engine language, and pick one portable way to feed the GPU.

## The recommendation

**Engine layer: Rust with a bespoke `winit` shell, `softbuffer` presentation,
and targeted `wgpu` acceleration.**
**Compute layer: CPU reference renderers everywhere, with WGSL shaders where
measurement justifies a portable GPU path.**
**Creative/live-coding target: a bespoke mathematical pattern DSL embedded in
the Rust host, plus raw WGSL for shader specialists. Neither public authoring
surface is shipped yet.**

Why this specific combination wins for *math + games + visualization + fun + truly cross-platform*:

- **`wgpu` is the one graphics stack that targets every desktop OS and every GPU from one codebase.** It compiles to Vulkan (Linux/Windows), Metal (macOS), and DX12 (Windows). The heavy math runs on **any** GPU: NVIDIA, AMD, Intel, and Apple Silicon, not just NVIDIA. This single fact eliminates CUDA as the primary compute path, because CUDA cannot run on a Mac, and "runs on Mac" is a hard requirement. (wgpu is a native GPU abstraction over Vulkan/Metal/DX12; it is not a browser and ships nothing web.)
- **Why not take the expedient web-wrapper route.** Electron, HTML, and webview
  shells are ruled out because they add a runtime layer the product does not
  need and work against its native, offline identity. The Rust workspace gives
  direct ownership of windows, audio, rendering, and one binary per face.
- **WGSL compute shaders give you real GPU parallelism for the math** (reaction-diffusion, Game of Life at millions of cells, Mandelbrot, particle fields) on that same portable stack. You write the kernel once, it runs everywhere.
- **Rust is the modern "we love this craft" systems language.** It is exactly the culture fit for a project that is an obsessive love letter to math: strong types make the Room contract airtight, zero-cost abstractions keep it fast, and the native graphics and audio ecosystem is mature.
- **The bespoke shell is now a measured decision.** The shipped app uses `winit`
  and `softbuffer`, with raw `wgpu` only where a room benefits. Bevy remains an
  evaluated alternative, not part of the current architecture.
- **Sharing is native, not a browser build.** PNG postcards, short looping APNG
  export (App key L), `.num` expression files, matching links, and WAV export
  exist today. App-side deep-link reopening, optional GIF/MP4 packaging, and
  operating-system URL registration remain roadmap work.
- **Audio is first-class in Rust:** `cpal` supplies cross-platform output while
  the workspace owns deterministic synthesis and bounded file rendering. More
  advanced DSP can be added only when the musical design and measured budget
  require it.

### Honest scorecard of the shortlist (for *this* project)

| Option | What it is | Verdict for Numinous |
| --- | --- | --- |
| **CUDA C++** | NVIDIA's mature, fastest GPU model | Fastest, but NVIDIA-only. Disqualified as the primary path because it cannot run on macOS or AMD/Intel GPUs. Keep as an **optional fast path** for NVIDIA-only "extreme" rooms (deep Mandelbrot perturbation, massive N-body). |
| **Triton** | Python-authored GPU kernels, from the ML world | Wrong domain (ML kernels), NVIDIA-centric. Not a rendering path. Possible optional accelerator for a couple of compute-only rooms; not the baseline. |
| **Gluon** | Lower-level, Triton-adjacent GPU | Too niche and low-level for a game. No. |
| **Chapel** | HPC cluster parallelism | Built for supercomputers, not real-time interactive graphics/audio. No. |
| **SYCL / OpenMP / Kokkos / RAJA** | C++ cross-platform parallel models | Portable but complex, slow to iterate, and only the compute layer: you would still bolt on a C++ engine. High pain, low fun. No. |
| **Bend (HVM)** | Experimental massively-parallel high-level language, runs on GPU | Genuinely exciting and on-brand, but too immature to bet the app on today. Perfect candidate for a single **experimental "compute universe" easter-egg room** later (see Lore), not the foundation. |
| **Mojo** | Python-superset, MLIR, systems+AI speed | Promising, young, no graphics/game/audio ecosystem yet. Revisit in a year. Not now. |
| **Julia + GPU** (CUDA.jl / Metal.jl / KernelAbstractions.jl, Makie) | High-level scientific/math language with vendor-agnostic GPU and beautiful viz | The **strongest alternative soul** (see below). Unmatched for writing math that reads like math. Weaker for shipping a polished cross-platform game shell with tight custom audio/UI. |
| **Rust + winit + wgpu** | Systems language + bespoke native shell + portable GPU graphics/compute | **The shipped choice.** It supports a native app, all three OSes, targeted portable GPU work, real-time audio, and a small dependency surface. |

### The two other serious routes (so we know we considered "done well")

If not Rust + wgpu, only these are serious enough to keep it a real app; everything web-based is out.

- **C++ + Vulkan** (optionally a lib like Magnum). The maximum-control, most-mature route, the same class of tech AAA engines are built on. Gives everything Rust does and slightly more raw ceiling, at the cost of memory-safety footguns and slower iteration. Choose only if a specific need demands it; Rust gives ~95% of the power with far less pain.
- **Godot 4** (the engine route). A real, native, cross-platform engine with compute shaders, a scene/UI system, and export to all three OSes. It was not selected: the shipped bespoke shell keeps the face thin and gives direct control over deterministic headless rendering and audio.

### The alternative soul: Julia

If the project's identity leans harder toward *"we want to write the math itself as beautifully as possible and have it just run on any GPU,"* the serious alternative is **Julia**: multiple dispatch makes math code read like a textbook, **KernelAbstractions.jl** compiles one kernel to CUDA/AMD/Metal/oneAPI (true portability, like wgpu but in a math-first language), and **Makie.jl** is a genuinely gorgeous GPU-accelerated visualization library. It is arguably the more "autistic love of math" choice.

The catch is the app-shell story: shipping a tightly-polished, custom-UI,
custom-audio consumer *game* to three OSes is harder in Julia than in the
shipped Rust and `winit` shell. Startup time, packaging, game input, and audio
ergonomics are weaker. Julia remains useful for isolated mathematical
prototypes when that reduces validation time, but it is not a runtime
dependency.

## The compute-kernel strategy

- **Baseline (every room, every platform):** deterministic CPU rendering through
  `Surface`, with a time-budgeted app downscale for expensive live frames.
- **Shipped GPU path:** WGSL through `wgpu` for Mandelbrot and Julia, with CPU
  fallback and deterministic headless exports.
- **Measured presentation candidate:** the disabled App `gpu-post` feature
  sends the fully composed room raster through one five-pass linear HDR and
  bloom implementation shared with deterministic offscreen validation. The
  window path keeps one frame in flight and writes its final tone-map pass into
  the acquired sRGB surface texture without an intermediate output copy or
  readback. Its presenter owns exactly one backend, skips transient surface
  unavailability, recreates a lost GPU surface once, and changes permanently to
  the ordinary `softbuffer` path with visible and logged notice if recovery
  fails. The feature remains a candidate until three-platform proof closes.
- **Optional future fast path:** CUDA or Triton only if measurement proves that
  a specific extreme room cannot meet its budget through portable WGSL.
- **Experimental sandbox (later, easter-egg): Bend/HVM** as a literal "alternate compute universe" a curious user can switch a room into, which is both a real technical experiment and perfectly on-theme with the Lore. Never on the critical path.

## The audio + live-coding stack

- **Real-time synthesis:** `cpal` for output and workspace-owned deterministic
  DSP for room voices and 128-step, four-cycle stereo chiptune arrangements,
  with one shared 16 kHz room-bed source resampled to the device rate, smoothed
  gain, focus ramps, source crossfades, explicit room, Studio, or radio source
  ownership, global keyboard and controller gain controls, and separate
  validated radio playback. The App renders the effective source, level, and
  silence reason through one persistent HUD state.
  A shared mix bus and
  sample-accurate scheduler remain roadmap work (see `MUSIC.md` and `SOUND.md`).
- **Headless room-bed evidence:** core owns the 16 kHz stereo arrangement and
  fixed-order signal analysis. The App consumes that source directly. CLI
  `sonify --layer room-bed` writes its exact PCM16 projection, while MCP
  `listen_room` exposes either a compact typed summary or every bounded event
  plus pre-master signal features. MCP never transports the sample buffer or a
  machine-local path. This is one shared contract across faces, not three
  reimplementations of the score.
- **The Studio today:** a bounded expression engine shared by the app, CLI, and
  MCP face. The larger pattern DSL, multiple synchronized representations, and
  a safe shader authoring surface remain staged creator work. See `STUDIO.md`.
- **Built-in radio:** station identity lives in the headless core, and the app validates and plays the source-shipped V0 MP3 soundtrack through a bounded pure Rust decoder. A cache override remains available for development. (See `MUSIC.md`.)

## The Room contract (the core abstraction)

Everything playable is a **Room**, a self-contained module implementing one interface. The engine knows nothing about math; rooms know nothing about packaging. This seam is also the future public SDK.

Core also owns the first directed score above individual rooms. `show.rs`
binds the six canonical `room_walk` steps to nonspoiling questions, exact look
roles and phases, reduced-motion projection, and deterministic seeded
variation. It does not own clocks, sessions, audio transport, or persistence.
The MCP `show` module renders one cue, validates its closed protocol shape, and
returns explicit continuation. Native all-room Show direction remains separate
and later Arc work.

```rust
trait RoomMetadata {
    fn meta(&self) -> RoomMeta;
}

trait Room: RoomMetadata {
    fn render(&self, surface: &mut dyn Surface, t: f64);
    fn reveal(&self) -> &'static str;
    fn postcard_t(&self) -> f64;
    fn motif(&self) -> Option<Motif>;
    fn status(&self, t: f64) -> Option<String>;
    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String>;
    fn verb(&self) -> Option<&'static str>;
    fn render_poked(&self, surface: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]);
    fn render_input(&self, surface: &mut dyn Surface, t: f64, inputs: &[RoomInput]);
    fn deep_cuts(&self) -> &'static [&'static str];
    fn goal(&self) -> Option<&'static str>;
    fn goal_met(&self, t: f64, inputs: &[RoomInput]) -> bool;
    fn parameter_sound(&self, t: f64, inputs: &[RoomInput]) -> Option<ParametricSound>;
    fn interaction_stereo(&self, inputs: &[RoomInput], sample_rate: u32) -> Option<Vec<f32>>;
    fn sound_input(&self, t: f64, inputs: &[RoomInput]) -> SoundSpec;
    fn sound(&self, t: f64) -> SoundSpec;
}
```

Custom rooms supply `meta`, `render`, and `reveal`; the other methods have
safe defaults that rooms override as their interaction or voice requires.
Built-in `RoomMetadata` implementations, module declarations, and replayable
constructors are generated from one typed catalog declaration. The inherited
`RoomMetadata::meta` method is the single object-safe discovery dispatch used
by faces. `Surface`
is the rendering seam for ASCII and RGBA output. `RoomInput` is bounded,
normalized, replayable gesture data. `Motif` and `SoundSpec` keep notation and
audio face-neutral. Seeded registry constructors provide variation without
ambient randomness. `parameter_sound` describes a continuous input-controlled
voice, while `interaction_stereo` optionally renders one bounded discrete
consequence after a face accepts its newest down, move, or lift. The App offers
each accepted event through this room-neutral seam; each room admits only the
event kind whose consequence it owns. Real-time faces prepare that buffer off
the callback and own its playback lifetime. Face-owned Journey,
export, window, and protocol concerns do not enter the room trait.

### Why this shape
- Room behavior is cheap and isolated: a new phenomenon is one module plus one
  coherent catalog entry, with no face changes.
- `ROOM_CATALOG` lets a face or agent inspect every listed room without
  constructing renderers. Hidden rooms use the same invariants but stay out of
  public discovery.
- The faces own clocks, input collection, persistence, and presentation while
  the core owns deterministic room behavior.
- Gesture and poke defaults preserve compatibility while allowing selected
  rooms to add held semantics without face-specific domain logic.
- This trait is the low-level extension seam. A later creator milestone may
  publish a supported SDK after compatibility and sandbox requirements are met.

### Authoring paths

Today every shipped room is a first-party Rust module implementing `Room` and
registered in `numinous-core`. Formula Studio expressions are bounded creative
artifacts, not room plugins. Two additional authoring paths are designed but
not built: declarative room programs in the future pattern DSL, and capability-
sandboxed compiled extensions. `STUDIO.md` owns the staged creator plan.

### Extensibility, and the safety of untrusted extensions

The design goal is that anyone can add a room without endangering the person
running it. The `Room` trait plus `Surface` are the built extension seam, but
the public plugin runtime is not built. The planned trust tiers are:

- **Tier 0, first-party (trusted):** native Rust rooms, compiled in and code-reviewed. Full power; the trust comes from review.
- **Tier 1, planned Studio DSL:** declarative patterns and expressions with no
  ambient filesystem or network authority.
- **Tier 2, planned compiled plugins:** WebAssembly behind explicit host
  capabilities, memory limits, fuel metering, deterministic inputs, and no raw
  GPU access. No runtime has been selected or added.

Curation remains a beauty and correctness gate, not a substitute for the
future sandbox. `STUDIO.md` and `QUALITY.md` define the evidence required before
either untrusted tier can be called safe or shipped.

## Module architecture (Rust workspace)

```
numinous/
├── crates/
│   ├── core/            # rooms, sims, games, Studio math, persistence, audio specs
│   ├── gpu/             # optional wgpu fractal renderer with CPU fallback
│   ├── audio/           # cpal output and looping sample player
│   └── broadcast/       # consent, pairing, framing, identity, bounded queue
├── faces/
│   ├── app/             # winit window, softbuffer, mouse/controller input, radio
│   ├── cli/             # terminal play, render, export, Studio, games
│   └── mcp/             # bounded stdio JSON-RPC surface for digital minds
├── assets/              # shipped radio and tracked screenshots
├── data/                # canonical shared Cairn
├── plugins/             # portable Agent Plugins discovery package and play skill
├── scripts/             # install, verification, hooks, and local utilities
└── docs/
```

**Dependency rule:** mathematical domain behavior lives in `numinous-core`.
The three production faces depend on core but never on one another. The MCP
integration suite uses the App library as a development-only dependency so one
test can exercise the shipped viewer against the shipped MCP subprocess without
duplicating either face. `numinous-gpu` and
`numinous-audio` are adapters used by faces, not alternate owners of room logic.
`numinous-broadcast` owns face-independent local session transport primitives,
typed public replay values, and the compatibility identity derived from core
catalog metadata. It never owns gameplay or persistence. MCP depends on that
crate through a thin producer adapter. The App depends on it through a
face-local loopback listener and read-only presentation adapter. Rooms are core
modules declared through one catalog and constructed through the registry
facade. The Gauntlet follows the same rule: one typed core puzzle owns seeded
construction, stage grading, combo math, reveal semantics, and leaderboard
identity. App, CLI, MCP, and Watch Agent only collect or present its values.
Core persistence also resolves the seven managed local-state paths through one
environment precedence rule. Faces consume those resolved paths rather than
reimplementing home-directory fallback or per-store overrides. A selected
`NUMINOUS_RADIO` soundtrack remains user-owned and outside managed cache
erasure. Focused `local_state` modules in the CLI and MCP faces translate that
one core contract into terminal prose or structured protocol results; neither
module reimplements persistence rules. The CLI also keeps accessibility switch
interpretation, `NO_COLOR` policy, known-limit disclosure, and report formatting
in a focused face-local `access` module. The command entry point only gathers
raw environment values and prints the result. A focused CLI `render_input`
module owns render size bounds, static hand-point and pointer-gesture parsing,
their mutual exclusion, and the typed input projection shared by render,
share, sonify, and Studio paths. Room behavior remains in core. The MCP face
has the corresponding `room_input` adapter for bounded JSON hand points and
gestures, mutual exclusion, canonical echo, and interaction-aware render and
status projection. A focused MCP `transport` adapter owns bounded newline
framing, overflow resynchronization without a second request-sized allocation,
and flushed one-line response writes. JSON-RPC semantics and dispatch remain
in the request entry point. A focused MCP `sim_tools` adapter owns simulation
discovery text, lever argument validation, and structured result projection;
core retains simulation metadata, bounds, rendering, and readouts. A focused
MCP `studio_tools` adapter owns Formula Jam discovery, portable creation
projection, optional audio attachment, and encounter receipts. A focused MCP
`game_tools` adapter owns stateless game replay and result presentation for
Hackenbush, Party, Fifteen, Quiz, Munch, Munch Arcade, and Nim, plus the shared
score table projection. Core retains deals, legality, grading, state
transitions, and score persistence. A focused CLI
`studio` adapter owns raw source-mode
selection, terminal error projection, bounded capsule loading, never-clobber
save and fork writes, sing-input resolution, and open-report formatting. Core
continues to own Studio parsing, evaluation, request bounds, rendering, melody
construction, capsule validity, and lineage semantics. A focused CLI
`game_input` adapter owns the
bounded terminal record reader, overflow resynchronization, neutral departure
presentation, and the games' `?` explanation door. Game rules, progression,
scoring, and concepts stay outside that adapter. The MCP face also keeps
protocol discovery, legacy negotiation,
server identity, and its immutable 40-tool JSON Schema catalog in a focused
`catalog` module. The request entry point retains transport validation,
dispatch, result decoration, and domain invocation. Its 214 request-dispatch
and cross-boundary unit regressions live in the sibling `tests` module, which
retains private module access without enlarging the production entry point.
The App preference store uses a strict std-only core schema for volume, mute,
Visual Era, and window mode. Core owns bounded reads, lock coordination, atomic
replacement, inventory, and complete erasure. The App owns only applying those
values to its window and audio adapters. Unsupported or malformed preference
documents apply no partial state and are preserved for diagnosis.
Core also owns `TemporalPair`, the exact validated origin and destination used
for two-observation room comparison. MCP renders both observations through the
ordinary `Room` contract and projects their existing `Canvas::delta`; no room
gains a temporal method or hidden session. The broadcast crate owns the shared
2,304-cell-per-observation public budget so producer and Watch Agent validate
the same size boundary. The top-level MCP frame and native viewer replay remain
the destination, while the additive temporal object carries the origin and its
directional delta. An action with explicit engineered Aha controls stays on the
exact public text path because the visible overlay is still MCP-owned; Watch
Agent does not claim a partial native reconstruction. This live evidence does
not create a Numinous Encounter Receipt or journal entry.

**Headless in production today.** Core rendering and audio synthesis work without
a window. The CLI, MCP server, exporters, and automated suite all use that seam.
The MCP stdio boundary is dual-era: legacy initialization and modern
2026-07-28 per-request metadata dispatch into the same tool functions. Protocol
discovery and the static tool contract stay in the face-local `catalog` module;
result decoration, caching hints, and multi round-trip input handling stay in
the request entry point. Mathematical poses, grading, rendering, and
persistence stay in core.

## Agent participation and interoperability stack

Agent participation is a product architecture, not one protocol stretched
beyond its purpose. Each layer has one owner and one prohibition:

| Layer | Pinned target | Owns | Must not own |
| --- | --- | --- | --- |
| Agent Plugins | 1.0.0 Working Draft | Portable discovery of the play skill and installed MCP server | Gameplay, player identity, persistence, or orchestration |
| Agent Skills | Current specification, reviewed 2026-08-13 | Play-first guidance and the consent boundary | Domain truth or hidden host state |
| MCP | 2026-07-28 plus documented compatibility eras | Bounded live perception, action, creation, and consent controls | Private host cognition or automatic transcripts |
| Headless core | Numinous replay ABI | Deterministic domain state, grading, rendering, and exact replay semantics | Face or model policy |
| Native journal | Numinous journal schema | Opt-in local records, correction, retention, and erasure | Automatic interpretation of a participant |
| Open Knowledge Format | v0.2 | Player-approved portable knowledge projection | Live game state, synchronization, or canonical persistence |
| Broadcast | Numinous local wire protocol | Consented, allowlisted public event projection | Control of the player or access to private activity |
| App viewer | Matching Numinous replay identity | Read-only local witnessing | Tool injection or inferred private activity |

The standards were reviewed on 2026-08-13. Agent Plugins 1.0.0 is a Working
Draft with independently versioned plugin and MCP schemas. The current official
Open Knowledge Format specification is v0.2; the reviewed upstream repository
head was `374e0bc4c644310ff56cdf9c0fe81eccdec862b0`. A moving `latest` is never a
compatibility promise. Each upgrade is a focused change with pinned fixtures,
validation, and release evidence.

The live path is deliberately simple:

```text
private player host or local model
    -> Agent Plugins discovery and play guidance
    -> bounded MCP face
    -> deterministic headless core
       -> typed result returned to the player
       -> explicit journal record -> native journal -> native, OKF, or portable export
       -> consent filter -> typed broadcast -> read-only App viewer
```

Agent Plugins packages the doorway. It does not install the
`numinous-mcp` binary, so binary installation and plugin loading remain two
explicit steps. Its `${PLUGIN_DATA}` location may later be offered as a
client-managed profile location, but it is never a player identity and cannot
silently split or migrate continuity. MCP Apps remain a possible runtime
enhancement with complete text and structured fallbacks; Agent Plugins v1 does
not define a portable app component.

OKF remains an export projection from native typed evidence. The built
`portable-1` evidence capsule is an explicit, bounded `export_journal` mode. Its
closed manifest hashes a sorted payload containing the native journal page, an
OKF v0.2 projection, privacy and retention manifests, and optional caller-supplied
Studio creation and replay-verified Numinous Encounter Receipt. Creation input
is capsule data, never a path, and is emitted as canonical `.num` text with
identity and lineage intact. The export creates no file and returns no host
path. A Numinous Encounter Receipt is a native replay and provenance artifact,
not an OKF Attested Computation Receipt or OKF projection. Raw frames, audio
buffers, host private prompts, host hidden reasoning, arbitrary host logs, and
mutable session state are not independently collected. Player-authored journal
fields are preserved exactly and are not scanned for secrets. Import remains
deferred until path safety, byte and entry bounds, provenance, unknown-field
preservation, preview, atomic commit, merge rules, and verified erasure are
specified and tested.

## Key technical concerns

- **Frame pacing:** the live app targets a 33 ms frame budget and adaptively
  reduces render resolution when a room exceeds it. Hardware-specific GPU and
  audio behavior still requires testing on representative machines.
- **Determinism and timing:** rooms are phase-based and deterministic. Faces own
  their clocks. Audio is not yet a master clock or a sample-accurate scheduler.
- **GPU scope:** only Mandelbrot and Julia have shipped `wgpu` paths. CPU
  renderers remain the portable baseline and the deterministic export path.
- **Accessibility:** hard mute plus keyboard and pointer operation are shipped.
  Reduce-motion controls, color controls, and broader accessibility evidence
  remain part of the 0.5 roadmap scope.

## Build and distribution

- **Current delivery:** GitHub prereleases carry deterministic, checksummed
  archives for Windows x64, Linux x64, macOS Intel, and macOS Apple silicon,
  plus one shared soundtrack archive. The repository installers verify each
  archive and closed payload manifest before replacing a managed install. A
  separate stable soundtrack content checksum covers only the licensed radio
  files, so binary-only releases do not force another large audio download.
  Every binary archive also carries the pinned portable Agent Plugins package;
  its MCP declaration launches the installed `numinous-mcp` executable and does
  not duplicate runtime or domain logic.
  `numinous update` stages the matching installer, waits for the running CLI to
  exit, and installs the latest published release. The release SBOM joins the
  exact locked Rust graph with hashes, formats, architectures, and direct
  header-declared imports from all twelve packaged PE, ELF, and Mach-O
  executables. Runtime-resolved native versions and soundtrack contents remain
  outside that inventory. Artifacts are not yet platform-signed.
- **Current sharing:** PNG postcards, short-loop APNG bundles, `.num` Studio
  files and links, and WAV audio export. Longer video export and
  operating-system URL associations are future work.
- **Current CI:** house style, dependency policy, coverage, format plus clippy
  plus rustdoc, doctests, all-target tests, and macOS, Ubuntu, and Windows
  builds. The exact historical dependency-migration receipt is also verified
  on every PR; `PERFORMANCE.md` owns its workload boundaries and evidence
  limits. There is no automated beauty screenshot job.
- **Local session broadcast, native room, Studio, game, and sound viewer, and subprocess proof built:**
  the App and MCP production faces remain independent. The shared broadcast
  crate owns one-use loopback pairing, server-first host proof, strict bounded
  framing, replay compatibility identity, typed public tool events, atomic
  consent epochs, ordered control barriers, and a fixed event queue. MCP adds
  one consent control, an exhaustive 23-public, 9-private, 1-control policy,
  daily-seed replay normalization, one serialized lifecycle, a bounded
  failed-start budget, and separate socket monitor and writer workers. The App
  adds exact receive-side session, compatibility, epoch, transition, sequence,
  and gap validation, an ephemeral loopback listener, and a read-only Watch
  Agent surface for typed action identity, input JSON, and human-readable MCP
  result text. Valid `play_room` actions are revalidated and reconstructed from
  the same core `Room` implementation at the local viewport size; invalid replay
  values fall back to typed text. Successful `plot_expression` actions are
  strictly parsed and rendered through one deterministic curve sampler shared
  with the live App Studio. Successful public `nim` actions are reduced through
  the shared core replay rules, attested against the complete MCP result, and
  drawn through one bounded heap renderer shared with the live App. Its ring
  retains at most 256 serialized public
  events or 16 MiB, persists nothing, and clears on close. A development-only
  MCP integration test opens that exact viewer and drives the actual MCP binary
  through the Times Tables explore, challenge, K5 goal, reveal, and stop path.
  Separate real sessions prove Formula Jam expression delivery, native Nim
  delivery, Munch, Arcade, Quiz, and Gauntlet delivery, and exact
  local-viewport body pixels. `INTERFACES.md` owns the complete contract and
  privacy boundary. Strictly accepted native room, Studio, Munch, Arcade, Quiz,
  and Gauntlet selections derive bounded mono sound from shared deterministic
  state at a fixed 16 kHz source rate, then the audio adapter resamples to the
  output device. Nim remains silent. One public-sequence owner prevents
  render-loop restarts; unsupported or mismatched selections retain typed text
  and publish silence. Mute, volume, focus, scrub, close, room-score restoration,
  and live-radio restoration remain App-local behavior with no control edge to
  MCP.
- **Release path:** deterministic Windows, macOS, and Linux archives, closed
  payload manifests, checksums, installers, updates, and CI self-tests are built.
  Tagged publication is now ordered after a closed-set audit and GitHub keyless
  SLSA build-provenance attestation whose subject set covers every archive, with
  the signed JSONL bundle attached to the release. The same audit generates a
  deterministic SPDX 2.3 inventory of the locked all-feature Rust graph plus
  exact hashes, formats, architectures, and direct header-declared imports for
  every packaged executable. A separate keyless SBOM attestation binds it to
  every archive. Platform code signing, notarization, runtime-resolved native
  versions, embedded per-binary Rust reachability, clean-machine evidence, and
  broader platform certification remain 0.6 work. The public launch gate is
  0.9.

## Remaining technical decisions

1. Select platform signing, notarization, runtime-native resolution evidence,
   and embedded per-binary Rust reachability policy beyond the current SBOM.
2. Specify the bounded pattern DSL and its compatibility contract.
3. Design the audio scheduler and master bus around measured latency.
4. Define native `.num` associations, URL handling, and loop export.
5. Add GPU paths only where profiling shows a user-visible benefit.
