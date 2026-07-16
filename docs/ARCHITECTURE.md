# Architecture

How Numinous is built. Non-negotiables: it is a **real native application** (not a website in a costume, no browser, no Electron, no HTML), it runs beautifully on macOS, Linux, and Windows, it does **serious GPU/parallel math in real time**, it makes **serious real-time audio**, and it iterates fast enough to stay fun to build. The code-quality standards, pinned versions (as of July 2026), lint/test/unsafe/doc policy, and CI gates that hold this to a professor-proud bar live in `ENGINEERING.md`.

> **The one-line answer to "what language."** Numinous is written in **Rust**.
> The app presents CPU room rasters through `softbuffer` and accelerates its two
> live fractal paths with portable **`wgpu`** and WGSL. The same deterministic
> headless core powers the App, CLI, and MCP faces. Details below.

**Shipped stack, 2026-07-13:** the app uses a bespoke `winit` event loop,
`softbuffer` CPU presentation, `gilrs` standard-controller input, and targeted
`wgpu` paths for Mandelbrot and Julia. The headless core renders every room
through `Surface`; the CLI and MCP faces consume the same core. Audio uses
`cpal`, custom deterministic stereo synthesis with crossfaded loop sources,
`hound`, and a bounded `symphonia` MP3 decoder. Bevy, `fundsp`, `kira`, CUDA,
Triton, Wasmtime plugins, the full pattern DSL, bloom, and packaged installers
are not current dependencies. They remain options or roadmap targets where this
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

```rust
trait Room {
    fn meta(&self) -> RoomMeta;
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
    fn sound(&self, t: f64) -> SoundSpec;
}
```

The required methods are `meta`, `render`, and `reveal`; the others have safe
defaults that rooms override as their interaction or voice requires. `Surface`
is the rendering seam for ASCII and RGBA output. `RoomInput` is bounded,
normalized, replayable gesture data. `Motif` and `SoundSpec` keep notation and
audio face-neutral. Seeded registry constructors provide variation without
ambient randomness. Face-owned Journey, export, window, and protocol concerns
do not enter the room trait.

### Why this shape
- Rooms are cheap and isolated: a new phenomenon is one module, no engine changes.
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
│   └── audio/           # cpal output and looping sample player
├── faces/
│   ├── app/             # winit window, softbuffer, mouse/controller input, radio
│   ├── cli/             # terminal play, render, export, Studio, games
│   └── mcp/             # bounded stdio JSON-RPC surface for digital minds
├── assets/              # shipped radio and tracked screenshots
├── data/                # canonical shared Cairn
├── scripts/             # install, verification, hooks, and local utilities
└── docs/
```

**Dependency rule:** mathematical domain behavior lives in `numinous-core`.
The three faces depend on core but never on one another. `numinous-gpu` and
`numinous-audio` are adapters used by faces, not alternate owners of room logic.
Rooms are core modules registered through one registry.

**Headless in production today.** Core rendering and audio synthesis work without
a window. The CLI, MCP server, exporters, and automated suite all use that seam.

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

- **Current delivery:** source installation through the repository scripts and
  a locked release build. Signed or packaged desktop artifacts are not shipped.
- **Current sharing:** PNG postcards, `.num` Studio files and links, and WAV
  audio export. Loop or video export and operating-system URL associations are
  future work.
- **Current CI:** house style, dependency policy, coverage, format plus clippy
  plus tests, and macOS, Ubuntu, and Windows builds. There is no automated
  beauty screenshot job.
- **Release path:** packaged artifacts belong to 0.6. The public launch gate is
  0.9.

## Remaining technical decisions

1. Select packaged artifact formats, signing, checksums, and update behavior.
2. Specify the bounded pattern DSL and its compatibility contract.
3. Design the audio scheduler and master bus around measured latency.
4. Define native `.num` associations, URL handling, and loop export.
5. Add GPU paths only where profiling shows a user-visible benefit.
