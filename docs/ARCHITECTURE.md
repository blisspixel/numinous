# Architecture

How Numinous is built. Non-negotiables: it is a **real native application** (not a website in a costume, no browser, no Electron, no HTML), it runs beautifully on macOS, Linux, and Windows, it does **serious GPU/parallel math in real time**, it makes **serious real-time audio**, and it iterates fast enough to stay fun to build. The code-quality standards, pinned versions (as of July 2026), lint/test/unsafe/doc policy, and CI gates that hold this to a professor-proud bar live in `ENGINEERING.md`.

> **The one-line answer to "what language."** The app is written in **Rust**, renders with **`wgpu`** (native Vulkan on Linux/Windows, Metal on macOS), and does its heavy math in **compute shaders (WGSL)**. None of the parallel-compute languages in the shortlist (Bend, Mojo, Triton, Gluon, Chapel, CUDA, SYCL/Kokkos/RAJA, Julia) is the *app* language, they are all kernel-only, and you do not ship a cross-platform desktop app in any of them. Rust is the app; portable compute shaders are the "CUDA-but-on-every-GPU-including-Mac"; CUDA itself is an optional NVIDIA-only fast path, never the baseline. Details below.

## First, untangle the question

"What language is best for this" is really two questions, because a game like this has two very different layers, and the best answer for each is different:

1. **The application / engine layer.** Windowing, input, the render pipeline, audio, UI, the Cabinet, scene/room management, saving, packaging, shipping to three OSes. This wants a **mature, fast, cross-platform systems language with a real graphics stack**.
2. **The compute-kernel layer.** The actual heavy math: fractal escape-time, reaction-diffusion, N-body, cellular automata over millions of cells, FFTs for audio, 4D projections, particle systems. This wants **portable, high-throughput GPU compute**.

Most of the languages in the shortlist (Triton, Gluon, CUDA C++, SYCL, Kokkos, RAJA, Chapel, Julia-GPU, Bend, Mojo) are answers to **layer 2 only**. None of them is a good answer to layer 1. You do not ship a cross-platform consumer game *in* Triton or Chapel. So the real decision is: pick one great engine language, and pick one portable way to feed the GPU.

## The recommendation

**Engine layer: Rust + `wgpu`, optionally inside the Bevy engine.**
**Compute layer: WGSL compute shaders as the portable baseline, with an optional CUDA/Triton fast path behind a feature flag for NVIDIA-only spectacle rooms.**
**Creative/live-coding layer: a bespoke pattern DSL (Strudel/TidalCycles-style) embedded in the Rust host, plus raw WGSL exposed for shader-heads.**

Why this specific combination wins for *math + games + visualization + fun + truly cross-platform*:

- **`wgpu` is the one graphics stack that targets every desktop OS and every GPU from one codebase.** It compiles to Vulkan (Linux/Windows), Metal (macOS), and DX12 (Windows). The heavy math runs on **any** GPU: NVIDIA, AMD, Intel, and Apple Silicon, not just NVIDIA. This single fact eliminates CUDA as the primary compute path, because CUDA cannot run on a Mac, and "runs on Mac" is a hard requirement. (wgpu is a native GPU abstraction over Vulkan/Metal/DX12; it is not a browser and ships nothing web.)
- **Why not do it "the easy shitty way."** Electron/HTML/webview apps are ruled out on purpose: they cannot do sample-accurate audio synthesis, cannot hit a locked 60/120fps with millions of GPU-computed elements, and never feel like a real app. Rust + wgpu is the serious route, native windows, native GPU, native audio, one binary per OS.
- **WGSL compute shaders give you real GPU parallelism for the math** (reaction-diffusion, Game of Life at millions of cells, Mandelbrot, particle fields) on that same portable stack. You write the kernel once, it runs everywhere.
- **Rust is the modern "we love this craft" systems language.** It is exactly the culture fit for a project that is an obsessive love letter to math: strong types make the Room contract airtight, zero-cost abstractions keep it fast, and the ecosystem (Bevy, wgpu, cpal, fundsp) is right here.
- **Bevy** (an ECS game engine built on wgpu) gives you the game scaffolding for free: input, windowing, scenes, hot-reloadable assets, an ergonomic render graph, and a passionate community. Use it for the shell and rooms; drop to raw wgpu compute for the hot kernels.
- **Sharing is native, not a browser build.** No web companion. Sharing happens through the app: high-quality video/image export of loops and stills, plus reproducible **seed strings / `.num` files** and a **`numinous://` URL scheme** that reopen an exact configuration in the installed app. The clip is the viral object; you do not need a website to spread it.
- **Audio is first-class in Rust:** `cpal` (cross-platform audio I/O), `fundsp` (functional DSP and synthesis), `kira` (game audio), and FFT crates for the sonification. The "everything is an instrument" pillar needs sample-accurate synthesis, and Rust delivers it natively (a browser could not, to the same standard).

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
| **Rust + wgpu (+ Bevy)** | Systems language + portable GPU graphics/compute + game engine | **The pick.** Only option that nails all of: native app, all-GPU-vendors, all three OSes, real-time audio, fast iteration, and craft-culture fit. |

### The two other serious routes (so we know we considered "done well")

If not Rust + wgpu, only these are serious enough to keep it a real app; everything web-based is out.

- **C++ + Vulkan** (optionally a lib like Magnum). The maximum-control, most-mature route, the same class of tech AAA engines are built on. Gives everything Rust does and slightly more raw ceiling, at the cost of memory-safety footguns and slower iteration. Choose only if a specific need demands it; Rust gives ~95% of the power with far less pain.
- **Godot 4** (the engine route). A real, native, cross-platform engine with compute shaders, a scene/UI system, and export to all three OSes, and you can write hot paths in Rust via GDExtension. Faster to stand up the app shell. The tradeoff for *this* project: we want unusually tight control over a custom generative aesthetic and custom audio DSP (the "everything is an instrument" pillar), which is more direct in a from-scratch wgpu + `cpal`/`fundsp` app than through an engine's rendering and audio systems. Worth a Phase 0 spike against bespoke wgpu before committing.

### The alternative soul: Julia

If the project's identity leans harder toward *"we want to write the math itself as beautifully as possible and have it just run on any GPU,"* the serious alternative is **Julia**: multiple dispatch makes math code read like a textbook, **KernelAbstractions.jl** compiles one kernel to CUDA/AMD/Metal/oneAPI (true portability, like wgpu but in a math-first language), and **Makie.jl** is a genuinely gorgeous GPU-accelerated visualization library. It is arguably the more "autistic love of math" choice.

The catch is the app-shell story: shipping a tightly-polished, custom-UI, custom-audio consumer *game* to three OSes is harder in Julia than in Rust/Bevy (startup time, packaging, game-input, and audio ergonomics are weaker). **Pragmatic hybrid worth considering:** prototype and validate the *math* of each room in Julia (fast, joyful, correct), then port the proven kernel to WGSL/Rust for the shipped app. You get Julia's math-expressiveness during design and Rust's shipping strength at runtime.

## The compute-kernel strategy

- **Baseline (every room, every platform): WGSL compute shaders** dispatched via wgpu. Portable, fast enough for essentially all rooms.
- **Optional fast path (feature-flagged, NVIDIA only): CUDA / Triton** for a small number of "extreme" rooms where we want to push far past what a portable shader comfortably does (deep fractal perturbation, huge particle N-body). The room detects the backend and gracefully falls back to WGSL everywhere else. This is how we get "fastest when possible, runs everywhere always."
- **Experimental sandbox (later, easter-egg): Bend/HVM** as a literal "alternate compute universe" a curious user can switch a room into, which is both a real technical experiment and perfectly on-theme with the Lore. Never on the critical path.

## The audio + live-coding stack

- **Real-time synthesis:** `cpal` for output, `fundsp`/custom DSP for the tuned synth voices and the master bus. This powers the "everything is an instrument" sonification and the programmatic chiptune engine (see `MUSIC.md`).
- **The Studio (Strudel-style creative canvas):** a small **pattern DSL** embedded in the host (evaluated live, hot, no recompile), inspired by TidalCycles/Strudel, where a user live-codes audiovisual math so that patterns drive both sound and geometry at once. Shader-heads can also drop into raw **WGSL** for visuals. This is the creator-tier surface that turns Numinous from a toy into an instrument-you-program. (See `DESIGN.md` "Modes" and "The Studio".)
- **Streaming/AI music (radio stations):** the **ElevenLabs** integration for the GTA-style stations lives behind a thin service layer in the Rust host (fetch/stream/cache tracks), fully decoupled from the synthesis engine. (See `MUSIC.md`.)

## The Room contract (the core abstraction)

Everything playable is a **Room**, a self-contained module implementing one interface. The engine knows nothing about math; rooms know nothing about packaging. This seam is also the future public SDK.

```rust
trait Room {
    fn meta(&self) -> RoomMeta;            // id, title, wing, wow, accent color, live preview

    // Lifecycle. The engine owns the loop and the clock; a room never spins its own.
    fn init(&mut self, ctx: &mut RoomContext);   // allocate GPU buffers, audio voices
    fn update(&mut self, dt: f32, t: f64);       // advance state (fixed-timestep aware, seeded RNG)
    fn render(&mut self, gfx: &mut Gfx);         // record draw/compute commands for this frame
    fn audio(&mut self, bus: &mut AudioBus);     // schedule/update sound
    fn dispose(&mut self);

    fn params(&self) -> &[ParamSpec];      // dials/toggles -> auto UI + audio mapping + deep-link state
    fn challenge(&self) -> Option<Challenge>;    // optional "Aha"
    fn reveal(&self) -> RevealCard;              // the mandatory revelation
}
```

`RoomContext` hands the room the shared services so it never reinvents them:

- **`Gfx`** wraps wgpu: render + compute passes, the camera, the active **Theme/Visual Era**, glow/bloom, and draw helpers. Rooms draw in theme-relative terms so an 8-bit/CRT/oscilloscope/modern skin restyles them for free.
- **`AudioBus`** wraps the synth: the shared voice, the scale/quantizer, reverb, master mix. Rooms request notes/drones; the bus keeps everything musical and coherent.
- **`Params`** are declared once and generate the auto-UI, the audio bindings, **and** the URL/state serialization for deep-links, from one source of truth. Every configuration is shareable by construction.
- **`Share`**: `capture_still()`, `capture_loop()`, `share_link()`.
- **RNG is seeded** (never ambient) so "random" rooms (Chaos Game, Galton) reproduce exactly from a deep-link.

### Why this shape
- Rooms are cheap and isolated: a new phenomenon is one module, no engine changes.
- The engine owns the loop, the clock, and the mix, so every room is automatically smooth, in-sync, and on-brand.
- `params` as single source of truth gives auto-UI, deep-links, and audio mappings with no per-room boilerplate.
- This trait *is* the low-level SDK surface. Phase 4 publishes it plus a template. Nothing else to expose.

### Two authoring paths (rooms are Studio programs)

There are two ways to author a room, and they target the same engine primitives (see `STUDIO.md`):

- **The Studio path (high-level, sandboxed):** a room written as Studio code/patterns, evaluated by the `studio` crate. Fast to write, safe to run untrusted, this is the path for **community rooms** and for rapidly prototyping first-party ones. The Phase 4 mod SDK is, in practice, "the Studio, shared."
- **The Rust `Room` trait (low-level, native):** hand-written Rust + custom WGSL for the heaviest spectacle rooms, where we want maximum control.

We **dogfood**: most rooms start as Studio programs and only drop to the native trait when they need to. This keeps the Studio continuously exercised by our own room-building, and means the runtime that powers the Studio (expression/pattern evaluation, the one-expression-to-sight-and-sound binding) is **engine-foundational and built early** (Phase 0 to 1), not a late add. Untrusted Studio code must be sandboxed (no filesystem/network, resource limits, GPU work only through the safe pipeline), a hard requirement tracked in `QUALITY.md`.

### Extensibility, and the safety of untrusted extensions

The design goal: **anyone can add a room, level, or phenomenon**, and doing so can never harm the person running it. The `Room` trait plus the `Surface` abstraction (already built) are the stable extension seam; adding a "level" means producing a `Room` and registering it. Extensions come in three tiers of increasing power and matching protection:

- **Tier 0, first-party (trusted):** native Rust rooms, compiled in and code-reviewed. Full power; the trust comes from review.
- **Tier 1, the Studio DSL (safe by construction):** a room expressed as declarative patterns / expressions, not arbitrary code. It cannot perform IO or reach the system at all; the host interprets it. Most "I have an idea for a level" contributions live here, and they are safe with no sandbox heroics because there is nothing dangerous to express. This is the primary path for community content.
- **Tier 2, compiled plugins (untrusted, sandboxed):** for contributors who need real code beyond the DSL, extensions ship as **WebAssembly** modules run in a capability sandbox (a WASM runtime such as Wasmtime). The layers of protection: no ambient authority (no filesystem, network, clock, or syscalls), only the explicit host API we hand in (the `Surface` drawing calls, seeded RNG, parameters); **memory and execution limits** (fuel metering, so a bad module cannot hang or exhaust memory); determinism (no wall-clock or true randomness); and GPU access only through the safe pipeline, never raw. A malicious or buggy module can waste its own bounded budget and produce an ugly frame, and nothing worse.

Curation (a beauty and correctness review before content is *featured*) is a quality gate, not the safety mechanism: safety comes from the sandbox, so even unreviewed Tier-2 modules are harmless to run. This is what makes "let anyone extend it" compatible with "it just works and is safe" (see `STUDIO.md`, `ROADMAP.md` Phase 4, and the sandbox requirements in `QUALITY.md`).

## Module architecture (Rust workspace)

```
numinous/
├── crates/
│   ├── engine/          # HEADLESS core: loop, clock, render+compute graph, Room trait + registry
│   │                    #   runs with NO window (offscreen render + audio-to-buffer) for CLI/MCP/CI
│   ├── gfx/             # wgpu wrappers, palette, glow/bloom, draw helpers (offscreen-capable)
│   ├── theme/           # Visual Eras (skins): teletype, 8-bit/CRT, oscilloscope, blueprint, modern
│   ├── audio/           # cpal/fundsp bus, synth voices, scales/quantizer, master mix
│   ├── music/           # programmatic station engine + ElevenLabs service layer (see MUSIC.md)
│   ├── studio/          # the live-coding pattern DSL (Strudel-style) + WGSL exposure
│   ├── params/          # ParamSpec -> UI + URL serialization + audio bindings
│   ├── share/           # still + loop capture, deep-link encode/decode
│   └── lore/            # the ARG/easter-egg layer (see LORE.md): triggers, secrets, the codex
├── rooms/               # one crate (or module) per room, depends on engine/* only
│   ├── times-tables/
│   ├── chaos-game/
│   └── ...
├── faces/               # the three thin frontends over the headless core (see INTERFACES.md)
│   ├── app/             # the native GUI: window, cabinet, packaging (Mac/Linux/Windows), no web
│   ├── cli/             # the `numinous` command: render, eval, tui, benchmark, insights, test
│   └── mcp/             # the MCP server: agents learn and play (list/describe/play/eval/create)
└── docs/
```

**Dependency rule:** `rooms/*` depend on `engine/*` only, never on each other, never on a face. The three faces (`app`, `cli`, `mcp`) depend on the core but never on each other, they are interchangeable views of the same headless engine (see `INTERFACES.md`). Rooms stay hot-swappable and SDK-ready.

**Headless from day one.** The core must render offscreen and synthesize audio to a buffer with no window, because the CLI, the MCP server, and the entire `QUALITY.md` test apparatus all drive it that way. This is a Phase 0 constraint, not a later port.

## Key technical concerns

- **Frame pacing:** fixed-timestep `update` decoupled from interpolated `render`, so physics rooms (pendulums, Galton) stay deterministic (and shareable) while rendering stays smooth. The **audio clock is the master timeline** for anything that must stay tight to sound.
- **Audio/visual sync:** visuals read the audio clock; sound is scheduled ahead on the audio thread, never fired from the render loop. This is what makes "everything is an instrument" feel locked-in.
- **GPU-heavy rooms** (Mandelbrot deep-zoom, reaction-diffusion) live entirely in compute/fragment shaders; deep Mandelbrot needs perturbation/extended-precision, so treat it as a dedicated spike, with the optional CUDA fast path as the "go even deeper" upgrade.
- **Performance budget:** 60fps floor on mid-range integrated GPUs (including Apple Silicon and Intel iGPUs). Each room declares a cost tier so Ambient/Watch mode never stacks two expensive rooms.
- **Accessibility as infrastructure:** mute, reduce-motion, and colorblind-safe palettes live in `theme`/`audio`, so rooms inherit them.

## Build & distribution

- **Desktop (the product):** Rust/Bevy to signed `.app` (macOS, universal/Apple-Silicon), `.AppImage` + `.deb` (Linux), `.msi` + `.exe` (Windows). Native, offline, fast.
- **Sharing (native):** in-app export of loops/stills (video/image files) plus `.num` seed files and the `numinous://` URL scheme that reopen an exact configuration in the installed app. No web build.
- **CI:** every commit builds all three desktop targets and runs the beauty-QA screenshot pass.
- **Later (Phase 4):** Steam (Workshop hosts community rooms) + itch.io.

## Open technical questions (resolve during Phase 0 spike)

1. **Bevy vs. bespoke wgpu shell.** Bevy accelerates the shell but adds opinions; a hand-rolled wgpu app is leaner but more work. Decide with a one-week spike building the same trivial room both ways.
2. **Native share/deep-link plumbing:** registering the `numinous://` URL scheme per OS and the `.num` seed-file association, so a shared link reliably launches the installed app to the exact configuration.
3. **CUDA/Triton fast-path interop** from Rust (FFI, or a separate compute process) for the one or two extreme rooms, decided only when a room actually needs it.
4. **The Studio DSL:** build a bespoke pattern language, or embed an existing scripting lang (Rhai/Lua) as the host and layer patterns on top. Prototype both.
5. **Audio latency** of `cpal` per platform under load, verified early (part of Phase 0).
